import { useEffect, useRef, useState, type ReactNode } from "react";

import { assertNever } from "../assertNever";
import type { Unlisten, VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { Recovery, SaveState, VaultStatus } from "../ipc/schemas";
import { InfoIcon, LockIcon, WarningIcon } from "../ui/icons";
import { Dialog } from "./Dialog";
import { RecoveryNotice } from "./RecoveryNotice";
import type { SaveSignal } from "./saveSignal";
import { formatCountdown } from "./format";
import { fileNameOf, type VaultRef } from "./state";
import { VaultScreen } from "./VaultScreen";

/** What a status-returning command ended in, as seen by the open vault. */
export type Outcome =
  | { readonly kind: "applied"; readonly status: VaultStatus }
  | { readonly kind: "failed"; readonly error: AppError }
  /** A lock happened since the action started: the answer was dropped (RF-32). */
  | { readonly kind: "stale" };

/** Started when the person clicks, so a lock during a native file dialog also drops the answer. */
export interface Guard {
  readonly stale: () => boolean;
  /** `remount`: the content may have changed on disk (reload, convert), so the vault screens ask again. */
  readonly apply: (op: () => Promise<Result<VaultStatus>>, remount: boolean) => Promise<Outcome>;
}

/** The save notice (RF-23, RF-24, RF-89). Nothing in it is a vault datum. */
type SaveView =
  | { readonly kind: "idle" }
  | { readonly kind: "saving" }
  | { readonly kind: "saved" }
  | { readonly kind: "reloaded" }
  | { readonly kind: "failed"; readonly error: AppError | null }
  | { readonly kind: "external"; readonly error: AppError | null };

function viewOf(state: SaveState): SaveView {
  switch (state) {
    case "saving":
      return { kind: "saving" };
    case "saved":
      return { kind: "saved" };
    case "failed":
      return { kind: "failed", error: null };
    case "external-change":
      return { kind: "external", error: null };
    default:
      return assertNever(state);
  }
}

/** How often the save state at the foot of the rail looks at the clock again. */
const SAVED_TICK_MS = 20_000;

/**
 * What the foot of the rail says. At rest, with nothing pending, the vault is as the file has it: "Guardado". A
 * read-only vault never saves, so it says nothing of its own.
 */
function politeText(view: SaveView, notice: string, readOnly: boolean): string {
  switch (view.kind) {
    case "saving":
      return "Guardando…";
    case "saved":
      return "Guardado";
    case "reloaded":
      return "Se recargó el archivo del disco y se descartó lo que no estaba guardado.";
    case "idle":
      return notice !== "" || readOnly ? notice : "Guardado";
    case "failed":
    case "external":
      return notice;
    default:
      return assertNever(view);
  }
}

/** How long ago the last save reached the disk, the way the rail says it. */
export function savedAgo(savedAt: number, now: number): string {
  const minutes = Math.floor(Math.max(0, now - savedAt) / 60_000);
  if (minutes < 1) return "hace un momento";
  return minutes < 60 ? `hace ${String(minutes)} min` : `hace ${String(Math.floor(minutes / 60))} h`;
}

interface OpenVaultProps {
  readonly client: VaultClient;
  readonly vault: VaultRef;
  readonly name: string;
  readonly readOnly: boolean;
  readonly weakKdf: boolean;
  /** `save_pending` of the last status: the same notice as a failed save (RF-23). */
  readonly savePending: boolean;
  /**
   * When the vault locks by itself (ms since the epoch), from the `locks_in_seconds` of the last status: counted down
   * here between polls, never guessed. Absent when the status does not bring it: then nothing is shown.
   */
  readonly lockDeadline?: number | undefined;
  readonly recovery: Recovery | null;
  /** Bumped after a reload so the vault screens drop what they had and ask the core again. */
  readonly generation: number;
  readonly weakDismissed: boolean;
  readonly onDismissWeak: () => void;
  readonly begin: () => Guard;
  readonly onLock: () => void;
  /** Controls that are not the vault's own (settings): they sit at the end of the top bar, after "Bloquear". */
  readonly tools?: ReactNode;
}

/**
 * P4 with its states: saving, saved, failed, changed on disk (RF-23, RF-24), read-only with conversion (RF-14, RF-15)
 * and weak protection (RF-17). Mounted only while the vault is open: a lock unmounts every notice and dialog here.
 */
export function OpenVault(props: OpenVaultProps) {
  const { client, vault, readOnly, weakKdf, begin } = props;
  const [save, setSave] = useState<SaveView>(() =>
    props.savePending ? { kind: "failed", error: null } : { kind: "idle" },
  );
  const [notice, setNotice] = useState("");
  /** RF-21: how many saves reached the disk since this screen opened (see `SaveSignal`). */
  const [savedCount, setSavedCount] = useState(0);
  /** The moment the last save reached the disk, as this window learned of it; `null` until one does. */
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [busy, setBusy] = useState(false);
  const [confirmReload, setConfirmReload] = useState(false);
  const [convertError, setConvertError] = useState<AppError | null>(null);
  /** The notice whose button had focus goes away: focus moves to the vault title instead of being lost. */
  const titleRef = useRef<HTMLHeadingElement>(null);
  const focusTitle = useRef(false);

  useEffect(() => {
    if (!focusTitle.current) return;
    focusTitle.current = false;
    titleRef.current?.focus();
  });

  useEffect(() => {
    if (savedAt === null) return;
    const timer = setInterval(() => {
      setNow(Date.now());
    }, SAVED_TICK_MS);
    return () => {
      clearInterval(timer);
    };
  }, [savedAt]);

  useEffect(() => {
    let disposed = false;
    let unlisten: Unlisten | undefined;
    client
      .onSaveState((state) => {
        if (disposed) return;
        setSave(viewOf(state));
        if (state === "saved") {
          setSavedCount((count) => count + 1);
          setSavedAt(Date.now());
          setNow(Date.now());
        }
      })
      .then(
        (stop) => {
          if (disposed) stop();
          else unlisten = stop;
        },
        () => {
          // Without the event the notice cannot appear live; `save_pending` still brings it back on the next status.
        },
      );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [client]);

  /** Retry, save as or reload: the answer is a status. */
  async function settle(guard: Guard, op: () => Promise<Result<VaultStatus>>, after: SaveView) {
    const outcome = await guard.apply(op, after.kind === "reloaded");
    setBusy(false);
    switch (outcome.kind) {
      case "stale":
        return;
      case "failed": {
        const { error } = outcome;
        setSave((current) =>
          error.code === "CambioExterno" || (current.kind === "external" && error.code !== "GuardadoFallido")
            ? { kind: "external", error }
            : { kind: "failed", error },
        );
        return;
      }
      case "applied":
        focusTitle.current = true;
        setNotice("");
        if (outcome.status.state === "open" && outcome.status.savePending) {
          setSave({ kind: "failed", error: null });
          return;
        }
        setSave(after);
        // A retry or a save elsewhere that answered without anything pending wrote the file.
        if (after.kind === "saved") {
          setSavedCount((count) => count + 1);
          setSavedAt(Date.now());
          setNow(Date.now());
        }
        return;
      default:
        assertNever(outcome);
    }
  }

  function retry() {
    if (busy) return;
    setBusy(true);
    void settle(begin(), () => client.vaultRetrySave(), { kind: "saved" });
  }

  async function saveElsewhere() {
    if (busy) return;
    const guard = begin();
    setBusy(true);
    const picked = await client.pickVaultToCreate();
    if (guard.stale()) return;
    if (!picked.ok || picked.value === null) {
      setBusy(false);
      if (!picked.ok) setSave((current) => withError(current, picked.error));
      return;
    }
    const path = picked.value;
    await settle(guard, () => client.vaultSaveAs(path), { kind: "saved" });
  }

  function reload() {
    if (busy) return;
    setBusy(true);
    setConfirmReload(false);
    void settle(begin(), () => client.vaultReload(), { kind: "reloaded" });
  }

  async function convert() {
    if (busy) return;
    const guard = begin();
    setBusy(true);
    setConvertError(null);
    const picked = await client.pickVaultToCreate();
    if (guard.stale()) return;
    if (!picked.ok || picked.value === null) {
      setBusy(false);
      if (!picked.ok) setConvertError(picked.error);
      return;
    }
    const path = picked.value;
    const outcome = await guard.apply(() => client.vaultConvert(path), true);
    setBusy(false);
    if (outcome.kind === "failed") setConvertError(outcome.error);
    if (outcome.kind === "applied") {
      focusTitle.current = true;
      setNotice(
        `Se creó ${fileNameOf(path)} en formato KDBX 4 y ya puedes editarla. El archivo original quedó igual.`,
      );
    }
  }

  const problem = save.kind === "failed" || save.kind === "external" ? save : null;
  const saveText = politeText(save, notice, readOnly);
  const ago = save.kind === "saved" && savedAt !== null ? savedAgo(savedAt, now) : "";
  const saveSignal: SaveSignal = { saved: savedCount, problem: problem !== null };

  /** What the person has to know before working: above the work area, never over it. */
  const notices = (
    <>
      {props.recovery !== null && <RecoveryNotice recovery={props.recovery} />}

      {problem !== null && (
        <section aria-labelledby="save-problem-title" className="notice notice-danger">
          <div role="alert">
            {problem.kind === "failed" ? (
              <>
                <h3 id="save-problem-title">
                  <WarningIcon />
                  No se pudo guardar: tus cambios siguen en memoria
                </h3>
                <p>
                  El archivo del disco quedó como estaba. Vuelve a intentarlo o guarda la bóveda en otro lugar. Si otra
                  app tiene abierto el archivo (por ejemplo, KeePass o un programa de sincronización), ciérrala y
                  reintenta.
                </p>
              </>
            ) : (
              <>
                <h3 id="save-problem-title">
                  <WarningIcon />
                  El archivo cambió en disco
                </h3>
                <p>
                  Otra app modificó el archivo de la bóveda y AlmarioKS no lo sobrescribió. Tus cambios siguen en
                  memoria: guárdalos en otro lugar o recarga el archivo descartando lo pendiente.
                </p>
              </>
            )}
            {problem.error !== null && <p className="error">{errorMessage(problem.error)}</p>}
          </div>
          <div className="actions">
            {problem.kind === "failed" && (
              <button type="button" disabled={busy} onClick={retry}>
                Reintentar el guardado
              </button>
            )}
            <button type="button" disabled={busy} onClick={() => void saveElsewhere()}>
              Guardar en otro lugar…
            </button>
            {problem.kind === "external" && (
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setConfirmReload(true);
                }}
              >
                Recargar descartando lo pendiente…
              </button>
            )}
          </div>
        </section>
      )}

      {readOnly && (
        <section aria-labelledby="read-only-title" className="notice">
          <h3 id="read-only-title">
            <InfoIcon />
            Bóveda en solo lectura
          </h3>
          <p>
            Esta bóveda usa el formato antiguo KDBX 3.x, así que se abrió en solo lectura: puedes buscar, ver y copiar,
            pero no crear, editar, mover ni borrar entradas o grupos, ni vaciar la papelera.
          </p>
          <p>
            Para editarla, conviértela a KDBX 4: AlmarioKS crea un archivo nuevo donde elijas y el archivo original no
            se toca.
          </p>
          <div className="actions">
            <button type="button" disabled={busy} onClick={() => void convert()}>
              Convertir a KDBX 4…
            </button>
          </div>
          <p role="alert" className="error">
            {convertError === null ? "" : errorMessage(convertError)}
          </p>
        </section>
      )}

      {weakKdf && !props.weakDismissed && (
        <section aria-labelledby="weak-kdf-title" className="notice">
          <h3 id="weak-kdf-title">
            <WarningIcon />
            Protección débil contra fuerza bruta
          </h3>
          <p>
            Los ajustes de cifrado de esta bóveda son antiguos o usan poca memoria: alguien con una copia del archivo
            podría probar contraseñas mucho más rápido. Tus datos siguen cifrados.
          </p>
          <p>
            Para reforzarla, ábrela en KeePass 2 o KeePassXC y cambia sus ajustes de cifrado a Argon2id con 64 MiB o
            más. Esta versión de AlmarioKS todavía no lo hace.
          </p>
          <div className="actions">
            <button
              type="button"
              onClick={() => {
                focusTitle.current = true;
                props.onDismissWeak();
              }}
            >
              Cerrar este aviso por esta sesión
            </button>
          </div>
        </section>
      )}

    </>
  );

  return (
    <section aria-labelledby="vault-title" className="open-vault">
      <VaultScreen
        key={`${vault.path}#${String(props.generation)}`}
        client={client}
        readOnly={readOnly}
        saveSignal={saveSignal}
        barEnd={
          <>
            {/* Keyed by the deadline: a new status starts the count from its own moment. */}
            {props.lockDeadline !== undefined && (
              <LockCountdown key={props.lockDeadline} deadline={props.lockDeadline} />
            )}
            {/* A bar action without a plate: the weight of the bar is the search. */}
            <button type="button" className="btn-bar" onClick={props.onLock}>
              <LockIcon />
              Bloquear
            </button>
            {props.tools}
          </>
        }
        railHead={
          <>
            <p className="rail-label">Bóveda</p>
            {/* The rail shows no paths: the file of the vault is in this `title` and in Settings. */}
            <h2 id="vault-title" ref={titleRef} tabIndex={-1} title={vault.path}>
              {props.name === "" ? vault.fileName : props.name}
            </h2>
          </>
        }
        banner={notices}
      />

      {/* Outside the vault screen on purpose: this live region must not be remounted with it after a reload. */}
      <div aria-live="polite" className="save-state" data-state={save.kind}>
        {saveText !== "" && (
          <p>
            <span>
              {saveText}
              {/* How long ago is for the eye only: a live region that changed every minute would be read every minute. */}
              {ago !== "" && <span aria-hidden="true"> {ago}</span>}
            </span>
          </p>
        )}
      </div>

      {confirmReload && (
        <Dialog
          title="Recargar el archivo del disco"
          tone="danger"
          onClose={() => {
            setConfirmReload(false);
          }}
        >
          <p>
            Se descartan todos los cambios que no se pudieron guardar y se carga el archivo tal como lo dejó la otra
            app. Lo descartado no se puede recuperar.
          </p>
          <div className="actions">
            <button type="button" className="btn-danger" onClick={reload}>
              Recargar y descartar
            </button>
            <button
              type="button"
              data-initial-focus=""
              onClick={() => {
                setConfirmReload(false);
              }}
            >
              Cancelar
            </button>
          </div>
        </Dialog>
      )}
    </section>
  );
}

/** RF-29: a leaf of its own, so only this line repaints every second. It never shows less than zero. */
function LockCountdown({ deadline }: { readonly deadline: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  }, []);
  const seconds = Math.max(0, Math.ceil((deadline - now) / 1000));
  return (
    <p className="lock-countdown">
      <span className="label">Se bloquea en</span> <span className="mono num">{formatCountdown(seconds)}</span>
    </p>
  );
}

function withError(view: SaveView, error: AppError): SaveView {
  return view.kind === "external" ? { kind: "external", error } : { kind: "failed", error };
}
