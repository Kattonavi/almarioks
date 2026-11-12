import { useEffect, useId, useRef, useState, type SubmitEvent } from "react";

import type { EntryFields, EntryPatch, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { EntryDetail } from "../ipc/schemas";
import { GenerateIcon, WarningIcon } from "../ui/icons";
import { shown, strengthLabel } from "./format";
import { Generator } from "./Generator";
import { checkEntry, entryFieldOf, parseTags, problemText, type EntryFieldKey, type FieldProblem } from "./rules";

/** RF-70: wait this long after the last keystroke before asking the core for the strength. */
export const ENTRY_STRENGTH_DELAY_MS = 150;

export type EntryFormMode =
  | { readonly kind: "create"; readonly groupId: string; readonly groupName: string }
  | { readonly kind: "edit"; readonly entry: EntryDetail };

type TextField = "title" | "username" | "url" | "notes";

interface Values {
  readonly title: string;
  readonly username: string;
  readonly url: string;
  readonly notes: string;
  readonly tags: string;
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

interface EntryFormProps {
  readonly client: Pick<VaultClient, "entryCreate" | "entryUpdate" | "passwordStrength" | "generatePassword">;
  readonly mode: EntryFormMode;
  /** `changed` is false when an edit had nothing to send. */
  readonly onSaved: (entry: EntryDetail, changed: boolean) => void;
  readonly onCancel: () => void;
}

const LABELS: Readonly<Record<EntryFieldKey, string>> = {
  title: "Título",
  username: "Usuario",
  password: "Contraseña",
  url: "URL",
  notes: "Notas",
  tags: "Etiquetas (una por línea)",
};

/** Reads a secret field and empties it at once: the value lives only in the caller's stack (ADR-0002). */
function take(field: HTMLInputElement | null): string {
  const value = field?.value ?? "";
  if (field !== null) field.value = "";
  return value;
}

function initialValues(mode: EntryFormMode): Values {
  if (mode.kind === "create") return { title: "", username: "", url: "", notes: "", tags: "" };
  const { entry } = mode;
  // A protected field (`null`) is never preloaded: like the password, it is sent only if something is written.
  return {
    title: entry.title ?? "",
    username: entry.username ?? "",
    url: entry.url ?? "",
    notes: entry.notes ?? "",
    tags: entry.tags.join("\n"),
  };
}

/**
 * Which core error belongs to which field when the core rejects what the window let through. The core names the field
 * and the limit (plan.md, precision 15); the window's own deduction is the fallback when it does not.
 */
function fieldOf(error: AppError, input: Partial<EntryFields>): FieldProblem | null {
  const local = (): FieldProblem | null => {
    const found = checkEntry(input);
    return found !== null && found.code === error.code ? found : null;
  };
  switch (error.code) {
    case "TituloVacio":
      return { field: "title", code: "TituloVacio" };
    case "EtiquetaInvalida":
      return { field: entryFieldOf(error.field) ?? "tags", code: "EtiquetaInvalida" };
    case "CampoMuyLargo": {
      const field = entryFieldOf(error.field);
      if (field === null) return local();
      return { field, code: "CampoMuyLargo", ...(error.limit === undefined ? {} : { limit: error.limit }) };
    }
    case "CaracterNoPermitido": {
      const field = entryFieldOf(error.field);
      return field === null ? local() : { field, code: "CaracterNoPermitido" };
    }
    default:
      return null;
  }
}

/**
 * P5, create and edit (RF-38 to RF-41, RF-45, RF-70, RF-91). Text fields are controlled; the password is not React
 * state: it is read and emptied right before the call. An edit is a patch: only what the owner changed travels, and
 * the password only if a new one was written or generated (example 8).
 */
export function EntryForm({ client, mode, onSaved, onCancel }: EntryFormProps) {
  const [values, setValues] = useState<Values>(() => initialValues(mode));
  const [phase, setPhase] = useState<Phase>({ kind: "editing", problem: null, error: null, passwordLost: false });
  const [score, setScore] = useState<number | null>(null);
  const [generatorOpen, setGeneratorOpen] = useState(false);
  const passwordRef = useRef<HTMLInputElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const strengthSeq = useRef(0);
  const strengthTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(false);
  const headingId = useId();
  const editing = mode.kind === "edit" ? mode.entry : null;

  useEffect(() => {
    mounted.current = true;
    titleRef.current?.focus();
    const timer = strengthTimer;
    const seq = strengthSeq;
    return () => {
      mounted.current = false;
      clearTimeout(timer.current);
      seq.current += 1;
    };
  }, []);

  // Back to the field that has the problem.
  useEffect(() => {
    if (phase.kind !== "editing") return;
    if (phase.problem !== null) document.getElementById(`entry-${phase.problem.field}`)?.focus();
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
        if (!mounted.current || mine !== strengthSeq.current) return;
        setScore(result.ok ? result.value.score : null);
      });
    }, ENTRY_STRENGTH_DELAY_MS);
  }

  function applyGenerated(password: string, generatedScore: number) {
    cancelStrength();
    if (passwordRef.current !== null) passwordRef.current.value = password;
    setScore(generatedScore);
    setGeneratorOpen(false);
    passwordRef.current?.focus();
  }

  function closeGenerator() {
    setGeneratorOpen(false);
    toggleRef.current?.focus();
  }

  function set(field: keyof Values, value: string) {
    setValues((current) => ({ ...current, [field]: value }));
  }

  /** What travels: everything on create; on edit, only what changed (RF-45). */
  function buildInput(password: string): Partial<EntryFields> {
    const tags = parseTags(values.tags);
    if (editing === null) return { ...values, tags, password };
    const changed = (field: TextField) => {
      const before = editing[field];
      return before === null ? values[field] !== "" : values[field] !== before;
    };
    // Compared as written, like the other fields: tags the owner did not touch are not sent again.
    const tagsChanged = values.tags !== editing.tags.join("\n");
    return {
      ...(changed("title") ? { title: values.title } : {}),
      ...(changed("username") ? { username: values.username } : {}),
      ...(password === "" ? {} : { password }),
      ...(changed("url") ? { url: values.url } : {}),
      ...(changed("notes") ? { notes: values.notes } : {}),
      ...(tagsChanged ? { tags } : {}),
    };
  }

  function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (phase.kind !== "editing") return;
    const input = buildInput(passwordRef.current?.value ?? "");
    const problem = checkEntry(input);
    if (problem !== null) {
      setPhase({ kind: "editing", problem, error: null, passwordLost: false });
      return;
    }
    // From here on the password is only in this stack frame.
    const password = take(passwordRef.current);
    cancelStrength();
    setScore(null);
    setGeneratorOpen(false);
    if (editing !== null && Object.keys(input).length === 0) {
      onSaved(editing, false);
      return;
    }
    setPhase({ kind: "saving" });
    void save(input, password !== "");
  }

  async function save(input: Partial<EntryFields>, hadPassword: boolean) {
    const result =
      mode.kind === "create"
        ? await client.entryCreate(mode.groupId, {
            title: input.title ?? "",
            username: input.username ?? "",
            password: input.password ?? "",
            url: input.url ?? "",
            notes: input.notes ?? "",
            tags: input.tags ?? [],
          })
        : await client.entryUpdate(mode.entry.id, input satisfies EntryPatch);
    if (!mounted.current) return;
    if (result.ok) {
      onSaved(result.value, true);
      return;
    }
    const problem = fieldOf(result.error, input);
    setPhase({
      kind: "editing",
      problem,
      error: problem === null ? result.error : null,
      passwordLost: hadPassword,
    });
  }

  const saving = phase.kind === "saving";
  const problem = phase.kind === "editing" ? phase.problem : null;

  function errorFor(field: EntryFieldKey) {
    const id = `entry-${field}-error`;
    return {
      describedBy: id,
      invalid: problem?.field === field,
      node: (
        <p id={id} role={problem?.field === field ? "alert" : undefined} className="error">
          {problem?.field === field ? problemText(problem) : ""}
        </p>
      ),
    };
  }

  function textField(field: TextField, type: "text" | "url" = "text") {
    const error = errorFor(field);
    const isProtected = editing !== null && editing[field] === null;
    const hintId = `entry-${field}-hint`;
    return (
      <div className="field">
        <label htmlFor={`entry-${field}`}>{LABELS[field]}</label>
        {field === "notes" ? (
          <textarea
            id={`entry-${field}`}
            rows={4}
            value={values.notes}
            spellCheck={false}
            aria-invalid={error.invalid}
            aria-describedby={`${error.describedBy}${isProtected ? ` ${hintId}` : ""}`}
            onChange={(event) => {
              set(field, event.target.value);
            }}
          />
        ) : (
          <input
            ref={field === "title" ? titleRef : undefined}
            id={`entry-${field}`}
            type={type}
            value={values[field]}
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            aria-invalid={error.invalid}
            aria-describedby={`${error.describedBy}${isProtected ? ` ${hintId}` : ""}`}
            onChange={(event) => {
              set(field, event.target.value);
            }}
          />
        )}
        {isProtected && (
          <p id={hintId} className="path">
            Este campo está protegido en la bóveda y no se muestra aquí. Déjalo vacío para conservarlo o escribe uno
            nuevo para reemplazarlo.
          </p>
        )}
        {error.node}
      </div>
    );
  }

  const passwordError = errorFor("password");
  const tagsError = errorFor("tags");
  const formError = phase.kind === "editing" ? phase.error : null;
  const passwordLost = phase.kind === "editing" && phase.passwordLost;

  return (
    <form onSubmit={onSubmit} noValidate aria-labelledby={headingId} className="entry-form">
      <h4 id={headingId}>
        {editing === null
          ? `Nueva entrada en ${mode.kind === "create" ? mode.groupName : ""}`
          : `Editar ${shown(editing.title, "(sin título)")}`}
      </h4>
      <fieldset disabled={saving}>
        <legend>{editing === null ? "Datos de la entrada nueva" : "Datos de la entrada"}</legend>
        {textField("title")}
        {textField("username")}

        <div className="field">
          <label htmlFor="entry-password">Contraseña</label>
          <input
            ref={passwordRef}
            id="entry-password"
            type="password"
            autoComplete="new-password"
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            aria-invalid={passwordError.invalid}
            aria-describedby={`entry-password-hint entry-strength ${passwordError.describedBy}`}
            onInput={onPasswordInput}
          />
          <p id="entry-password-hint" className="path">
            {editing === null
              ? "Escríbela o genérala. Se guarda tal cual, con sus espacios y mayúsculas."
              : "Déjala vacía para conservar la contraseña actual. Para cambiarla, escribe una nueva o genérala."}
          </p>
          <div id="entry-strength" aria-live="polite" className="strength">
            {score !== null && (
              <p className="strength-line">
                Fortaleza: {strengthLabel(score)} ({score} de 4){" "}
                <meter min={0} max={4} low={2} high={3} optimum={4} value={score} aria-label="Fortaleza" />
              </p>
            )}
          </div>
          {passwordLost && (
            <p className="notice notice-icon">
              <WarningIcon />
              La contraseña se borró del formulario por seguridad: vuelve a escribirla o genérala.
            </p>
          )}
          {passwordError.node}
          <div className="actions">
            <button
              ref={toggleRef}
              type="button"
              aria-expanded={generatorOpen}
              aria-controls={generatorOpen ? "entry-generator" : undefined}
              onClick={() => {
                if (generatorOpen) closeGenerator();
                else setGeneratorOpen(true);
              }}
            >
              <GenerateIcon />
              Generar contraseña
            </button>
          </div>
          {generatorOpen && (
            <div id="entry-generator">
              <Generator client={client} onUse={applyGenerated} onClose={closeGenerator} />
            </div>
          )}
        </div>

        {textField("url", "url")}
        {textField("notes")}

        <div className="field">
          <label htmlFor="entry-tags">{LABELS.tags}</label>
          <textarea
            id="entry-tags"
            rows={2}
            value={values.tags}
            spellCheck={false}
            aria-invalid={tagsError.invalid}
            aria-describedby={`entry-tags-hint ${tagsError.describedBy}`}
            onChange={(event) => {
              set("tags", event.target.value);
            }}
          />
          <p id="entry-tags-hint" className="path">
            Sin ; ni , dentro de una etiqueta. Hasta 50 etiquetas.
          </p>
          {tagsError.node}
        </div>

        <div className="actions">
          <button type="submit" className="btn-primary">
            Guardar entrada
          </button>
          <button type="button" onClick={onCancel}>
            Cancelar
          </button>
        </div>
      </fieldset>
      <div aria-live="polite">{saving && <p role="status">Guardando la entrada…</p>}</div>
      <p role="alert" className="error">
        {formError === null ? "" : errorMessage(formError)}
      </p>
    </form>
  );
}
