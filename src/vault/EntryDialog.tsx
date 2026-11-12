import { useEffect, useRef, useState, type SubmitEvent } from "react";

import type { EntryFields, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { EntryDetail } from "../ipc/schemas";
import { WarningIcon } from "../ui/icons";
import { Dialog } from "./Dialog";
import { buildInput, fieldOf, initialValues, saveEntry, type EntryTextField, type EntryValues } from "./entryDraft";
import type { GroupLine } from "./lines";
import { PasswordField, type PasswordFieldHandle } from "./PasswordField";
import { checkEntry, problemText, type EntryFieldKey, type FieldProblem } from "./rules";

/** The group named under the title: its name and its line colour (`assignLines`). */
export interface EntryDialogGroup {
  readonly id: string;
  readonly name: string;
  readonly line: GroupLine;
}

export type EntryDialogMode =
  /** The entry is created in `group`. */
  | { readonly kind: "new"; readonly group: EntryDialogGroup }
  /** `group` is the one the entry is in (`EntryDetail` only brings its id). */
  | { readonly kind: "edit"; readonly entry: EntryDetail; readonly group: EntryDialogGroup };

export interface EntryDialogProps {
  readonly client: Pick<VaultClient, "entryCreate" | "entryUpdate" | "passwordStrength" | "generatePassword">;
  readonly mode: EntryDialogMode;
  /** The dialog asks to go away (after saving, or after closing with nothing to lose): the owner unmounts it. */
  readonly onClose: () => void;
  /** Right before `onClose`. `changed` is false when an edit had nothing to send (no save is on its way). */
  readonly onSaved: (entryId: string, changed: boolean) => void;
}

type Phase =
  | {
      readonly kind: "editing";
      readonly problem: FieldProblem | null;
      readonly error: AppError | null;
      /** The password was emptied before a call that failed: it has to be written again. */
      readonly passwordLost: boolean;
    }
  | { readonly kind: "saving" };

const FIELDS: readonly (keyof EntryValues)[] = ["title", "username", "url", "notes", "tags"];

const PROTECTED_HINT =
  "Este campo está protegido en la bóveda y no se muestra aquí. Déjalo vacío para conservarlo o escribe uno nuevo " +
  "para reemplazarlo.";

function fieldId(field: EntryFieldKey): string {
  return `entry-dialog-${field}`;
}

function messageOf(problem: FieldProblem): string {
  if (problem.field === "title" && problem.code === "TituloVacio") {
    return "Ponle un título para poder encontrarla después.";
  }
  return problemText(problem);
}

/**
 * "Nueva entrada" and "Editar entrada" in a modal dialog (RF-100; RF-38 to RF-41, RF-45, RF-91). The draft lives in
 * this component alone: it goes away when the owner unmounts the dialog, also when the vault locks (RF-32).
 */
export function EntryDialog({ client, mode, onClose, onSaved }: EntryDialogProps) {
  const editing = mode.kind === "edit" ? mode.entry : null;
  const [initial] = useState<EntryValues>(() => initialValues(editing));
  const [values, setValues] = useState<EntryValues>(initial);
  const [phase, setPhase] = useState<Phase>({ kind: "editing", problem: null, error: null, passwordLost: false });
  const [passwordWritten, setPasswordWritten] = useState(false);
  const [moreOpen, setMoreOpen] = useState(
    () => editing !== null && (editing.tags.length > 0 || editing.customFields.length > 0),
  );
  const [confirming, setConfirming] = useState(false);
  const passwordRef = useRef<PasswordFieldHandle>(null);
  const mounted = useRef(false);
  const busy = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  // Back to the field that has the problem.
  useEffect(() => {
    if (phase.kind !== "editing" || phase.problem === null) return;
    document.getElementById(fieldId(phase.problem.field))?.focus();
  }, [phase]);

  const saving = phase.kind === "saving";
  const problem = phase.kind === "editing" ? phase.problem : null;
  const formError = phase.kind === "editing" ? phase.error : null;
  const passwordLost = phase.kind === "editing" && phase.passwordLost;
  const dirty = passwordWritten || FIELDS.some((field) => values[field] !== initial[field]);

  /** Escape, the close button and "Cancelar" (RF-100): it asks only if something would be lost. */
  function requestClose() {
    if (saving) return;
    if (dirty) setConfirming(true);
    else onClose();
  }

  function fail(next: { problem: FieldProblem | null; error: AppError | null; passwordLost: boolean }) {
    // Tags sit in "Más campos": a problem there has to be seen.
    if (next.problem?.field === "tags") setMoreOpen(true);
    setPhase({ kind: "editing", ...next });
  }

  function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy.current) return;
    const input = buildInput(values, editing, passwordRef.current?.peek() ?? "");
    const found = checkEntry(input);
    if (found !== null) {
      fail({ problem: found, error: null, passwordLost: false });
      return;
    }
    // From here on the password is only in `input`, on its way to the core.
    const hadPassword = (passwordRef.current?.take() ?? "") !== "";
    if (editing !== null && Object.keys(input).length === 0) {
      onSaved(editing.id, false);
      onClose();
      return;
    }
    busy.current = true;
    setPhase({ kind: "saving" });
    void save(input, hadPassword);
  }

  async function save(input: Partial<EntryFields>, hadPassword: boolean) {
    const result = await saveEntry(
      client,
      mode.kind === "new" ? { kind: "create", groupId: mode.group.id } : { kind: "edit", id: mode.entry.id },
      input,
    );
    busy.current = false;
    if (!mounted.current) return;
    if (result.ok) {
      onSaved(result.value.id, true);
      onClose();
      return;
    }
    const found = fieldOf(result.error, input);
    fail({ problem: found, error: found === null ? result.error : null, passwordLost: hadPassword });
  }

  function errorFor(field: EntryFieldKey) {
    const id = `${fieldId(field)}-error`;
    const mine = problem?.field === field ? problem : null;
    return {
      id,
      invalid: mine !== null,
      node: (
        <p id={id} role={mine === null ? undefined : "alert"} className="field-error">
          {mine === null ? "" : messageOf(mine)}
        </p>
      ),
    };
  }

  function textField(field: EntryTextField, label: string, placeholder?: string) {
    const error = errorFor(field);
    const isProtected = editing !== null && editing[field] === null;
    const hintId = `${fieldId(field)}-hint`;
    const common = {
      id: fieldId(field),
      value: values[field],
      placeholder,
      spellCheck: false,
      "aria-invalid": error.invalid,
      "aria-describedby": `${error.id}${isProtected ? ` ${hintId}` : ""}`,
    };
    return (
      <div className="entry-field">
        <label htmlFor={fieldId(field)}>{label}</label>
        {field === "notes" ? (
          <textarea
            {...common}
            rows={2}
            onChange={(event) => {
              set(field, event.target.value);
            }}
          />
        ) : (
          <input
            {...common}
            type={field === "url" ? "url" : "text"}
            autoComplete="off"
            autoCapitalize="off"
            data-initial-focus={field === "title" ? "" : undefined}
            onChange={(event) => {
              set(field, event.target.value);
            }}
          />
        )}
        {isProtected && (
          <p id={hintId} className="path">
            {PROTECTED_HINT}
          </p>
        )}
        {error.node}
      </div>
    );
  }

  function set(field: keyof EntryValues, value: string) {
    setValues((current) => ({ ...current, [field]: value }));
  }

  const passwordError = errorFor("password");
  const tagsError = errorFor("tags");
  const customNames = editing === null ? [] : editing.customFields.map((field) => field.name);

  return (
    <>
      <Dialog
        title={editing === null ? "Nueva entrada" : "Editar entrada"}
        layout="form"
        lead={
          <p className="dialog-lead">
            <span className="dialog-dot" data-line={mode.group.line} aria-hidden="true" />
            En el grupo {mode.group.name}
          </p>
        }
        onClose={requestClose}
      >
        <form className="entry-dialog" noValidate onSubmit={onSubmit}>
          <div className="entry-dialog-body">
            <fieldset className="entry-dialog-fields" disabled={saving}>
              {textField("title", "Título", "GitHub")}
              {textField("username", "Usuario o correo", "ana@example.com")}
              <PasswordField
                ref={passwordRef}
                id={fieldId("password")}
                client={client}
                hint={
                  editing === null
                    ? null
                    : "Déjala vacía para conservar la contraseña actual. Para cambiarla, escribe una nueva o genérala."
                }
                invalid={passwordError.invalid}
                errorId={passwordError.id}
                onWrittenChange={setPasswordWritten}
              >
                {passwordLost && (
                  <p className="notice notice-icon">
                    <WarningIcon />
                    La contraseña se borró del formulario por seguridad: vuelve a escribirla o genérala.
                  </p>
                )}
                {passwordError.node}
              </PasswordField>
              {textField("url", "Sitio", "https://github.com")}
              {textField("notes", "Notas")}

              <div className="entry-more">
                <button
                  type="button"
                  className="link-button"
                  aria-expanded={moreOpen}
                  aria-controls={moreOpen ? "entry-dialog-more" : undefined}
                  onClick={() => {
                    setMoreOpen((open) => !open);
                  }}
                >
                  Más campos
                </button>
                {moreOpen && (
                  <div id="entry-dialog-more" className="entry-field">
                    <label htmlFor={fieldId("tags")}>Etiquetas (una por línea)</label>
                    <textarea
                      id={fieldId("tags")}
                      rows={2}
                      value={values.tags}
                      spellCheck={false}
                      aria-invalid={tagsError.invalid}
                      aria-describedby={`entry-dialog-tags-hint ${tagsError.id}`}
                      onChange={(event) => {
                        set("tags", event.target.value);
                      }}
                    />
                    <p id="entry-dialog-tags-hint" className="path">
                      Sin ; ni , dentro de una etiqueta. Hasta 50 etiquetas.
                    </p>
                    {tagsError.node}
                    {customNames.length > 0 && (
                      <p className="path">
                        Campos propios de esta entrada: {customNames.join(", ")}. Se conservan como están; todavía no
                        se editan desde aquí.
                      </p>
                    )}
                  </div>
                )}
              </div>
            </fieldset>
          </div>
          <footer className="entry-dialog-foot">
            <p role="alert" className="error">
              {formError === null ? "" : errorMessage(formError)}
            </p>
            <span role="status" className="sr-only">
              {saving ? "Guardando la entrada…" : ""}
            </span>
            <button type="button" disabled={saving} onClick={requestClose}>
              Cancelar
            </button>
            <button type="submit" className="btn-primary" aria-disabled={saving}>
              {saving ? "Guardando…" : "Guardar entrada"}
            </button>
          </footer>
        </form>
      </Dialog>
      {confirming && (
        <Dialog
          title="¿Descartar los cambios?"
          onClose={() => {
            setConfirming(false);
          }}
        >
          <p>Lo que escribiste en esta entrada no se guardará.</p>
          <div className="actions">
            <button type="button" className="btn-danger" onClick={onClose}>
              Descartar
            </button>
            <button
              type="button"
              data-initial-focus=""
              onClick={() => {
                setConfirming(false);
              }}
            >
              Seguir editando
            </button>
          </div>
        </Dialog>
      )}
    </>
  );
}
