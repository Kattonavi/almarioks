import { useCallback, useEffect, useReducer, useRef, useState } from "react";

import { assertNever } from "../assertNever";
import type { Unlisten, VaultClient } from "../ipc/client";
import { errorMessage } from "../ipc/errors";
import type { VaultStatus } from "../ipc/schemas";
import { SettingsPanel } from "../settings/SettingsPanel";
import { SettingsIcon } from "../ui/icons";
import { CreateScreen } from "./CreateScreen";
import { Dialog } from "./Dialog";
import { OpenVault, type Guard } from "./OpenVault";
import { initialState, reducer, type AppState } from "./state";
import { UnlockScreen, type UnlockKey } from "./UnlockScreen";
import { useActivityPing } from "./useActivityPing";
import { WelcomeScreen } from "./WelcomeScreen";

interface OpenInfo {
  readonly weakKdf: boolean;
  readonly savePending: boolean;
  /** When the vault locks by itself (ms since the epoch), anchored to the moment the last status arrived (RF-29). */
  readonly lockDeadline: number | undefined;
  readonly generation: number;
}

function openInfoReducer(
  current: OpenInfo,
  action: {
    readonly status: Extract<VaultStatus, { state: "open" }>;
    readonly remount: boolean;
    /** `Date.now()` when the answer arrived: taken by the caller, so the reducer stays pure. */
    readonly receivedAt: number;
  },
): OpenInfo {
  const seconds = action.status.locksInSeconds;
  return {
    weakKdf: action.status.weakKdf,
    savePending: action.status.savePending,
    lockDeadline: seconds === undefined ? undefined : action.receivedAt + seconds * 1000,
    generation: current.generation + (action.remount ? 1 : 0),
  };
}

/** How often the window asks again while the core reports a derivation it did not start. */
const DERIVING_POLL_MS = 250;

/**
 * RF-29: how often the open vault asks the core how long is left before it locks. `vault_status` is cheap and is not
 * activity: asking does not push the lock back.
 */
export const STATUS_POLL_MS = 5000;

/**
 * Owns the app state machine. Fed by `vault_status` and by the core's `vault-locked` event. Every async answer is
 * tagged with the lock epoch it started in: an answer that arrives after a lock is dropped, so a late `vault_open`
 * can never mount the vault again (RF-32).
 */
