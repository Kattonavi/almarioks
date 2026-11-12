import { useEffect, useRef, useState, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { KeyFileChoice, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { CreateLocation } from "../ipc/schemas";
import { FolderIcon, HideIcon, ShowIcon, WarningIcon } from "../ui/icons";
import { fileNameOf } from "./state";

/** RF-7: wait this long after the last keystroke before asking the core for the strength. */
export const STRENGTH_DELAY_MS = 150;
/** RF-97: wait this long after the last keystroke of the name before asking the core where the vault would go. */
export const LOCATION_DELAY_MS = 150;

type KeyMode = KeyFileChoice["kind"];

// RF-99, owner's decision of 2026-11-12 (second round, 1e): "Crear bóveda" opens with "Sin archivo llave" marked.
const DEFAULT_KEY_MODE: KeyMode = "none";

const KEY_OPTIONS: readonly { readonly mode: KeyMode; readonly label: string }[] = [
  { mode: "none", label: "Sin archivo llave" },
  { mode: "generate", label: "Crear uno nuevo" },
  { mode: "existing", label: "Usar uno mío" },
];

export const KEY_HELP_NONE = "La bóveda se abre solo con la contraseña maestra.";
export const KEY_HELP_EXISTING = "Eliges un archivo llave que ya tienes.";
/** RF-113: what "Crear uno nuevo" says while the core has not answered where the vault goes. */
export const KEY_HELP_BESIDE = "Se guarda junto a la bóveda. Guarda una copia en otro lugar.";

/**
 * The line under the key file options. RF-113: with "Crear uno nuevo" it says where the key file will be, with the
 * name and the path the core answers, written like the line of the vault (the folder by its name when it is the
 * proposed one, the whole path when another location was chosen).
 */
function KeyHelp({ mode, location }: { readonly mode: KeyMode; readonly location: CreateLocation | null }) {
  switch (mode) {
    case "none":
      return KEY_HELP_NONE;
    case "existing":
      return KEY_HELP_EXISTING;
    case "generate":
      if (location === null) return KEY_HELP_BESIDE;
      return (
        <>
          Se guarda junto a la bóveda: {location.inDefaultFolder && "Documentos › AlmarioKS › "}
          <span className="create-key-where">
            <span className="create-file mono" title={location.keyFilePath}>
              {location.inDefaultFolder ? location.keyFile : location.keyFilePath}
            </span>
            .
          </span>{" "}
          Guarda una copia en otro lugar.
        </>
      );
    default:
      return assertNever(mode);
  }
}

type FormError =
  | { readonly kind: "app"; readonly error: AppError }
  | { readonly kind: "no-name" }
  | { readonly kind: "no-path" }
  | { readonly kind: "exists" }
  | { readonly kind: "key-exists" }
  | { readonly kind: "mismatch" }
  | { readonly kind: "no-key-file" }
  | { readonly kind: "key-beside-needs-password" };

/** Where a message is shown: next to the field it is about; what belongs to no field goes above the actions. */
type Slot = "name" | "location" | "confirm" | "key" | "form";

/** What the last attempt said, and the field it is about: fixed when it fails, so a later choice does not move it. */
interface Failure {
  readonly slot: Slot;
  readonly text: string;
}

type Phase =
  | { readonly kind: "editing"; readonly error: Failure | null }
  | { readonly kind: "creating" }
  /** RF-9: created with a key file; the recovery notice is shown again before going to the vault. */
  | { readonly kind: "created"; readonly keyFilePath: string };

/** RF-97: where the vault would be created, as the core answers it. Nothing here is computed by the window. */
type Place =
  | { readonly kind: "asking" }
  | { readonly kind: "known"; readonly location: CreateLocation }
  /** The system gave no Documents folder: there is no proposal until a location is chosen with the dialog. */
  | { readonly kind: "no-documents" };

interface Message {
  readonly text: string;
  /** Said at once (after pressing "Crear bóveda"); what appears while typing is read with its field instead. */
  readonly alert: boolean;
}

interface CreateScreenProps {
  readonly client: Pick<
    VaultClient,
    "vaultCreateLocation" | "pickVaultToCreate" | "pickKeyFile" | "passwordStrength" | "vaultCreate"
  >;
  readonly onBack: () => void;
  /** The core created and opened the vault: ask it for its status. */
  readonly onDone: () => void;
}

/** RF-98: said before pressing "Crear bóveda" and again if the core still answers `ArchivoExiste`. */
export const EXISTS_VAULT = "Ya hay un archivo con ese nombre en esa carpeta. Cambia el nombre o la ubicación.";
/**
 * RF-4: the app never replaces a file, but when the location came from the Windows save dialog, that dialog has just
 * asked "replace it?". Only then does the message say so.
 */
export const EXISTS_CHOSEN =
  "Ya hay un archivo con ese nombre en esa carpeta. AlmarioKS nunca reemplaza un archivo, aunque Windows haya preguntado si querías reemplazarlo: no se tocó. Elige otra ubicación.";
/** RF-99: the key file to create goes beside the vault and never replaces one. */
export const EXISTS_KEY =
  "Ya hay un archivo llave con ese nombre junto a la bóveda y AlmarioKS no lo reemplaza. Cambia el nombre o la ubicación, o elige «Usar uno mío».";
export const PATH_HELP =
  "AlmarioKS no reemplaza archivos: si eliges uno que ya existe, aunque Windows pregunte si quieres reemplazarlo, te pedirá otro nombre.";
export const NO_NAME = "Ponle un nombre a la bóveda.";
/** A key file created beside the vault protects nothing by itself: whoever copies the folder takes both. */
export const KEY_BESIDE_NEEDS_PASSWORD =
  "El archivo llave nuevo queda junto a la bóveda: quien copie la carpeta se llevaría los dos. Escribe también una contraseña maestra, o elige «Usar uno mío» con un archivo llave guardado en otro lugar.";
export const NO_PATH = "Elige dónde guardar la bóveda.";
export const MISMATCH = "Las dos contraseñas no son iguales.";

function slotOf(error: FormError, chosen: boolean): Slot {
  switch (error.kind) {
    case "app":
      if (error.error.code === "ArchivoExiste") return chosen ? "location" : "name";
      return error.error.code === "NombreVacio" ? "name" : "form";
    case "no-name":
      return "name";
    case "exists":
      return chosen ? "location" : "name";
    case "no-path":
      return "location";
    case "mismatch":
      return "confirm";
    case "key-exists":
    case "no-key-file":
    case "key-beside-needs-password":
      return "key";
    default:
      return assertNever(error);
  }
}

function formErrorText(error: FormError, chosen: boolean): string {
  switch (error.kind) {
    case "app":
      if (error.error.code === "ArchivoExiste") return chosen ? EXISTS_CHOSEN : EXISTS_VAULT;
      // The shared message of this code talks about a group.
      return error.error.code === "NombreVacio" ? NO_NAME : errorMessage(error.error);
    case "no-name":
      return NO_NAME;
    case "exists":
      return chosen ? EXISTS_CHOSEN : EXISTS_VAULT;
    case "no-path":
      return NO_PATH;
    case "mismatch":
      return `${MISMATCH} Escríbelas otra vez.`;
    case "key-exists":
      return EXISTS_KEY;
    case "no-key-file":
      return "Elige tu archivo llave o marca «Sin archivo llave».";
    case "key-beside-needs-password":
      return KEY_BESIDE_NEEDS_PASSWORD;
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

function ErrorLine({ id, message }: { readonly id: string; readonly message: Message | null }) {
  if (message === null) return null;
  return (
    <p id={id} role={message.alert ? "alert" : undefined} className="create-error">
      {message.text}
    </p>
  );
}

/** The ids of the messages that exist, for `aria-describedby`. */
function describedBy(...ids: readonly (string | false)[]): string | undefined {
  const present = ids.filter((id) => id !== false);
  return present.length === 0 ? undefined : present.join(" ");
}

/**
 * P2: create a vault (RF-1, RF-3, RF-7 to RF-9, RF-96 to RF-99). The master password and its confirmation are never
 * React state: they are read and emptied on submit, whatever happens next, and travel in a single `vault_create`.
 * While they are typed the component keeps only what it shows about them: a score and whether they differ.
 */
export function CreateScreen({ client, onBack, onDone }: CreateScreenProps) {
  const nameRef = useRef<HTMLInputElement>(null);
  const locationButtonRef = useRef<HTMLButtonElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);
  const confirmRef = useRef<HTMLInputElement>(null);
  const strengthSeq = useRef(0);
  const strengthTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const firstLocation = useRef(true);
  const mounted = useRef(false);
  const [name, setName] = useState("");
  const [place, setPlace] = useState<Place>({ kind: "asking" });
  /** The path the save dialog answered: from then on the location stops following the name (RF-97). */
  const [chosen, setChosen] = useState<string | null>(null);
  const [keyMode, setKeyMode] = useState<KeyMode>(DEFAULT_KEY_MODE);
  const [ownKey, setOwnKey] = useState<string | null>(null);
  const [score, setScore] = useState<number | null>(null);
  const [differ, setDiffer] = useState(false);
  const [revealed, setRevealed] = useState(false);
  const [phase, setPhase] = useState<Phase>({ kind: "editing", error: null });
  const [pickError, setPickError] = useState<{ readonly at: "location" | "key"; readonly error: AppError } | null>(
    null,
  );

  useEffect(() => {
    mounted.current = true;
    const timers = strengthTimer;
    const seq = strengthSeq;
    // RF-96: the name is the first thing asked.
    nameRef.current?.focus();
    return () => {
      mounted.current = false;
      clearTimeout(timers.current);
      seq.current += 1;
    };
  }, []);

  // RF-97: the proposal follows the name, a moment after the last keystroke; an answer for an older name is dropped.
  const noDocuments = place.kind === "no-documents";
  useEffect(() => {
    if (chosen !== null || noDocuments) return;
    let stale = false;
    const delay = firstLocation.current ? 0 : LOCATION_DELAY_MS;
    firstLocation.current = false;
    const timer = setTimeout(() => {
      void client.vaultCreateLocation(name, null).then((result) => {
        if (stale) return;
        if (result.ok) setPlace({ kind: "known", location: result.value });
        else setPlace(result.error.code === "ErrorInterno" ? { kind: "no-documents" } : { kind: "asking" });
      });
    }, delay);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [client, name, chosen, noDocuments]);

  // Back to the field to fix after a failed attempt.
  useEffect(() => {
    if (phase.kind !== "editing" || phase.error === null) return;
    if (phase.error.slot === "name") nameRef.current?.focus();
    else if (phase.error.slot === "location") locationButtonRef.current?.focus();
    else passwordRef.current?.focus();
  }, [phase]);

  /** Read through a call, so the compiler does not carry the answer across an `await`. */
  function gone(): boolean {
    return !mounted.current;
  }

  function cancelStrength() {
    clearTimeout(strengthTimer.current);
    strengthSeq.current += 1;
  }

  /** A message about a field goes away when the person starts fixing that field. */
  function clearError(...slots: readonly Slot[]) {
    if (phase.kind === "editing" && phase.error !== null && slots.includes(phase.error.slot)) {
      setPhase({ kind: "editing", error: null });
    }
  }

  function fail(error: FormError) {
    const fromDialog = chosen !== null;
    setPhase({ kind: "editing", error: { slot: slotOf(error, fromDialog), text: formErrorText(error, fromDialog) } });
  }

  function onSecretInput() {
    const password = passwordRef.current?.value ?? "";
    const confirmation = confirmRef.current?.value ?? "";
    setDiffer(confirmation !== "" && confirmation !== password);
    clearError("confirm");
  }

  function onPasswordInput() {
    onSecretInput();
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

  async function pickLocation() {
    setPickError(null);
    const picked = await client.pickVaultToCreate();
    if (gone()) return;
    if (!picked.ok) {
      setPickError({ at: "location", error: picked.error });
      return;
    }
    if (picked.value === null) return;
    // The core answers that same path, the key file beside it and whether either exists.
    const located = await client.vaultCreateLocation(name, picked.value);
    if (gone()) return;
    if (!located.ok) {
      setPickError({ at: "location", error: located.error });
      return;
    }
    setChosen(picked.value);
    setPlace({ kind: "known", location: located.value });
    clearError("name", "location");
  }

  async function pickOwnKey() {
    setPickError(null);
    const picked = await client.pickKeyFile();
    if (gone()) return;
    if (!picked.ok) {
      setPickError({ at: "key", error: picked.error });
      return;
    }
    if (picked.value === null) return;
    setOwnKey(picked.value);
    clearError("key");
  }

  function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (phase.kind !== "editing") return;
    // Both fields are emptied before any check, so no branch can leave a secret on screen.
    const password = take(passwordRef.current);
    const confirmation = take(confirmRef.current);
    cancelStrength();
    setScore(null);
    setDiffer(false);
    setRevealed(false);
    setPickError(null);

    if (name.trim() === "") {
      fail({ kind: "no-name" });
      return;
    }
    if (chosen === null && place.kind === "no-documents") {
      fail({ kind: "no-path" });
      return;
    }
    if (password !== confirmation) {
      fail({ kind: "mismatch" });
      return;
    }
    if (keyMode === "existing" && ownKey === null) {
      fail({ kind: "no-key-file" });
      return;
    }
    // RF-3: an empty password is no password; it is never trimmed (RF-41).
    if (password === "" && keyMode === "none") {
      fail({ kind: "app", error: { code: "LlaveVacia" } });
      return;
    }
    // Security review of the interaction pass (P-2): the new key file is written beside the vault, so alone it is
    // no protection. A vault with only a key file is still possible with one of the person's own, kept elsewhere.
    if (password === "" && keyMode === "generate") {
      fail({ kind: "key-beside-needs-password" });
      return;
    }
    setPhase({ kind: "creating" });
    void create(password);
  }

  async function create(password: string) {
    // Asked again now: the path sent is the one the core answers for the name as it is, not for an older keystroke.
    const located = await client.vaultCreateLocation(name, chosen);
    if (gone()) return;
    if (!located.ok) {
      if (chosen === null && located.error.code === "ErrorInterno") {
        setPlace({ kind: "no-documents" });
        fail({ kind: "no-path" });
      } else fail({ kind: "app", error: located.error });
      return;
    }
    const location = located.value;
    setPlace({ kind: "known", location });
    if (location.vaultExists) {
      fail({ kind: "exists" });
      return;
    }
    let keyFile: KeyFileChoice;
    switch (keyMode) {
      case "none":
        keyFile = { kind: "none" };
        break;
      case "existing":
        if (ownKey === null) {
          fail({ kind: "no-key-file" });
          return;
        }
        keyFile = { kind: "existing", path: ownKey };
        break;
      case "generate":
        // RF-99: beside the vault, with the path the core gives; no dialog, and never over an existing file.
        if (location.keyFileExists) {
          fail({ kind: "key-exists" });
          return;
        }
        keyFile = { kind: "generate", path: location.keyFilePath };
        break;
      default:
        return assertNever(keyMode);
    }
    const result = await client.vaultCreate({
      path: location.vaultPath,
      // RF-96: the name as written; the file may be called something else.
      name,
      ...(password === "" ? {} : { password }),
      keyFile,
    });
    if (gone()) return;
    if (!result.ok) {
      fail({ kind: "app", error: result.error });
      return;
    }
    if (keyFile.kind === "none") onDone();
    else setPhase({ kind: "created", keyFilePath: keyFile.path });
  }

  if (phase.kind === "created") {
    return (
      <section aria-labelledby="created-title" className="create">
        <div className="create-column">
          <h2 id="created-title" className="create-title">
            Bóveda creada
          </h2>
          <div className="create-notice" role="status">
            <WarningIcon />
            <div>
              <p>
                Tu bóveda usa el archivo llave <span className="path mono">{phase.keyFilePath}</span>.
              </p>
              <p>
                Sin una copia de ese archivo y sin la contraseña no hay forma de recuperarla. Guarda ahora una copia del
                archivo llave en otro lugar seguro, por ejemplo una memoria USB.
              </p>
            </div>
          </div>
          <div className="create-actions">
            <button type="button" className="btn-primary" onClick={onDone}>
              Ir a la bóveda
            </button>
          </div>
        </div>
      </section>
    );
  }

  const creating = phase.kind === "creating";
  const formError = phase.kind === "editing" ? phase.error : null;
  const location = place.kind === "known" ? place.location : null;

  /** What is wrong at `slot`: what the last attempt said, or else what is already known while filling the form. */
  function messageAt(slot: Slot): Message | null {
    if (formError?.slot === slot) return { text: formError.text, alert: true };
    switch (slot) {
      case "name":
        // RF-98: said before pressing "Crear bóveda".
        return location?.vaultExists === true && chosen === null ? { text: EXISTS_VAULT, alert: true } : null;
      case "location":
        if (pickError?.at === "location") return { text: errorMessage(pickError.error), alert: true };
        return location?.vaultExists === true && chosen !== null ? { text: EXISTS_CHOSEN, alert: true } : null;
      case "confirm":
        return differ ? { text: MISMATCH, alert: false } : null;
      case "key":
        if (pickError?.at === "key") return { text: errorMessage(pickError.error), alert: true };
        return keyMode === "generate" && location?.keyFileExists === true ? { text: EXISTS_KEY, alert: true } : null;
      case "form":
        return null;
      default:
        return assertNever(slot);
    }
  }

  const nameMessage = messageAt("name");
  const locationMessage = messageAt("location");
  const confirmMessage = messageAt("confirm");
  const keyMessage = messageAt("key");
  const secretType = revealed ? "text" : "password";

  return (
    <section aria-labelledby="create-title" className="create">
      <div className="create-column">
        <header>
          <h2 id="create-title" className="create-title">
            Crear bóveda
          </h2>
          <p className="create-lead">Ponle nombre y protégela. Lo demás se puede cambiar después.</p>
        </header>

        <form onSubmit={onSubmit} noValidate className="create-form">
          <fieldset disabled={creating}>
            <legend className="sr-only">Bóveda nueva</legend>

            <div className="create-field">
              <label htmlFor="create-name">Nombre de la bóveda</label>
              <input
                ref={nameRef}
                id="create-name"
                type="text"
                placeholder="Familia"
                autoComplete="off"
                value={name}
                aria-invalid={nameMessage !== null}
                aria-describedby={describedBy(nameMessage !== null && "create-name-error", "create-location")}
                onChange={(event) => {
                  setName(event.target.value);
                  clearError("name");
                }}
              />
              <ErrorLine id="create-name-error" message={nameMessage} />
              <div className="create-line">
                <span id="create-location" className="create-line-text">
                  <FolderIcon />
                  {noDocuments && chosen === null ? (
                    <span>Windows no dio la carpeta Documentos. {NO_PATH}</span>
                  ) : location === null || location.inDefaultFolder ? (
                    <>
                      <span>Se guarda en Documentos › AlmarioKS ›</span>
                      {location !== null && (
                        <span className="create-file mono" title={location.vaultPath}>
                          {location.vaultFile}
                        </span>
                      )}
                    </>
                  ) : (
                    <>
                      <span>Se guarda en</span>
                      <span className="create-file mono" title={location.vaultPath}>
                        {location.vaultPath}
                      </span>
                    </>
                  )}
                </span>
                <button
                  ref={locationButtonRef}
                  type="button"
                  className="create-link"
                  aria-describedby={describedBy(locationMessage !== null && "create-location-error")}
                  onClick={() => void pickLocation()}
                >
                  {noDocuments && chosen === null ? "Elegir ubicación…" : "Cambiar ubicación…"}
                </button>
              </div>
              {chosen !== null && <p className="create-hint">{PATH_HELP}</p>}
              <ErrorLine id="create-location-error" message={locationMessage} />
            </div>

            <div className="create-field">
              <label htmlFor="create-password">Contraseña maestra</label>
              <div className="create-secret">
                <input
                  ref={passwordRef}
                  id="create-password"
                  data-secret=""
                  type={secretType}
                  autoComplete="new-password"
                  autoCapitalize="off"
                  autoCorrect="off"
                  spellCheck={false}
                  aria-describedby="create-strength"
                  onInput={onPasswordInput}
                />
                <button
                  type="button"
                  aria-label={revealed ? "Ocultar contraseña" : "Mostrar contraseña"}
                  title={revealed ? "Ocultar contraseña" : "Mostrar contraseña"}
                  onClick={() => {
                    setRevealed((now) => !now);
                  }}
                >
                  {revealed ? <HideIcon /> : <ShowIcon />}
                </button>
              </div>
              <div id="create-strength" aria-live="polite" className="create-strength">
                {score !== null && (
                  <>
                    <p className="create-strength-line">
                      <meter min={0} max={4} low={2} high={3} optimum={4} value={score} aria-label="Fortaleza" />
                      Fortaleza: {strengthLabel(score)} ({score} de 4)
                    </p>
                    {/* RF-7: below 3 it warns and never blocks; only a score of 0 or 1 is called weak. */}
                    {score < 3 && (
                      <p className="create-hint">
                        {score <= 1
                          ? "Es una contraseña débil. Puedes continuar, pero una más larga y variada protege mejor tu bóveda."
                          : "Todavía es fácil de adivinar. Puedes continuar, pero una más larga y variada protege mejor tu bóveda."}
                      </p>
                    )}
                  </>
                )}
              </div>
            </div>

            <div className="create-field">
              <label htmlFor="create-confirm">Repite la contraseña maestra</label>
              <input
                ref={confirmRef}
                id="create-confirm"
                data-secret=""
                type={secretType}
                autoComplete="new-password"
                autoCapitalize="off"
                autoCorrect="off"
                spellCheck={false}
                aria-invalid={confirmMessage !== null}
                aria-describedby={describedBy(confirmMessage !== null && "create-confirm-error")}
                onInput={onSecretInput}
              />
              <ErrorLine id="create-confirm-error" message={confirmMessage} />
            </div>

            <fieldset
              className="create-keyfile"
              aria-describedby={describedBy("create-key-help", keyMessage !== null && "create-key-error")}
            >
              <legend>Archivo llave (opcional)</legend>
              <div className="create-keys">
                {KEY_OPTIONS.map(({ mode, label }) => (
                  <label key={mode} className="create-key">
                    <input
                      type="radio"
                      name="create-key-mode"
                      checked={keyMode === mode}
                      onChange={() => {
                        setKeyMode(mode);
                        clearError("key");
                      }}
                    />
                    {label}
                  </label>
                ))}
              </div>
              <p id="create-key-help" className="create-hint">
                <KeyHelp mode={keyMode} location={location} />
              </p>
              {keyMode === "existing" && (
                <div className="create-line">
                  <span className="create-line-text">
                    {ownKey === null ? (
                      <span>Todavía no has elegido ninguno.</span>
                    ) : (
                      <span className="create-file mono" title={ownKey}>
                        {fileNameOf(ownKey)}
                      </span>
                    )}
                  </span>
                  <button type="button" className="create-link" onClick={() => void pickOwnKey()}>
                    {ownKey === null ? "Elegir archivo llave…" : "Elegir otro…"}
                  </button>
                </div>
              )}
              <ErrorLine id="create-key-error" message={keyMessage} />
            </fieldset>

            <div className="create-notice">
              <WarningIcon />
              <p>
                Si pierdes la contraseña maestra o el archivo llave, nadie puede abrir la bóveda. No hay recuperación.
              </p>
            </div>

            <div className="create-foot">
              <ErrorLine id="create-error" message={messageAt("form")} />
              <div role="status" aria-live="polite" className="create-progress">
                {creating && (
                  <>
                    <p>Creando la bóveda: derivando la llave. Puede tardar unos segundos.</p>
                    <div className="progress-track">
                      <progress aria-label="Creando la bóveda" />
                    </div>
                  </>
                )}
              </div>
              <div className="create-actions">
                <button type="button" onClick={onBack}>
                  Cancelar
                </button>
                <button type="submit" className="btn-primary">
                  Crear bóveda
                </button>
              </div>
            </div>
          </fieldset>
        </form>
      </div>
    </section>
  );
}
