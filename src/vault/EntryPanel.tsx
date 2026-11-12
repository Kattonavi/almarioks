import { useEffect, useRef, useState, type ReactNode, type RefObject, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { FieldName, VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { EntryDetail, GroupNode } from "../ipc/schemas";
import { CopyIcon, EditIcon, HideIcon, OpenLinkIcon, ShowIcon } from "../ui/icons";
import { MenuButton, type MenuItem } from "../ui/MenuButton";
import { Dialog } from "./Dialog";
import { EntryForm } from "./EntryForm";
import { formatDate, formatDay, groupOptions, isInBin, shown, trailTo } from "./format";
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

/**
 * RF-74: what the copy notice has to say. `what` names the field ("Contraseña copiada"), never its value; `deadline`
 * is the moment the core clears the clipboard, or `null` when the copy has no countdown (a field that is not
 * protected).
 */
export interface CopyNotice {
  readonly what: string;
  readonly deadline: number | null;
}

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
   * The clipboard took a value of this entry. The notice lives in the detail zone, not here: this panel goes away
   * with its entry and the countdown has to go on. Nothing of the value travels.
   */
  readonly onCopied?: (copy: CopyNotice) => void;
}

/**
 * KeePass keys of the standard fields, as the core lists them in `protected_fields`, with the name each one has on
 * screen. A map, not an object: a custom field can be called `constructor`.
 */
const STANDARD_LABELS: ReadonlyMap<string, string> = new Map([
  ["Title", "Título"],
  ["UserName", "Usuario"],
  ["Password", "Contraseña"],
  ["URL", "Sitio"],
  ["Notes", "Notas"],
]);

/** RF-74: what the copy notice says of each standard field; a custom one is named in quotes. */
const COPIED: ReadonlyMap<string, string> = new Map([
  ["Title", "Título copiado"],
  ["UserName", "Usuario copiado"],
  ["Password", "Contraseña copiada"],
  ["URL", "Dirección copiada"],
  ["Notes", "Notas copiadas"],
]);

/** The fields with a row of their own in the card: a protected one outside this list gets a row after them. */
const OWN_ROW: readonly string[] = ["UserName", "Password", "URL", "Notes"];

function labelOf(field: FieldName): string {
  return STANDARD_LABELS.get(field) ?? field;
}

/** For button names: "Mostrar contraseña", "Copiar «PIN»". */
function nameOf(field: FieldName): string {
  return STANDARD_LABELS.get(field)?.toLowerCase() ?? `«${field}»`;
}

function copyLabelOf(field: FieldName): string {
  return field === "URL" ? "Copiar dirección del sitio" : `Copiar ${nameOf(field)}`;
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

interface IconButtonProps {
  /** The accessible name and the help under the pointer. It names the field, never its value. */
  readonly label: string;
  readonly onClick: () => void;
  /** The one icon button with a plate: "Copiar contraseña". */
  readonly primary?: boolean;
  readonly buttonRef?: RefObject<HTMLButtonElement | null> | undefined;
  readonly children: ReactNode;
}

/** An icon alone in a 44 px square, at the end of a row of the card. */
function IconButton({ label, onClick, primary = false, buttonRef, children }: IconButtonProps) {
  return (
    <button
      ref={buttonRef}
      type="button"
      className={primary ? "btn-icon btn-primary" : "btn-icon"}
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

interface CardRowProps {
  readonly label: string;
  /** The icon buttons of the row: they close it, so the buttons of every row line up in one column. */
  readonly actions?: ReactNode;
  /** The notes: the label at the top and the text with its own line breaks. */
  readonly notes?: boolean;
  readonly children: ReactNode;
}

/** One row of the card: label | value | icon buttons. */
function CardRow({ label, actions, notes = false, children }: CardRowProps) {
  return (
    <div className={notes ? "card-row card-row-notes" : "card-row"}>
      <dt>{label}</dt>
      <dd>
        {children}
        {actions !== undefined && <span className="card-actions">{actions}</span>}
      </dd>
    </div>
  );
}

/** A value that is not protected: one line, cut with an ellipsis; the whole text is in its `title`. */
function PlainValue({ value, empty }: { readonly value: string | null; readonly empty: string }) {
  if (value === "") return <span className="card-empty">{empty}</span>;
  // `null`: protected in the file, but with nothing to show or copy.
  if (value === null) return <span className="card-empty">{shown(value)}</span>;
  return (
    <span className="card-value" title={value}>
      {value}
    </span>
  );
}

/**
 * The value of a protected field: hidden, a bar of fixed width that never gives away how long it is; shown, the text
 * in a `.secret` element, which cannot be selected or dragged (copying goes through the core). The value is a React
 * text child and nothing else: it never reaches an attribute.
 */
function SecretValue({ value }: { readonly value: string | undefined }) {
  return value === undefined ? (
    <span className="secret" data-masked="">
      <span className="sr-only">(oculto)</span>
    </span>
  ) : (
    <span className="secret">{value}</span>
  );
}

interface FieldActions {
  readonly onReveal: (field: FieldName) => Promise<void>;
  readonly onHide: (field: FieldName) => void;
  readonly onCopy: (field: FieldName) => Promise<void>;
}

function CopyButton({ field, onCopy }: { readonly field: FieldName } & Pick<FieldActions, "onCopy">) {
  return (
    <IconButton label={copyLabelOf(field)} onClick={() => void onCopy(field)}>
      <CopyIcon />
    </IconButton>
  );
}

interface SecretActionsProps extends FieldActions {
  readonly field: FieldName;
  /** Whether its value is on screen right now. Only that: the value itself does not come here. */
  readonly revealed: boolean;
  readonly copyRef?: RefObject<HTMLButtonElement | null> | undefined;
}

/** Show or hide, and copy: the two buttons every protected field has. Only the password's copy has a plate. */
function SecretActions({ field, revealed, copyRef, onReveal, onHide, onCopy }: SecretActionsProps) {
  return (
    <>
      <IconButton
        label={revealed ? `Ocultar ${nameOf(field)}` : `Mostrar ${nameOf(field)}`}
        onClick={() => {
          if (revealed) onHide(field);
          else void onReveal(field);
        }}
      >
        {revealed ? <HideIcon /> : <ShowIcon />}
      </IconButton>
      <IconButton
        label={copyLabelOf(field)}
        primary={field === "Password"}
        buttonRef={copyRef}
        onClick={() => void onCopy(field)}
      >
        <CopyIcon />
      </IconButton>
    </>
  );
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
  /** A map, not an object: the name of a custom field is whatever the file says. */
  const [revealed, setRevealed] = useState<ReadonlyMap<FieldName, string>>(() => new Map());
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
    setRevealed((current) => new Map([...current].filter(([name]) => name !== field)));
  }

  function hideAll() {
    for (const timer of timers.current.values()) clearTimeout(timer);
    timers.current.clear();
    setRevealed(new Map());
  }

  async function reveal(field: FieldName) {
    setActionError(null);
    const result = await client.entryReveal(entryId, field);
    if (!mounted.current) return;
    if (!result.ok) {
      setActionError(result.error);
      return;
    }
    setRevealed((current) => new Map([...current, [field, result.value]]));
    clearTimeout(timers.current.get(field));
    timers.current.set(
      field,
      setTimeout(() => {
        hide(field);
      }, REVEAL_MS),
    );
  }

  /** RF-71: always through the core; the window never holds what it copies. */
  async function copy(field: FieldName) {
    setActionError(null);
    say("");
    const result = await client.entryCopy(entryId, field);
    if (!mounted.current) return;
    if (!result.ok) {
      setActionError(result.error);
      return;
    }
    // Told here, not from an effect: this panel goes away with its entry, and the notice must stay. The core says
    // whether the clipboard clears by itself (a protected field) and in how long.
    const seconds = result.value.clearsInSeconds;
    onCopied?.({
      what: COPIED.get(field) ?? `«${field}» copiado`,
      deadline: seconds === null ? null : Date.now() + seconds * 1000,
    });
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
      <div className="entry">
        <h4 className="entry-title">{shown(listTitle, "(sin título)")}</h4>
        {load.kind === "loading" ? (
          <p role="status">Cargando la entrada…</p>
        ) : (
          <>
            <p role="alert" className="error">
              {errorMessage(load.error)}
            </p>
            <div className="actions">
              <button
                type="button"
                onClick={() => {
                  setLoad({ kind: "loading" });
                  setAttempt((value) => value + 1);
                }}
              >
                Reintentar
              </button>
            </div>
          </>
        )}
      </div>
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
  const isSecret = (field: FieldName) => secretFields.includes(field);
  const plainCustom = entry.customFields.filter((field) => !field.protected);
  const has = (value: string | null) => value !== null && value !== "";
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

  const openButton = (
    <IconButton label="Abrir el sitio en el navegador" onClick={() => void openUrl()}>
      <OpenLinkIcon />
    </IconButton>
  );

  /** The two buttons of a protected field, wired to this entry. */
  function secretActions(field: FieldName) {
    return (
      <SecretActions
        field={field}
        revealed={revealed.has(field)}
        copyRef={field === "Password" ? copyPasswordRef : undefined}
        onReveal={reveal}
        onHide={hide}
        onCopy={copy}
      />
    );
  }

  return (
    <div className="entry">
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
      <dl className="entry-card">
        {isSecret("UserName") ? (
          <CardRow label="Usuario" actions={secretActions("UserName")}>
            <SecretValue value={revealed.get("UserName")} />
          </CardRow>
        ) : (
          <CardRow
            label="Usuario"
            actions={has(entry.username) ? <CopyButton field="UserName" onCopy={copy} /> : undefined}
          >
            <PlainValue value={entry.username} empty="Sin usuario" />
          </CardRow>
        )}
        {isSecret("Password") ? (
          <CardRow label="Contraseña" actions={secretActions("Password")}>
            <SecretValue value={revealed.get("Password")} />
          </CardRow>
        ) : (
          <CardRow label="Contraseña">
            <span className="card-empty">Sin contraseña</span>
          </CardRow>
        )}
        {isSecret("URL") ? (
          <CardRow
            label="Sitio"
            actions={
              <>
                {openButton}
                {secretActions("URL")}
              </>
            }
          >
            <SecretValue value={revealed.get("URL")} />
          </CardRow>
        ) : (
          <CardRow
            label="Sitio"
            actions={
              entry.url === "" ? undefined : (
                <>
                  {openButton}
                  {has(entry.url) && <CopyButton field="URL" onCopy={copy} />}
                </>
              )
            }
          >
            <PlainValue value={entry.url} empty="Sin sitio" />
          </CardRow>
        )}
        {/* What the file protects beyond the rows above: a protected title and the protected custom fields. */}
        {secretFields
          .filter((field) => !OWN_ROW.includes(field))
          .map((field) => (
            <CardRow key={`secret:${field}`} label={labelOf(field)} actions={secretActions(field)}>
              <SecretValue value={revealed.get(field)} />
            </CardRow>
          ))}
        {plainCustom.map((field) => (
          <CardRow
            key={`custom:${field.name}`}
            label={field.name}
            actions={has(field.value) ? <CopyButton field={field.name} onCopy={copy} /> : undefined}
          >
            <PlainValue value={field.value ?? ""} empty="Sin valor" />
          </CardRow>
        ))}
        {entry.tags.length > 0 && (
          <CardRow label="Etiquetas">
            <ul className="tags">
              {entry.tags.map((tag) => (
                <li key={tag}>{tag}</li>
              ))}
            </ul>
          </CardRow>
        )}
        {entry.attachments.length > 0 && (
          <CardRow label="Adjuntos">
            <ul className="attachments">
              {entry.attachments.map((attachment) => (
                <li key={attachment.name}>
                  <span>{attachment.name}</span> <span className="path">({attachment.size} bytes)</span>
                </li>
              ))}
            </ul>
          </CardRow>
        )}
        {/* The notes close the card, without buttons unless the file protects them. */}
        {isSecret("Notes") ? (
          <CardRow label="Notas" notes actions={secretActions("Notes")}>
            <SecretValue value={revealed.get("Notes")} />
          </CardRow>
        ) : (
          <CardRow label="Notas" notes>
            {entry.notes === null ? (
              <span className="card-empty">{shown(entry.notes)}</span>
            ) : entry.notes === "" ? (
              <span className="card-empty">Sin notas</span>
            ) : (
              <span className="card-notes">{entry.notes}</span>
            )}
          </CardRow>
        )}
      </dl>

      {revealed.size > 0 && <p className="path field-hint">Lo que muestres se oculta solo a los 20 segundos.</p>}

      <p className="entry-dates">
        Creada el{" "}
        <time dateTime={entry.created} title={formatDate(entry.created)}>
          {formatDay(entry.created)}
        </time>{" "}
        · Modificada el{" "}
        <time dateTime={entry.modified} title={formatDate(entry.modified)}>
          {formatDay(entry.modified)}
        </time>
      </p>

      {readOnly && <p className="path">Solo lectura: esta entrada no se puede editar, mover ni borrar.</p>}

      {/* What an order left to say. A copy is not said here: its notice is the one at the foot of the detail zone. */}
      <div aria-live="polite" aria-atomic="true" className="entry-status">
        {status !== "" && <p>{status}</p>}
        {status === "" && savedText !== "" && <p>{savedText}</p>}
      </div>
      <p role="alert" className="error">
        {actionError === null ? "" : errorMessage(actionError)}
      </p>

      {renderDialog()}
    </div>
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
