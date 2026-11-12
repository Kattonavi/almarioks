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
  | { readonly kind: "installing" }
  | { readonly kind: "error"; readonly failure: UpdateFailure };

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
    case "installing":
      // Only reachable after `download()` resolved, which the plugin does only for a valid signature.
      return "Firma verificada. Instalando: AlmarioKS se cerrará y volverá a abrirse sola en la versión nueva.";
    case "error":
      return FAILURE_MESSAGES[state.failure];
    default:
      return assertNever(state);
  }
}

export function UpdatePanel() {
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
    setState({ kind: "installing" });
    try {
      // On Windows the app exits here; the installer (passive mode) shows progress and reopens the app.
      await update.install();
    } catch (error: unknown) {
      setState({ kind: "error", failure: classifyUpdateError(error) });
    }
  }

  const busy =
    state.kind === "checking" ||
    state.kind === "downloading" ||
    state.kind === "verifying" ||
    state.kind === "installing";
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
