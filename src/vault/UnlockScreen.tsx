import { useEffect, useRef, useState, type SubmitEvent } from "react";

import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { Recovery } from "../ipc/schemas";
import { FileIcon } from "../ui/icons";
import { initialOf } from "./lines";
import { RecoveryNotice } from "./RecoveryNotice";
import type { VaultRef } from "./state";

export interface UnlockKey {
  readonly password?: string;
  readonly keyFilePath?: string;
}

interface UnlockScreenProps {
  readonly client: Pick<VaultClient, "pickKeyFile">;
  readonly vault: VaultRef;
  /** RF-18: the key is being derived; the form is disabled and the progress is shown. */
  readonly busy: boolean;
  readonly error: AppError | null;
  readonly recovery: Recovery | null;
  readonly onUnlock: (key: UnlockKey) => void;
  /** "Volver a la bienvenida" for a file just chosen, "Cerrar bóveda" for a locked one. */
  readonly secondary: { readonly label: string; readonly onClick: () => void };
}

/**
 * P3: unlock with the master password, a key file or both. Also the lock screen.
 * The password is never React state: it is read from the field on submit, the field is emptied right away, and the
 * value travels in a single `invoke` (ADR-0002, AMENAZAS W2).
 */
export function UnlockScreen({ client, vault, busy, error, recovery, onUnlock, secondary }: UnlockScreenProps) {
  const passwordRef = useRef<HTMLInputElement>(null);
  const [keyFilePath, setKeyFilePath] = useState<string | null>(null);
  const [pickError, setPickError] = useState<AppError | null>(null);

  // Focus the password when the screen appears and again after a failed attempt.
  useEffect(() => {
    if (!busy) passwordRef.current?.focus();
  }, [busy, error]);

  function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const field = passwordRef.current;
    const password = field?.value ?? "";
    if (field !== null) field.value = "";
    onUnlock({
      ...(password === "" ? {} : { password }),
      ...(keyFilePath === null ? {} : { keyFilePath }),
    });
  }

  async function pickKeyFile() {
    setPickError(null);
    const result = await client.pickKeyFile();
    if (!result.ok) setPickError(result.error);
    else if (result.value !== null) setKeyFilePath(result.value);
  }

  const shownError = error ?? pickError;

  return (
    <section aria-labelledby="unlock-title" className="gate">
      {/* The vault is the station every line reaches: the badge the root group wears once it is open. */}
      <header className="gate-head">
        <span className="bullet" data-line="station" data-initial={initialOf(vault.fileName)} aria-hidden="true" />
        <h2 id="unlock-title" className="gate-title">
          Desbloquear {vault.fileName}
        </h2>
        <p className="path mono">{vault.path}</p>
      </header>
      {recovery !== null && <RecoveryNotice recovery={recovery} />}

      <form onSubmit={onSubmit} noValidate className="gate-form">
        <fieldset disabled={busy}>
          <legend>Llave de la bóveda</legend>
          <p id="unlock-help" className="gate-help">
            Usa la contraseña maestra, el archivo llave o los dos, según cómo creaste la bóveda.
          </p>

          <div className="field">
            <label htmlFor="unlock-password">Contraseña maestra</label>
            <input
              ref={passwordRef}
              id="unlock-password"
              type="password"
              autoComplete="off"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
              aria-describedby={shownError === null ? "unlock-help" : "unlock-help unlock-error"}
              aria-invalid={error?.code === "LlaveIncorrecta"}
            />
          </div>

          <div className="field" role="group" aria-labelledby="key-file-label">
            <span id="key-file-label" className="field-label">
              Archivo llave
            </span>
            <p className={keyFilePath === null ? "path" : "path mono"}>{keyFilePath ?? "Ninguno"}</p>
            <div className="actions">
              <button type="button" onClick={() => void pickKeyFile()}>
                <FileIcon />
                Elegir archivo llave…
              </button>
              {keyFilePath !== null && (
                <button type="button" onClick={() => { setKeyFilePath(null); }}>
                  Quitar archivo llave
                </button>
              )}
            </div>
          </div>

          {/* The key failed as a whole (it never says which part), so the error sits under both of its parts. */}
          <p id="unlock-error" role="alert" className="error">
            {shownError === null ? "" : errorMessage(shownError)}
          </p>

          <div className="actions gate-actions">
            <button type="submit" className="btn-primary">
              Desbloquear
            </button>
            <button type="button" onClick={secondary.onClick}>
              {secondary.label}
            </button>
          </div>
        </fieldset>
      </form>

      <div role="status" aria-live="polite" className="deriving">
        {busy && (
          <>
            <p>Abriendo la bóveda: derivando la llave. Puede tardar unos segundos.</p>
            <div className="progress-track">
              <progress aria-label="Abriendo la bóveda" />
            </div>
          </>
        )}
      </div>
    </section>
  );
}
