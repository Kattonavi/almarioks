import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { useState } from "react";

import { classifyUpdateError, FAILURE_MESSAGES, type UpdateFailure } from "./updateError";

/** Update flow with consent (ADR-0003): nothing is checked or downloaded without a click. */
export type UpdateState =
  | { readonly kind: "idle" }
  | { readonly kind: "checking" }
  | { readonly kind: "upToDate" }
  | { readonly kind: "available"; readonly update: Update }
  | { readonly kind: "downloading"; readonly downloaded: number; readonly total: number | undefined }
  | { readonly kind: "verifying" }
  | { readonly kind: "preparing" }
  | { readonly kind: "installing" }
  | { readonly kind: "error"; readonly failure: PanelFailure };

/**
 * Besides the plugin's failures: `prepare` — the vault could not be closed before installing, so nothing was installed;
 * `restart` — the vault was already closed (the core is exiting) but the installer did not start.
 */
type PanelFailure = UpdateFailure | "prepare" | "restart";

const PANEL_MESSAGES: Readonly<Record<PanelFailure, string>> = {
  ...FAILURE_MESSAGES,
  prepare:
    "No se pudo cerrar la bóveda antes de actualizar; ciérrala con «Bloquear» y vuelve a intentarlo. No se instaló nada.",
  restart:
    "La bóveda ya se cerró, pero la actualización no se pudo instalar. Cierra AlmarioKS y vuelve a abrirlo para seguir usándolo e intentarlo de nuevo.",
};

function assertNever(value: never): never {
  throw new Error(`Unexpected update state: ${JSON.stringify(value)}`);
}

/** The plugin sends `contentLength: null` when the server gives no Content-Length: treat it as unknown. */
function knownLength(value: number | null | undefined): number | undefined {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : undefined;
}

function statusText(state: UpdateState): string {
  switch (state.kind) {
    case "idle":
      return "";
    case "checking":
      return "Buscando actualizaciones…";
    case "upToDate":
      return "Tienes la versión más reciente.";
    case "available":
      return `Hay una versión nueva: ${state.update.version}.`;
    case "downloading":
      return state.total === undefined
        ? "Descargando la actualización…"
        : `Descargando la actualización: ${String(Math.min(100, Math.floor((state.downloaded * 100) / state.total)))} %.`;
    case "verifying":
      return "Descarga completa. Verificando la firma…";
    case "preparing":
      return "Firma verificada. Cerrando la bóveda antes de instalar…";
    case "installing":
      // Only reachable after `download()` resolved, which the plugin does only for a valid signature.
      return "Firma verificada. Instalando: AlmarioKS se cerrará y volverá a abrirse sola en la versión nueva.";
    case "error":
      return PANEL_MESSAGES[state.failure];
    default:
      return assertNever(state);
  }
}

interface UpdatePanelProps {
  /**
   * RF-75 + RF-36 (C-1): runs after the signature was verified and BEFORE installing, and is awaited. It must close
   * the vault and clear our clipboard, because on Windows `install()` ends the process. If it rejects, nothing is
   * installed.
   */
  readonly beforeInstall: () => Promise<void>;
}

export function UpdatePanel({ beforeInstall }: UpdatePanelProps) {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });

  async function onCheck() {
    setState({ kind: "checking" });
    try {
      const update = await check();
      setState(update === null ? { kind: "upToDate" } : { kind: "available", update });
    } catch (error: unknown) {
      setState({ kind: "error", failure: classifyUpdateError(error) });
    }
  }

  async function onInstall(update: Update) {
    let downloaded = 0;
    let total: number | undefined;
    setState({ kind: "downloading", downloaded, total });
    const onEvent = (event: DownloadEvent) => {
      switch (event.event) {
        case "Started":
          total = knownLength(event.data.contentLength);
          setState({ kind: "downloading", downloaded, total });
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          setState({ kind: "downloading", downloaded, total });
          break;
        case "Finished":
          // The plugin reports the end of the download BEFORE it verifies the signature.
          setState({ kind: "verifying" });
          break;
        default:
          assertNever(event);
      }
    };
    try {
      // Resolves only if the package matches the signature and the pubkey bundled in the app.
      await update.download(onEvent);
    } catch (error: unknown) {
      setState({ kind: "error", failure: classifyUpdateError(error) });
      return;
    }
    setState({ kind: "preparing" });
    try {
      await beforeInstall();
    } catch {
      setState({ kind: "error", failure: "prepare" });
      return;
    }
    setState({ kind: "installing" });
    try {
      // On Windows the app exits here; the installer (passive mode) shows progress and reopens the app.
      await update.install();
    } catch {
      // The core is already exiting with the vault closed: only a restart brings the app back.
      setState({ kind: "error", failure: "restart" });
    }
  }

  const busy =
    state.kind === "checking" ||
    state.kind === "downloading" ||
    state.kind === "verifying" ||
    state.kind === "preparing" ||
    state.kind === "installing" ||
    // After preparing, the core is exiting: checking again cannot help until AlmarioKS is reopened.
    (state.kind === "error" && state.failure === "restart");
  const text = statusText(state);

  return (
    <section aria-labelledby="update-title">
      <h2 id="update-title">Actualizaciones</h2>
      <p>AlmarioKS solo busca actualizaciones cuando tú lo pides.</p>

      <p role="status" aria-live="polite">
        {text}
      </p>

      {state.kind === "downloading" && (
        <div className="progress-track">
          <progress
            aria-label="Progreso de la descarga"
            {...(state.total !== undefined && state.total > 0
              ? { value: state.downloaded, max: state.total }
              : {})}
          />
        </div>
      )}

      {state.kind === "available" ? (
        <>
          {state.update.body !== undefined && state.update.body.trim() !== "" && (
            <>
              <h3>Novedades de la versión {state.update.version}</h3>
              <p className="release-notes">{state.update.body}</p>
            </>
          )}
          <div className="actions">
            <button type="button" className="btn-primary" onClick={() => void onInstall(state.update)}>
              Actualizar
            </button>
            <button type="button" onClick={() => { setState({ kind: "idle" }); }}>
              Ahora no
            </button>
          </div>
        </>
      ) : (
        <button type="button" disabled={busy} onClick={() => void onCheck()}>
          Buscar actualizaciones
        </button>
      )}
    </section>
  );
}
