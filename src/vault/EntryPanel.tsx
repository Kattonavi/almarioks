import { useEffect, useRef, useState, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { FieldName, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { EntryDetail, GroupNode } from "../ipc/schemas";
import { CopyIcon, EditIcon, HideIcon, OpenLinkIcon, ShowIcon } from "../ui/icons";
import { MenuButton, type MenuItem } from "../ui/MenuButton";
import { Dialog } from "./Dialog";
import { EntryForm } from "./EntryForm";
import { formatDate, groupOptions, isInBin, shown, trailTo } from "./format";
import { assignLines, entryInitial, entryLine } from "./lines";
import { NO_SAVE_SIGNAL, savingText, type SaveSignal } from "./saveSignal";

/** RF-21: an edit the core applied, before and after its save reaches the disk. */
const WAITING = "Cambios aplicados. Guardando la entrada en el archivo…";
const DONE = "Entrada guardada.";

/** RF-44: a revealed value hides itself after this long. */
export const REVEAL_MS = 20_000;

export type EntryClient = Pick<
  VaultClient,
  | "entryGet"
  | "entryReveal"
  | "entryCopy"
  | "entryUpdate"
  | "entryCreate"
  | "entryMove"
  | "entryDelete"
  | "entryOpenUrl"
  | "passwordStrength"
  | "generatePassword"
>;

type LoadState =
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly error: AppError }
  | { readonly kind: "ready"; readonly entry: EntryDetail };

type DialogState = { readonly kind: "move" } | { readonly kind: "delete" } | null;

/** What the menu of an entry offers. */
type EntryOrder = "move" | "bin" | "delete";

interface EntryPanelProps {
  readonly client: EntryClient;
  readonly entryId: string;
  /** The title from the list, shown while the detail arrives. */
  readonly listTitle: string | null;
  readonly root: GroupNode;
  /** RF-14: view, reveal and copy only. */
  readonly readOnly?: boolean;
  /** RF-21: saves that reached the disk; an edit is "guardada" only after the next one. */
  readonly saveSignal?: SaveSignal;
  /**
   * RF-59: opened from the keyboard. A new non-zero value moves the focus, once the entry is loaded, to "Copiar
   * contraseña" (or to the title when it has none), so copying is one key away. 0: leave the focus where it is.
   */
  readonly focusRequest?: number;
  /**
   * The smallest window shows one zone at a time: this goes back to the list. Offered only while viewing, so an edit
   * in progress is never dropped by it; the stylesheet shows it only where the list is hidden.
   */
  readonly onBack?: () => void;
  /** The entry changed (edited, moved): the lists must be asked again. */
  readonly onChanged: () => void;
  /** The entry left this view (to the recycle bin or for good). */
  readonly onDeleted: (notice: string) => void;
  /**
   * The clipboard took a value of this entry: the moment the core clears it, or `null` when the copy has no
   * countdown. For the list, which stamps the row; nothing of the value travels.
   */
  readonly onCopied?: (deadline: number | null) => void;
}

/** KeePass keys of the standard fields, as the core lists them in `protected_fields`. */
const STANDARD_LABELS: Readonly<Record<string, string>> = {
  Title: "Título",
  UserName: "Usuario",
  Password: "Contraseña",
  URL: "URL",
  Notes: "Notas",
};

function labelOf(field: FieldName): string {
  return STANDARD_LABELS[field] ?? field;
}

/** For button names: "Mostrar contraseña", "Copiar «PIN»". */
function nameOf(field: FieldName): string {
  const standard = STANDARD_LABELS[field];
  return standard === undefined ? `«${field}»` : field === "URL" ? "URL" : standard.toLowerCase();
}

function titleOf(entry: EntryDetail): string {
  return entry.title === null || entry.title === "" ? "la entrada" : `«${entry.title}»`;
}

/** Protected fields with a value, standard and custom, without repeats. */
function protectedOf(entry: EntryDetail): FieldName[] {
  const names = [...entry.protectedFields];
  for (const custom of entry.customFields) if (custom.protected && !names.includes(custom.name)) names.push(custom.name);
  return names;
}

/**
 * P5, view (RF-43, RF-44, RF-46 to RF-49, RF-71, RF-74). Mounted with `key={entryId}`: another entry, a lock or a
 * close unmounts it, and every revealed value goes with it. Revealed values live only in this component's state.
 */
export function EntryPanel({
  client,
  entryId,
  listTitle,
  root,
  readOnly = false,
  saveSignal = NO_SAVE_SIGNAL,
  focusRequest = 0,
  onBack,
  onChanged,
  onDeleted,
  onCopied,
}: EntryPanelProps) {
  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [mode, setMode] = useState<"view" | "edit">("view");
  const [revealed, setRevealed] = useState<Readonly<Record<FieldName, string>>>({});
  const [copied, setCopied] = useState<{ readonly deadline: number } | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [actionError, setActionError] = useState<AppError | null>(null);
  const [status, setStatus] = useState("");
  /** An edit applied when `saveSignal.saved` was this value, waiting for its save to reach the disk. */
  const [editSince, setEditSince] = useState<number | null>(null);
  const [dialog, setDialog] = useState<DialogState>(null);
  /** Any other message replaces the save notice of an earlier edit. */
  function say(text: string) {
    setStatus(text);
    setEditSince(null);
  }
  const timers = useRef(new Map<FieldName, ReturnType<typeof setTimeout>>());
  const mounted = useRef(false);
  const editRef = useRef<HTMLButtonElement>(null);
  const copyPasswordRef = useRef<HTMLButtonElement>(null);
  const entryTitleRef = useRef<HTMLHeadingElement>(null);
  const focusHandled = useRef(0);
  const focusEdit = useRef(false);

  useEffect(() => {
    mounted.current = true;
    const pending = timers.current;
    return () => {
      mounted.current = false;
      for (const timer of pending.values()) clearTimeout(timer);
      pending.clear();
    };
  }, []);

  useEffect(() => {
    let active = true;
    void client.entryGet(entryId).then((result) => {
      if (!active) return;
      setLoad(result.ok ? { kind: "ready", entry: result.value } : { kind: "failed", error: result.error });
    });
    return () => {
      active = false;
    };
  }, [client, entryId, attempt]);

  // RF-74: the countdown of the clipboard, from the seconds the core gave.
  useEffect(() => {
    if (copied === null) return;
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  }, [copied]);

  useEffect(() => {
    if (focusRequest === 0 || focusHandled.current === focusRequest || load.kind !== "ready") return;
    focusHandled.current = focusRequest;
    (copyPasswordRef.current ?? entryTitleRef.current)?.focus();
  }, [focusRequest, load]);

  useEffect(() => {
    if (mode === "view" && focusEdit.current) {
      focusEdit.current = false;
      editRef.current?.focus();
    }
  }, [mode]);

  function hide(field: FieldName) {
    clearTimeout(timers.current.get(field));
    timers.current.delete(field);
    setRevealed((current) => Object.fromEntries(Object.entries(current).filter(([name]) => name !== field)));
  }

  function hideAll() {
    for (const timer of timers.current.values()) clearTimeout(timer);
    timers.current.clear();
    setRevealed({});
  }

  async function reveal(field: FieldName) {
    setActionError(null);
    const result = await client.entryReveal(entryId, field);
    if (!mounted.current) return;
    if (!result.ok) {
      setActionError(result.error);
      return;
    }
    setRevealed((current) => ({ ...current, [field]: result.value }));
    clearTimeout(timers.current.get(field));
    timers.current.set(
      field,
      setTimeout(() => {
        hide(field);
      }, REVEAL_MS),
    );
  }

  async function copy(field: FieldName) {
    setActionError(null);
    say("");
    const result = await client.entryCopy(entryId, field);
    if (!mounted.current) return;
    if (!result.ok) {
      setActionError(result.error);
      return;
    }
    if (result.value.clearsInSeconds === null) {
      // A field that is not protected: no countdown, and the person knows it stays on the clipboard.
      setCopied(null);
      onCopied?.(null);
      say(`${labelOf(field)}: copiado. No es un dato protegido, así que el portapapeles no se limpia solo.`);
      return;
    }
    const at = Date.now();
    setNow(at);
    setCopied({ deadline: at + result.value.clearsInSeconds * 1000 });
    // Told here, not from an effect: this panel goes away with its entry, and the stamp on the row must stay.
    onCopied?.(at + result.value.clearsInSeconds * 1000);
  }

  async function openUrl() {
    setActionError(null);
    say("");
    const result = await client.entryOpenUrl(entryId);
    if (!mounted.current) return;
    if (result.ok) say("La dirección se abrió en el navegador.");
    else setActionError(result.error);
  }

  async function remove(entry: EntryDetail, forGood: boolean) {
    setActionError(null);
    const result = await client.entryDelete(entryId);
    if (!mounted.current) return;
    if (!result.ok) {
      setDialog(null);
      setActionError(result.error);
      return;
    }
    onDeleted(
      forGood
        ? `Se borró ${titleOf(entry)} de forma definitiva.`
        : `${titleOf(entry)} se movió a la papelera. Para recuperarla, ábrela en la papelera y muévela a otro grupo.`,
    );
  }

  if (load.kind !== "ready") {
    return (
      <>
        <h4 className="entry-title">{shown(listTitle, "(sin título)")}</h4>
        {load.kind === "loading" ? (
          <p role="status">Cargando la entrada…</p>
        ) : (
          <>
            <p role="alert" className="error">
              {errorMessage(load.error)}
            </p>
            <button
              type="button"
              onClick={() => {
                setLoad({ kind: "loading" });
                setAttempt((value) => value + 1);
              }}
            >
              Reintentar
            </button>
          </>
        )}
      </>
    );
  }

  const { entry } = load;

  if (mode === "edit") {
    return (
      <EntryForm
        client={client}
        mode={{ kind: "edit", entry }}
        onSaved={(saved, changed) => {
          setLoad({ kind: "ready", entry: saved });
          setStatus(changed ? "" : "No había cambios que guardar.");
          setEditSince(changed ? saveSignal.saved : null);
          focusEdit.current = true;
          setMode("view");
          if (changed) onChanged();
        }}
        onCancel={() => {
          focusEdit.current = true;
          setMode("view");
        }}
      />
    );
  }

  const inBin = isInBin(root, entry.groupId);
  const savedText = editSince === null ? "" : savingText(saveSignal, editSince, WAITING, DONE);
  const secretFields = protectedOf(entry);
  const plainCustom = entry.customFields.filter((field) => !field.protected);
  const secondsLeft = copied === null ? null : Math.max(0, Math.ceil((copied.deadline - now) / 1000));
  /** The groups from the root down to the entry's own: the header names the last one, its `title` the whole way. */
  const trail = (trailTo(root, entry.groupId) ?? []).map((group) => shown(group.name, "(sin nombre)"));
  const line = entryLine(assignLines(root).get(entry.groupId));
  /**
   * The menu of the entry. Outside the recycle bin, sending it there is the last order, and it acts at once (RF-47:
   * it can be undone from the bin). Inside the bin, moving it out is how it is restored, and deleting asks first.
   */
  const orders: readonly MenuItem<EntryOrder>[] = [
    { order: "move", label: "Mover a otro grupo…" },
    inBin
      ? { order: "delete", label: "Borrar definitivamente…", danger: true }
      : { order: "bin", label: "Enviar a la papelera…", danger: true },
  ];

  function onOrder(order: EntryOrder) {
    setActionError(null);
    switch (order) {
      case "move":
        setDialog({ kind: "move" });
        return;
      case "bin":
        void remove(entry, false);
        return;
      case "delete":
        setDialog({ kind: "delete" });
        return;
      default:
        assertNever(order);
    }
  }

  return (
    <>
      {onBack !== undefined && (
        <button type="button" className="back-to-list" onClick={onBack}>
          Volver a la lista
        </button>
      )}
      <header className="entry-head">
        {/* The same circle as its row in the list, larger. Drawn by CSS from `data-initial`: it adds no text. */}
        <span
          className="bullet entry-badge"
          data-line={line}
          data-initial={entryInitial(entry.title)}
          aria-hidden="true"
        />
        <div className="entry-heading">
          <h4 ref={entryTitleRef} tabIndex={-1} className="entry-title">
            {shown(entry.title, "(sin título)")}
          </h4>
          <p className="entry-group" title={trail.join(" › ")}>
            <span className="sr-only">Grupo:</span>
            <span className="group-dot" data-line={line} aria-hidden="true" /> {trail.at(-1) ?? ""}
          </p>
        </div>
        {!readOnly && (
          <div className="entry-head-actions">
            <button
              ref={editRef}
              type="button"
              onClick={() => {
                hideAll();
                say("");
                setActionError(null);
                setMode("edit");
              }}
            >
              <EditIcon />
              Editar
            </button>
            <MenuButton
              label="Más acciones de la entrada"
              menuLabel="Acciones de la entrada"
              items={orders}
              onChoose={onOrder}
              className="entry-menu"
              buttonClassName="entry-more"
            />
          </div>
        )}
      </header>
      <dl className="entry-meta">
        <div>
          <dt>Creada</dt>
          <dd>
            <time dateTime={entry.created}>{formatDate(entry.created)}</time>
          </dd>
        </div>
        <div>
          <dt>Modificada</dt>
          <dd>
            <time dateTime={entry.modified}>{formatDate(entry.modified)}</time>
          </dd>
        </div>
      </dl>

      <dl className="entry-fields">
        <div>
          <dt>Usuario</dt>
          <dd>
            <span title={entry.username === null || entry.username === "" ? undefined : entry.username}>{shown(entry.username, "(vacío)")}</span>
            {entry.username !== null && entry.username !== "" && (
              <span className="field-actions">
                <button type="button" aria-label="Copiar usuario" onClick={() => void copy("UserName")}>
                  <CopyIcon />
                  <span>
                    Copiar<span className="label-more"> usuario</span>
                  </span>
                </button>
              </span>
            )}
          </dd>
        </div>
      </dl>

      {secretFields.length > 0 && (
        <>
          <h5 className="sr-only">Campos protegidos</h5>
          <dl className="entry-fields">
            {secretFields.map((field) => {
              const value = revealed[field];
              return (
                <div key={field}>
                  <dt>{labelOf(field)}</dt>
                  <dd>
                    {value === undefined ? (
                      // A bar of fixed width: the hidden value never gives away its length.
                      <span className="secret" data-masked="">
                        <span className="sr-only">(oculto)</span>
                      </span>
                    ) : (
                      <span className="secret">{value}</span>
                    )}
                    <span className="field-actions">
                      <button
                        type="button"
                        className="btn-icon"
                        title={value === undefined ? `Mostrar ${nameOf(field)}` : `Ocultar ${nameOf(field)}`}
                        onClick={() => {
                          if (value === undefined) void reveal(field);
                          else hide(field);
                        }}
                      >
                        {value === undefined ? <ShowIcon /> : <HideIcon />}
                        <span className="sr-only">
                          {value === undefined ? `Mostrar ${nameOf(field)}` : `Ocultar ${nameOf(field)}`}
                        </span>
                      </button>
                      <button
                        type="button"
                        ref={field === "Password" ? copyPasswordRef : undefined}
                        className={field === "Password" ? "btn-primary" : undefined}
                        aria-label={`Copiar ${nameOf(field)}`}
                        onClick={() => void copy(field)}
                      >
                        <CopyIcon />
                        <span>
                          Copiar<span className="label-more"> {nameOf(field)}</span>
                        </span>
                      </button>
                    </span>
                  </dd>
                </div>
              );
            })}
          </dl>
          <p className="path field-hint">Lo que muestres se oculta solo a los 20 segundos.</p>
        </>
      )}

      <dl className="entry-fields">
        <div>
          <dt>URL</dt>
          <dd>
            <span className="path" title={entry.url === null || entry.url === "" ? undefined : entry.url}>
              {shown(entry.url, "(vacía)")}
            </span>
            {entry.url !== "" && (
              <span className="field-actions">
                <button type="button" aria-label="Abrir URL en el navegador" onClick={() => void openUrl()}>
                  <OpenLinkIcon />
                  <span>
                    Abrir<span className="label-more"> URL en el navegador</span>
                  </span>
                </button>
              </span>
            )}
          </dd>
        </div>
        <div>
          <dt>Etiquetas</dt>
          <dd>
            {entry.tags.length === 0 ? (
              <span>(sin etiquetas)</span>
            ) : (
              <ul className="tags">
                {entry.tags.map((tag) => (
                  <li key={tag}>{tag}</li>
                ))}
              </ul>
            )}
          </dd>
        </div>
        <div>
          <dt>Notas</dt>
          <dd>
            <span className="notes">{shown(entry.notes, "(sin notas)")}</span>
          </dd>
        </div>
      </dl>

      {plainCustom.length > 0 && (
        <>
          <h5>Campos propios</h5>
          <dl className="entry-fields">
            {plainCustom.map((field) => (
              <div key={field.name}>
                <dt>{field.name}</dt>
                <dd>{field.value ?? ""}</dd>
              </div>
            ))}
          </dl>
        </>
      )}

      {entry.attachments.length > 0 && (
        <>
          <h5>Adjuntos</h5>
          <ul>
            {entry.attachments.map((attachment) => (
              <li key={attachment.name}>
                <span>{attachment.name}</span> <span className="path">({attachment.size} bytes)</span>
              </li>
            ))}
          </ul>
        </>
      )}

      {readOnly && <p className="path">Solo lectura: esta entrada no se puede editar, mover ni borrar.</p>}

      <div aria-live="polite" aria-atomic="true" className="entry-status">
        {secondsLeft !== null && (
          <p>
            {secondsLeft > 0
              ? `Copiado: se borra en ${String(secondsLeft)} s`
              : "Se cumplió el tiempo: AlmarioKS limpió el portapapeles si nadie copió otra cosa."}
          </p>
        )}
        {status !== "" && <p>{status}</p>}
        {status === "" && savedText !== "" && <p>{savedText}</p>}
      </div>
      <p role="alert" className="error">
        {actionError === null ? "" : errorMessage(actionError)}
      </p>

      {renderDialog()}
    </>
  );

  function renderDialog() {
    if (dialog === null) return null;
    switch (dialog.kind) {
      case "move":
        return (
          <MoveEntryDialog
            client={client}
            entry={entry}
            root={root}
            onClose={() => {
              setDialog(null);
            }}
            onMoved={(groupName) => {
              setDialog(null);
              say(`${titleOf(entry)} se movió a ${groupName}.`);
              setAttempt((value) => value + 1);
              onChanged();
            }}
          />
        );
      case "delete":
        return (
          <Dialog
            title={`Borrar ${titleOf(entry)} de forma definitiva`}
            tone="danger"
            onClose={() => {
              setDialog(null);
            }}
          >
            <p>Está en la papelera: si la borras, se elimina para siempre y no se puede deshacer.</p>
            <div className="actions">
              <button type="button" className="btn-danger" onClick={() => void remove(entry, true)}>
                Borrar definitivamente
              </button>
              <button
                type="button"
                data-initial-focus=""
                onClick={() => {
                  setDialog(null);
                }}
              >
                Cancelar
              </button>
            </div>
          </Dialog>
        );
      default:
        return assertNever(dialog);
    }
  }
}

interface MoveEntryDialogProps {
  readonly client: Pick<VaultClient, "entryMove">;
  readonly entry: EntryDetail;
  readonly root: GroupNode;
  readonly onClose: () => void;
  readonly onMoved: (groupName: string) => void;
}

/** RF-46: the destination by a `<select>`; the recycle bin is not a destination (that is "Borrar"). */
function MoveEntryDialog({ client, entry, root, onClose, onMoved }: MoveEntryDialogProps) {
  const options = groupOptions(root, (node) => node.kind === "recycle_bin").filter(
    (option) => option.node.id !== entry.groupId,
  );
  const [target, setTarget] = useState(options[0]?.node.id ?? "");
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  async function onSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    const chosen = options.find((option) => option.node.id === target);
    if (chosen === undefined || busy) return;
    setBusy(true);
    const result = await client.entryMove(entry.id, chosen.node.id);
    setBusy(false);
    if (result.ok) onMoved(shown(chosen.node.name, "(sin nombre)"));
    else setError(result.error);
  }

  return (
    <Dialog title={`Mover ${titleOf(entry)}`} onClose={onClose}>
      <form onSubmit={(event) => void onSubmit(event)}>
        {options.length === 0 ? (
          <p>No hay otro grupo al que moverla. Crea uno primero.</p>
        ) : (
          <div className="field">
            <label htmlFor="move-entry-target">Grupo de destino</label>
            <select
              id="move-entry-target"
              value={target}
              onChange={(event) => {
                setTarget(event.target.value);
              }}
            >
              {options.map((option) => (
                <option key={option.node.id} value={option.node.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
        )}
        <p role="alert" className="error">
          {error === null ? "" : errorMessage(error)}
        </p>
        <div className="actions">
          <button type="submit" className="btn-primary" disabled={options.length === 0 || busy}>
            Mover
          </button>
          <button type="button" onClick={onClose}>
            Cancelar
          </button>
        </div>
      </form>
    </Dialog>
  );
}
