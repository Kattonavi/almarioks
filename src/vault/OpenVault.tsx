import { useEffect, useState } from "react";

import { assertNever } from "../assertNever";
import type { Unlisten, VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { Recovery, SaveState, VaultStatus } from "../ipc/schemas";
import { Dialog } from "./Dialog";
import { RecoveryNotice } from "./RecoveryNotice";
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

function politeText(view: SaveView, notice: string): string {
  switch (view.kind) {
    case "saving":
      return "Guardando…";
    case "saved":
      return "Guardado.";
    case "reloaded":
      return "Se recargó el archivo del disco y se descartó lo que no estaba guardado.";
    case "idle":
    case "failed":
    case "external":
      return notice;
    default:
      return assertNever(view);
  }
}

interface OpenVaultProps {
  readonly client: VaultClient;
  readonly vault: VaultRef;
  readonly name: string;
  readonly readOnly: boolean;
  readonly weakKdf: boolean;
  /** `save_pending` of the last status: the same notice as a failed save (RF-23). */
  readonly savePending: boolean;
  readonly recovery: Recovery | null;
  /** Bumped after a reload so the vault screens drop what they had and ask the core again. */
  readonly generation: number;
  readonly weakDismissed: boolean;
  readonly onDismissWeak: () => void;
  readonly begin: () => Guard;
  readonly onLock: () => void;
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
  const [busy, setBusy] = useState(false);
  const [confirmReload, setConfirmReload] = useState(false);
  const [convertError, setConvertError] = useState<AppError | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: Unlisten | undefined;
    client
      .onSaveState((state) => {
        if (!disposed) setSave(viewOf(state));
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
        setNotice("");
        setSave(
          outcome.status.state === "open" && outcome.status.savePending ? { kind: "failed", error: null } : after,
        );
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
      setNotice(
        `Se creó ${fileNameOf(path)} en formato KDBX 4 y ya puedes editarla. El archivo original quedó igual.`,
      );
    }
  }

  const problem = save.kind === "failed" || save.kind === "external" ? save : null;

  return (
    <section aria-labelledby="vault-title">
      <h2 id="vault-title">{props.name === "" ? vault.fileName : props.name}</h2>
      <p className="path">{vault.path}</p>
      {props.recovery !== null && <RecoveryNotice recovery={props.recovery} />}

      <div aria-live="polite">{politeText(save, notice) !== "" && <p>{politeText(save, notice)}</p>}</div>

      {problem !== null && (
        <section aria-labelledby="save-problem-title" className="notice">
          <div role="alert">
            {problem.kind === "failed" ? (
              <>
                <h3 id="save-problem-title">No se pudo guardar: tus cambios siguen en memoria</h3>
                <p>
                  El archivo del disco quedó como estaba. Vuelve a intentarlo o guarda la bóveda en otro lugar. Si otra
                  app tiene abierto el archivo (por ejemplo, KeePass o un programa de sincronización), ciérrala y
                  reintenta.
                </p>
              </>
            ) : (
              <>
                <h3 id="save-problem-title">El archivo cambió en disco</h3>
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
          <h3 id="read-only-title">Bóveda en solo lectura</h3>
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
          <h3 id="weak-kdf-title">Protección débil contra fuerza bruta</h3>
          <p>
            Los ajustes de cifrado de esta bóveda son antiguos o usan poca memoria: alguien con una copia del archivo
            podría probar contraseñas mucho más rápido. Tus datos siguen cifrados.
          </p>
          <p>
            Para reforzarla, ábrela en KeePass 2 o KeePassXC y cambia sus ajustes de cifrado a Argon2id con 64 MiB o
            más. Esta versión de AlmarioKS todavía no lo hace.
          </p>
          <div className="actions">
            <button type="button" onClick={props.onDismissWeak}>
              Cerrar este aviso por esta sesión
            </button>
          </div>
        </section>
      )}

      <div className="actions">
        <button type="button" onClick={props.onLock}>
          Bloquear
        </button>
      </div>
      <VaultScreen key={`${vault.path}#${String(props.generation)}`} client={client} readOnly={readOnly} />

      {confirmReload && (
        <Dialog
          title="Recargar el archivo del disco"
          onClose={() => {
            setConfirmReload(false);
          }}
        >
          <p>
            Se descartan todos los cambios que no se pudieron guardar y se carga el archivo tal como lo dejó la otra
            app. Lo descartado no se puede recuperar.
          </p>
          <div className="actions">
            <button type="button" onClick={reload}>
              Recargar y descartar
            </button>
            <button
              type="button"
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

function withError(view: SaveView, error: AppError): SaveView {
  return view.kind === "external" ? { kind: "external", error } : { kind: "failed", error };
}
