/**
 * Classifies an updater failure without showing the raw technical error to the user.
 *
 * The plugin rejects with the `Display` text of its Rust error (tauri-plugin-updater 2.14, minisign-verify 0.3):
 * - signature: "The signature verification failed", "The signature was created with a different key…",
 *   "The update was signed for version…", "The signature … could not be decoded…".
 * - network: reqwest errors ("error sending request for url…"), timeouts, DNS and connection failures.
 */
export type UpdateFailure = "offline" | "signature" | "unknown";

const SIGNATURE = /signature|signed for version/i;
const OFFLINE =
  /error sending request|connect|dns|timed out|timeout|network|offline|unreachable|resolve/i;

export function classifyUpdateError(error: unknown): UpdateFailure {
  const text = error instanceof Error ? error.message : String(error);
  if (SIGNATURE.test(text)) return "signature";
  if (OFFLINE.test(text)) return "offline";
  return "unknown";
}

export const FAILURE_MESSAGES: Readonly<Record<UpdateFailure, string>> = {
  offline:
    "No hay conexión con el servidor de actualizaciones. Revisa tu conexión a internet y vuelve a intentarlo.",
  signature:
    "La firma de la actualización no es válida: no se instaló. Tu versión actual sigue intacta. Descarga AlmarioKS solo desde su página oficial de GitHub.",
  unknown:
    "No se pudo completar la actualización. Vuelve a intentarlo más tarde; si se repite, descarga la última versión desde la página oficial de GitHub.",
};
