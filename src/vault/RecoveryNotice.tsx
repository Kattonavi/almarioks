import { assertNever } from "../assertNever";
import type { Recovery } from "../ipc/schemas";

/** RF-34: after a lock with changes that could not be saved, say where they went, or that they were lost. */
export function RecoveryNotice({ recovery }: { readonly recovery: Recovery }) {
  switch (recovery.kind) {
    case "saved":
      return (
        <p className="notice" role="status">
          Al bloquearse había cambios sin guardar. Quedaron en un archivo de recuperación:{" "}
          <span className="path">{recovery.path}</span>. Ábrelo con la misma llave para recuperarlos.
        </p>
      );
    case "lost":
      return (
        <p className="notice" role="status">
          Al bloquearse había cambios sin guardar y no se pudieron conservar. Revisa tus últimos cambios antes de
          seguir.
        </p>
      );
    default:
      return assertNever(recovery);
  }
}