export function VaultRoot({ client }: { readonly client: VaultClient }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [attempt, setAttempt] = useState(0);
  const epoch = useRef(0);
  /** Flags of the open vault that only its screen needs (RF-17, RF-23); `generation` remounts it after a reload. */
  const [openInfo, setOpenInfo] = useReducer(openInfoReducer, {
    weakKdf: false,
    savePending: false,
    lockDeadline: undefined,
    generation: 0,
  });
  /** RF-17: the weak-protection notice closed "for this session" (until the app closes), per vault file. */
  const [weakDismissed, setWeakDismissed] = useState<ReadonlySet<string>>(() => new Set());

  const applyStatus = useCallback((status: VaultStatus, remount = false) => {
    dispatch({ type: "status", status });
    if (status.state === "open") setOpenInfo({ status, remount, receivedAt: Date.now() });
  }, []);

  const refresh = useCallback(async () => {
    const started = epoch.current;
    const result = await client.vaultStatus();
    if (started !== epoch.current) return;
    if (result.ok) applyStatus(result.value);
    else dispatch({ type: "status-failed", error: result.error });
  }, [client, applyStatus]);

  /** Tags an action of the open vault with the current lock epoch: a late answer after a lock is dropped (RF-32). */
  const begin = useCallback((): Guard => {
    const started = epoch.current;
    const stale = () => started !== epoch.current;
    return {
      stale,
      apply: async (op, remount) => {
        const result = await op();
        if (stale()) return { kind: "stale" };
        if (!result.ok) return { kind: "failed", error: result.error };
        applyStatus(result.value, remount);
        return { kind: "applied", status: result.value };
      },
    };
  }, [applyStatus]);

  /** P8: one dialog, reachable from every screen (it shows nothing of the vault). */
  const [settingsOpen, setSettingsOpen] = useState(false);

  const dropVault = useCallback(() => {
    epoch.current += 1;
    // RF-32: every open dialog goes with the rest.
    setSettingsOpen(false);
    dispatch({ type: "vault-locked" });
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: Unlisten | undefined;
    client
      .onVaultLocked(() => {
        dropVault();
        void refresh();
      })
      .then(
        (stop) => {
          if (disposed) stop();
          else unlisten = stop;
        },
        () => {
          // Without the lock event the window could keep showing a locked vault: fail closed.
          if (disposed) return;
          epoch.current += 1;
          dispatch({ type: "status-failed", error: { code: "Inesperado" } });
        },
      );
    void refresh();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [client, attempt, refresh, dropVault]);

  // The core reported a derivation already running (e.g. after the webview reloaded): ask until it ends.
  useEffect(() => {
    if (state.kind !== "deriving" || state.origin !== "status") return;
    const timer = setTimeout(() => void refresh(), DERIVING_POLL_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [state, refresh]);

  /**
   * RF-29: the countdown follows the core. A failed or late answer changes nothing: a passing hiccup must not take
   * the vault (and a half-filled form) off the screen, and an answer from before a lock is dropped (RF-32).
   */
  const pollStatus = useCallback(async () => {
    const started = epoch.current;
    const result = await client.vaultStatus();
    if (started !== epoch.current || !result.ok) return;
    if (result.value.state === "open") setOpenInfo({ status: result.value, remount: false, receivedAt: Date.now() });
    else applyStatus(result.value);
  }, [client, applyStatus]);

  const isOpen = state.kind === "open";
  useEffect(() => {
    if (!isOpen) return;
    const timer = setInterval(() => void pollStatus(), STATUS_POLL_MS);
    return () => {
      clearInterval(timer);
    };
  }, [isOpen, pollStatus]);

  const ping = useCallback(() => {
    void client.activity();
  }, [client]);
  useActivityPing(ping, state.kind === "open");

  async function unlock(path: string, key: UnlockKey) {
    if (state.kind === "deriving") return;
    const started = epoch.current;
    dispatch({ type: "unlock-started" });
    const result = await client.vaultOpen({ path, ...key });
    if (started !== epoch.current) return;
    if (result.ok) applyStatus(result.value);
    else dispatch({ type: "unlock-failed", error: result.error });
  }

  async function lock() {
    dropVault();
    await client.vaultLock();
    await refresh();
  }

  async function close() {
    dropVault();
    await client.vaultClose();
    await refresh();
  }

  // A render function, not a nested component: a nested component would be a new type on every render and
  // remount the screen (and its form) each time.
  const settingsButton = (
    <button
      type="button"
      className="btn-tool"
      title="Ajustes y Acerca de"
      onClick={() => {
        setSettingsOpen(true);
      }}
    >
      <SettingsIcon />
      <span className="label">Ajustes y Acerca de</span>
    </button>
  );

  return (
    <>
      {/* With the vault open the button sits at the end of the vault bar, so the tab order follows what is seen. */}
      {state.kind !== "open" && <div className="actions app-tools">{settingsButton}</div>}
      {renderScreen(state)}
      {settingsOpen && (
        <Dialog
          title="Ajustes y Acerca de"
          onClose={() => {
            setSettingsOpen(false);
          }}
        >
          <SettingsPanel
            client={client}
            vaultPath={state.kind === "open" ? state.vault.path : undefined}
            onSaved={() => {
              // A new lock time starts counting now: show it without waiting for the next poll.
              if (isOpen) void pollStatus();
            }}
          />
          <div className="actions">
            <button
              type="button"
              onClick={() => {
                setSettingsOpen(false);
              }}
            >
              Cerrar
            </button>
          </div>
        </Dialog>
      )}
    </>
  );

  function renderScreen(current: AppState) {
    switch (current.kind) {
      case "loading":
        return (
          <p role="status" className="gate pane-state">
            Consultando la bóveda…
          </p>
        );
      case "unavailable":
        return (
          <section aria-labelledby="unavailable-title" className="gate">
            <h2 id="unavailable-title" className="gate-title">
              No se pudo consultar la bóveda
            </h2>
            <p role="alert" className="error">
              {errorMessage(current.error)}
            </p>
            <button type="button" onClick={() => { setAttempt((value) => value + 1); }}>
              Reintentar
            </button>
          </section>
        );
      case "closed":
        switch (current.view.kind) {
          case "welcome":
            return (
              <WelcomeScreen
                client={client}
                onChoose={(path) => { dispatch({ type: "vault-chosen", path }); }}
                onCreate={() => { dispatch({ type: "create-requested" }); }}
              />
            );
          case "create":
            return (
              <CreateScreen
                client={client}
                onBack={() => { dispatch({ type: "back-to-welcome" }); }}
                onDone={() => void refresh()}
              />
            );
          case "unlock": {
            const { vault } = current.view;
            return (
              <UnlockScreen
                client={client}
                vault={vault}
                busy={false}
                error={current.view.error}
                recovery={null}
                onUnlock={(key) => void unlock(vault.path, key)}
                secondary={{
                  label: "Volver a la bienvenida",
                  onClick: () => { dispatch({ type: "back-to-welcome" }); },
                }}
              />
            );
          }
          default:
            return assertNever(current.view);
        }
      case "locked":
      case "deriving": {
        const { vault } = current;
        return (
          <UnlockScreen
            client={client}
            vault={vault}
            busy={current.kind === "deriving"}
            error={current.kind === "locked" ? current.error : null}
            recovery={current.recovery}
            onUnlock={(key) => void unlock(vault.path, key)}
            secondary={
              current.kind === "deriving" && current.back === "closed"
                ? { label: "Volver a la bienvenida", onClick: () => { dispatch({ type: "back-to-welcome" }); } }
                : { label: "Cerrar bóveda", onClick: () => void close() }
            }
          />
        );
      }
      case "open": {
        const { path } = current.vault;
        return (
          <OpenVault
            client={client}
            vault={current.vault}
            name={current.name}
            readOnly={current.readOnly}
            weakKdf={openInfo.weakKdf}
            savePending={openInfo.savePending}
            lockDeadline={openInfo.lockDeadline}
            recovery={current.recovery}
            generation={openInfo.generation}
            weakDismissed={weakDismissed.has(path)}
            onDismissWeak={() => {
              setWeakDismissed((dismissed) => new Set([...dismissed, path]));
            }}
            begin={begin}
            onLock={() => void lock()}
            tools={settingsButton}
          />
        );
      }
      default:
        return assertNever(current);
    }
  }
}
