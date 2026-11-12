import { useEffect, useRef, useState, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { KeyFileChoice, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import { fileNameOf } from "./state";

/** RF-7: wait this long after the last keystroke before asking the core for the strength. */
export const STRENGTH_DELAY_MS = 150;

type KeyMode = KeyFileChoice["kind"];

type FormError =
  | { readonly kind: "app"; readonly error: AppError }
  | { readonly kind: "mismatch" }
  | { readonly kind: "no-path" }
  | { readonly kind: "no-key-file" };

type Phase =
  | { readonly kind: "editing"; readonly error: FormError | null }
  | { readonly kind: "creating" }
  /** RF-9: created with a key file; the recovery notice is shown again before going to the vault. */
  | { readonly kind: "created"; readonly keyFilePath: string };

interface CreateScreenProps {
  readonly client: Pick<
    VaultClient,
    "pickVaultToCreate" | "pickKeyFile" | "pickNewKeyFile" | "passwordStrength" | "vaultCreate"
  >;
  readonly onBack: () => void;
  /** The core created and opened the vault: ask it for its status. */
  readonly onDone: () => void;
}

function formErrorText(error: FormError): string {
  switch (error.kind) {
    case "app":
      return errorMessage(error.error);
    case "mismatch":
      return "La contraseña maestra y su confirmación no coinciden. Escríbelas otra vez.";
    case "no-path":
      return "Elige dónde guardar la bóveda.";
    case "no-key-file":
      return "Elige el archivo llave o marca «Ninguno».";
    default:
      return assertNever(error);
  }
}

function strengthLabel(score: number): string {
  return ["muy débil", "débil", "regular", "buena", "muy buena"][score] ?? "";
}

/** Reads a secret field and empties it at once: the value lives only in the caller's stack (ADR-0002). */
function take(field: HTMLInputElement | null): string {
  const value = field?.value ?? "";
  if (field !== null) field.value = "";
  return value;
}

/**
 * P2: create a vault (RF-1, RF-3, RF-7, RF-8, RF-9). The master password and its confirmation are never React state:
 * they are read and emptied on submit, whatever happens next, and travel in a single `vault_create`.
 */
export function CreateScreen({ client, onBack, onDone }: CreateScreenProps) {
  const passwordRef = useRef<HTMLInputElement>(null);
  const confirmRef = useRef<HTMLInputElement>(null);
  const strengthSeq = useRef(0);
  const strengthTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(false);
  const [path, setPath] = useState<string | null>(null);
  const [keyMode, setKeyMode] = useState<KeyMode>("none");
  const [keyPaths, setKeyPaths] = useState<{ readonly existing: string | null; readonly generate: string | null }>({
    existing: null,
    generate: null,
  });
  const [score, setScore] = useState<number | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "editing", error: null });
  const [pickError, setPickError] = useState<AppError | null>(null);

  useEffect(() => {
    mounted.current = true;
    const timers = strengthTimer;
    const seq = strengthSeq;
    return () => {
      mounted.current = false;
      clearTimeout(timers.current);
      seq.current += 1;
    };
  }, []);

  // Back to the password after a failed attempt.
  useEffect(() => {
    if (phase.kind === "editing" && phase.error !== null) passwordRef.current?.focus();
  }, [phase]);

  function cancelStrength() {
    clearTimeout(strengthTimer.current);
    strengthSeq.current += 1;
  }

  function onPasswordInput() {
    cancelStrength();
    const mine = strengthSeq.current;
    if ((passwordRef.current?.value ?? "") === "") {
      setScore(null);
      return;
    }
    strengthTimer.current = setTimeout(() => {
      const value = passwordRef.current?.value ?? "";
      if (value === "") return;
      void client.passwordStrength(value).then((result) => {
        // A late answer for an older value is dropped.
        if (!mounted.current || mine !== strengthSeq.current) return;
        setScore(result.ok ? result.value.score : null);
      });
    }, STRENGTH_DELAY_MS);
  }

  async function pick(target: "vault" | KeyMode) {
    setPickError(null);
    const result =
      target === "vault"
        ? await client.pickVaultToCreate()
        : target === "existing"
          ? await client.pickKeyFile()
          : await client.pickNewKeyFile();
    if (!mounted.current) return;
    if (!result.ok) {
      setPickError(result.error);
      return;
    }
    const chosen = result.value;
    if (chosen === null) return;
    if (target === "vault") setPath(chosen);
    else if (target === "existing") setKeyPaths((paths) => ({ ...paths, existing: chosen }));
    else if (target === "generate") setKeyPaths((paths) => ({ ...paths, generate: chosen }));
  }

  function keyFileChoice(): KeyFileChoice | null {
    switch (keyMode) {
      case "none":
        return { kind: "none" };
      case "existing":
        return keyPaths.existing === null ? null : { kind: "existing", path: keyPaths.existing };
      case "generate":
        return keyPaths.generate === null ? null : { kind: "generate", path: keyPaths.generate };
      default:
        return assertNever(keyMode);
    }
  }

  function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (phase.kind !== "editing") return;
    // Both fields are emptied before any check, so no branch can leave a secret on screen.
    const password = take(passwordRef.current);
    const confirmation = take(confirmRef.current);
    cancelStrength();
    setScore(null);
    setPickError(null);

    const fail = (error: FormError) => {
      setPhase({ kind: "editing", error });
    };
    if (path === null) {
      fail({ kind: "no-path" });
      return;
    }
    if (password !== confirmation) {
      fail({ kind: "mismatch" });
      return;
    }
    const keyFile = keyFileChoice();
    if (keyFile === null) {
      fail({ kind: "no-key-file" });
      return;
    }
    // RF-3: an empty password is no password; it is never trimmed (RF-41).
    if (password === "" && keyFile.kind === "none") {
      fail({ kind: "app", error: { code: "LlaveVacia" } });
      return;
    }
    setPhase({ kind: "creating" });
    void create(path, password, keyFile);
  }

  async function create(vaultPath: string, password: string, keyFile: KeyFileChoice) {
    const result = await client.vaultCreate({
      path: vaultPath,
      name: fileNameOf(vaultPath).replace(/\.kdbx$/i, ""),
      ...(password === "" ? {} : { password }),
      keyFile,
    });
    if (!mounted.current) return;
    if (!result.ok) {
      setPhase({ kind: "editing", error: { kind: "app", error: result.error } });
      return;
    }
    if (keyFile.kind === "none") onDone();
    else setPhase({ kind: "created", keyFilePath: keyFile.path });
  }

  if (phase.kind === "created") {
    return (
      <section aria-labelledby="created-title">
        <h2 id="created-title">Bóveda creada</h2>
        <div className="notice" role="status">
          <p>
            Tu bóveda usa el archivo llave <span className="path">{phase.keyFilePath}</span>.
          </p>
          <p>
            Sin una copia de ese archivo y sin la contraseña no hay forma de recuperarla. Guarda ahora una copia del
            archivo llave en otro lugar seguro, por ejemplo una memoria USB.
          </p>
        </div>
        <div className="actions">
          <button type="button" onClick={onDone}>
            Ir a la bóveda
          </button>
        </div>
      </section>
    );
  }

  const creating = phase.kind === "creating";
  const formError = phase.kind === "editing" ? phase.error : null;
  const shownError = formError === null ? (pickError === null ? "" : errorMessage(pickError)) : formErrorText(formError);
  const keyPath = keyMode === "none" ? null : keyPaths[keyMode];

  return (
    <section aria-labelledby="create-title">
      <h2 id="create-title">Crear bóveda</h2>
      <p id="create-help">
        Elige dónde guardar el archivo y protégelo con una contraseña maestra, un archivo llave o los dos.
      </p>

      <form onSubmit={onSubmit} noValidate>
        <fieldset disabled={creating}>
          <legend>Bóveda nueva</legend>

          <div className="field" role="group" aria-labelledby="create-path-label">
            <span id="create-path-label">Archivo de la bóveda</span>
            <p className="path">{path ?? "Sin elegir"}</p>
            <div className="actions">
              <button type="button" onClick={() => void pick("vault")}>
                Elegir dónde guardarla…
              </button>
            </div>
          </div>

          <div className="field">
            <label htmlFor="create-password">Contraseña maestra</label>
            <input
              ref={passwordRef}
              id="create-password"
              type="password"
              autoComplete="new-password"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
              aria-describedby="create-strength"
              onInput={onPasswordInput}
            />
            <div id="create-strength" aria-live="polite">
              {score !== null && (
                <>
                  <p>
                    Fortaleza: {strengthLabel(score)} ({score} de 4){" "}
                    <meter min={0} max={4} low={2} high={3} optimum={4} value={score} aria-label="Fortaleza" />
                  </p>
                  {score < 3 && (
                    <p className="notice">
                      Es una contraseña débil. Puedes continuar, pero una más larga y variada protege mejor tu bóveda.
                    </p>
                  )}
                </>
              )}
            </div>
          </div>

          <div className="field">
            <label htmlFor="create-confirm">Repite la contraseña maestra</label>
            <input
              ref={confirmRef}
              id="create-confirm"
              type="password"
              autoComplete="new-password"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
              aria-invalid={formError?.kind === "mismatch"}
              aria-describedby="create-error"
            />
          </div>

          <fieldset>
            <legend>Archivo llave (opcional, recomendado)</legend>
            {(
              [
                ["none", "Ninguno"],
                ["existing", "Usar un archivo llave que ya tengo"],
                ["generate", "Generar un archivo llave nuevo"],
              ] as const
            ).map(([mode, label]) => (
              <div key={mode}>
                <input
                  type="radio"
                  id={`create-key-${mode}`}
                  name="create-key-mode"
                  checked={keyMode === mode}
                  onChange={() => {
                    setKeyMode(mode);
                  }}
                />{" "}
                <label htmlFor={`create-key-${mode}`}>{label}</label>
              </div>
            ))}
            {keyMode !== "none" && (
              <div className="field">
                <p className="path">{keyPath ?? "Sin elegir"}</p>
                <div className="actions">
                  <button type="button" onClick={() => void pick(keyMode)}>
                    {keyMode === "existing" ? "Elegir archivo llave…" : "Elegir dónde guardar el archivo llave…"}
                  </button>
                </div>
              </div>
            )}
          </fieldset>

          <div className="notice">
            <p>Nadie puede recuperar tu bóveda si olvidas la contraseña maestra: ni AlmarioKS ni nadie más.</p>
            {keyMode !== "none" && (
              <p>
                Sin una copia del archivo llave y sin la contraseña no hay forma de recuperarla. Guarda una copia del
                archivo llave en otro lugar.
              </p>
            )}
          </div>

          <div className="actions">
            <button type="submit">Crear bóveda</button>
            <button type="button" onClick={onBack}>
              Volver a la bienvenida
            </button>
          </div>
        </fieldset>
      </form>

      <div role="status" aria-live="polite">
        {creating && (
          <>
            <p>Creando la bóveda: derivando la llave. Puede tardar unos segundos.</p>
            <progress aria-label="Creando la bóveda" />
          </>
        )}
      </div>
      <p id="create-error" role="alert" className="error">
        {shownError}
      </p>
    </section>
  );
}
