import { useCallback, useEffect, useReducer, useRef, useState } from "react";

import { assertNever } from "../assertNever";
import type { Unlisten, VaultClient } from "../ipc/client";
import { errorMessage } from "../ipc/errors";
import { CreateScreen } from "./CreateScreen";
import { RecoveryNotice } from "./RecoveryNotice";
import { initialState, reducer, type AppState } from "./state";
import { UnlockScreen, type UnlockKey } from "./UnlockScreen";
import { useActivityPing } from "./useActivityPing";
import { WelcomeScreen } from "./WelcomeScreen";

/** How often the window asks again while the core reports a derivation it did not start. */
const DERIVING_POLL_MS = 250;

/**
 * Owns the app state machine. Fed by `vault_status` and by the core's `vault-locked` event. Every async answer is
 * tagged with the lock epoch it started in: an answer that arrives after a lock is dropped, so a late `vault_open`
 * can never mount the vault again (RF-32).
 */
export function VaultRoot({ client }: { readonly client: VaultClient }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [attempt, setAttempt] = useState(0);
  const epoch = useRef(0);

  const refresh = useCallback(async () => {
    const started = epoch.current;
    const result = await client.vaultStatus();
    if (started !== epoch.current) return;
    dispatch(
      result.ok ? { type: "status", status: result.value } : { type: "status-failed", error: result.error },
    );
  }, [client]);

  const dropVault = useCallback(() => {
    epoch.current += 1;
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
    dispatch(
      result.ok ? { type: "status", status: result.value } : { type: "unlock-failed", error: result.error },
    );
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
  return renderScreen(state);

  function renderScreen(current: AppState) {
    switch (current.kind) {
      case "loading":
        return <p role="status">Consultando la bóveda…</p>;
      case "unavailable":
        return (
          <section aria-labelledby="unavailable-title">
            <h2 id="unavailable-title">No se pudo consultar la bóveda</h2>
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
      case "open":
        return (
          <section aria-labelledby="vault-title">
            <h2 id="vault-title">{current.name === "" ? current.vault.fileName : current.name}</h2>
            <p className="path">{current.vault.path}</p>
            {current.readOnly && <p>Esta bóveda está abierta en solo lectura.</p>}
            {current.recovery !== null && <RecoveryNotice recovery={current.recovery} />}
            <p>Los grupos y las entradas llegan en la próxima versión de desarrollo.</p>
            <button type="button" onClick={() => void lock()}>
              Bloquear
            </button>
          </section>
        );
      default:
        return assertNever(current);
    }
  }
}
