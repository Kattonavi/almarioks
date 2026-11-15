import { useState, type ReactNode, type SubmitEvent } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { DeletePreview, GroupNode } from "../ipc/schemas";
import { Dialog } from "./Dialog";
import { groupOptions, isInBin, plural, shown, trailTo } from "./format";
import { checkGroupName, groupNameText, type GroupNameProblem } from "./rules";

export type GroupClient = Pick<
  VaultClient,
  "groupCreate" | "groupRename" | "groupMove" | "groupDeletePreview" | "groupDelete" | "recycleBinEmpty"
>;

/** What can be asked of a group (P7). */
export type GroupOrder = "create" | "rename" | "move" | "delete" | "empty-bin";

/** One line of the menu of a group. */
export interface GroupOrderItem {
  readonly order: GroupOrder;
  readonly label: string;
  /** The order that destroys or sends to the bin: drawn in the danger role, always the last one. */
  readonly danger: boolean;
}

/**
 * The orders a group takes, in menu order. The core decides what is protected; the menu only leaves out what can
 * never apply (moving the root, renaming the bin, a new group inside the bin). A new group in the root is not here:
 * it is the "+" of the rail.
 */
export function groupOrders(root: GroupNode, group: GroupNode): readonly GroupOrderItem[] {
  if (group.kind === "recycle_bin") return [{ order: "empty-bin", label: "Vaciar papelera…", danger: true }];
  const rename: GroupOrderItem = { order: "rename", label: "Renombrar…", danger: false };
  if (group.kind === "root") return [rename];
  const inBin = isInBin(root, group.id);
  return [
    ...(inBin ? [] : [{ order: "create", label: "Nuevo subgrupo…", danger: false } as const]),
    rename,
    { order: "move", label: "Mover a otro grupo…", danger: false },
    // Outside the bin a group goes to the bin; the order says so, like its dialog.
    { order: "delete", label: inBin ? "Borrar…" : "Enviar a la papelera…", danger: true },
  ];
}

type GroupDialog =
  | { readonly kind: "create"; readonly group: GroupNode }
  | { readonly kind: "rename"; readonly group: GroupNode }
  | { readonly kind: "move"; readonly group: GroupNode }
  | { readonly kind: "delete"; readonly group: GroupNode; readonly preview: DeletePreview; readonly forGood: boolean }
  | { readonly kind: "empty-bin"; readonly group: GroupNode };

/** What changed, for the screen that owns the tree. */
export interface GroupChange {
  readonly root: GroupNode;
  /** The group that left its place (sent to the bin or deleted), if any: a list that showed it shows something else. */
  readonly gone: string | null;
  /** Entries left the view (deleted, bin emptied): the detail is closed and focus goes back to the tree. */
  readonly removed: boolean;
}

/** The group orders of the rail: `start` runs one, `view` is what it leaves to say and its open dialog. */
export interface GroupActions {
  readonly start: (group: GroupNode, order: GroupOrder) => void;
  readonly view: ReactNode;
}

function nameOf(group: GroupNode): string {
  return `«${shown(group.name, "(sin nombre)")}»`;
}

function contentText(preview: DeletePreview): string {
  return `${plural(preview.entries, "entrada", "entradas")} y ${plural(preview.subgroups, "subgrupo", "subgrupos")}`;
}

/** Content of the recycle bin, counted from the tree the window already has. */
function binContent(bin: GroupNode): DeletePreview {
  const walk = (node: GroupNode): DeletePreview =>
    node.children.reduce(
      (sum, child) => {
        const inner = walk(child);
        return { entries: sum.entries + inner.entries, subgroups: sum.subgroups + 1 + inner.subgroups };
      },
      { entries: node.entryCount, subgroups: 0 },
    );
  return walk(bin);
}

/**
 * P7 (RF-51 to RF-58): create, rename, move and delete a group, and empty the recycle bin. Every order is asked from
 * the menu of its group (or the "+" of the rail) and answered in a native dialog: no dragging. One order at a time,
 * for whichever group asked; `root` is `null` until the tree arrives.
 */
export function useGroupActions(
  client: GroupClient,
  root: GroupNode | null,
  onChange: (change: GroupChange) => void,
): GroupActions {
  const [dialog, setDialog] = useState<GroupDialog | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);

  async function askDelete(group: GroupNode, forGood: boolean) {
    if (busy) return;
    setError(null);
    setStatus("");
    setBusy(true);
    const result = await client.groupDeletePreview(group.id);
    setBusy(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setDialog({ kind: "delete", group, preview: result.value, forGood });
  }

  function start(group: GroupNode, order: GroupOrder) {
    switch (order) {
      case "create":
      case "rename":
      case "move":
      case "empty-bin":
        setError(null);
        setStatus("");
        setDialog({ kind: order, group });
        return;
      case "delete":
        // What is already in the bin is deleted for good; the dialog says which of the two it is.
        void askDelete(group, root !== null && isInBin(root, group.id));
        return;
      default:
        assertNever(order);
    }
  }

  function done(tree: GroupNode, message: string, gone: string | null, removed: boolean) {
    setDialog(null);
    setStatus(message);
    onChange({ root: tree, gone, removed });
  }

  function close() {
    setDialog(null);
  }

  const view = (
    <div className="rail-status">
      <div aria-live="polite">{status !== "" && <p>{status}</p>}</div>
      <p role="alert" className="error">
        {error === null || dialog !== null ? "" : errorMessage(error)}
      </p>
      {root !== null && renderDialog(root)}
    </div>
  );

  return { start, view };

  function renderDialog(tree: GroupNode) {
    if (dialog === null) return null;
    const { group } = dialog;
    switch (dialog.kind) {
      case "create":
        return (
          <NameDialog
            title={`Nuevo grupo dentro de ${nameOf(group)}`}
            initial=""
            submitLabel="Crear grupo"
            onClose={close}
            onSubmit={(name) => client.groupCreate(group.id, name)}
            onDone={(next) => {
              done(next, "Grupo creado.", null, false);
            }}
          />
        );
      case "rename":
        return (
          <NameDialog
            title={`Renombrar ${nameOf(group)}`}
            initial={group.name}
            submitLabel="Guardar nombre"
            onClose={close}
            onSubmit={(name) => client.groupRename(group.id, name)}
            onDone={(next) => {
              done(next, "Grupo renombrado.", null, false);
            }}
          />
        );
      case "move":
        return (
          <MoveGroupDialog
            client={client}
            root={tree}
            group={group}
            onClose={close}
            onDone={(next, targetName) => {
              done(next, `${nameOf(group)} se movió a ${targetName}.`, null, false);
            }}
          />
        );
      case "delete":
        return (
          <ConfirmDialog
            title={
              dialog.forGood ? `Borrar ${nameOf(group)} de forma definitiva` : `Enviar ${nameOf(group)} a la papelera`
            }
            text={
              dialog.forGood
                ? `${nameOf(group)} está en la papelera con ${contentText(dialog.preview)}. Si lo borras, se elimina todo para siempre y no se puede deshacer.`
                : `${nameOf(group)} contiene ${contentText(dialog.preview)}. Todo pasa a la papelera y desde ahí se puede recuperar.`
            }
            confirmLabel={dialog.forGood ? "Borrar definitivamente" : "Enviar a la papelera"}
            onClose={close}
            onConfirm={() => client.groupDelete(group.id)}
            onDone={(next) => {
              done(
                next,
                dialog.forGood ? `Se borró ${nameOf(group)} de forma definitiva.` : `${nameOf(group)} se movió a la papelera.`,
                group.id,
                true,
              );
            }}
          />
        );
      case "empty-bin":
        return (
          <ConfirmDialog
            title="Vaciar la papelera"
            text={`Se borrarán de forma definitiva ${contentText(binContent(group))}. No se puede deshacer.`}
            confirmLabel="Vaciar papelera"
            onClose={close}
            onConfirm={() => client.recycleBinEmpty()}
            onDone={(next) => {
              done(next, "La papelera quedó vacía.", null, true);
            }}
          />
        );
      default:
        return assertNever(dialog);
    }
  }
}

interface NameDialogProps {
  readonly title: string;
  readonly initial: string;
  readonly submitLabel: string;
  readonly onClose: () => void;
  readonly onSubmit: (name: string) => Promise<Result<GroupNode>>;
  readonly onDone: (tree: GroupNode) => void;
}

type NameError = { readonly kind: "name"; readonly problem: GroupNameProblem } | { readonly kind: "app"; readonly error: AppError };

/** RF-51 to RF-53: the name is sent as written; the core trims it. */
function NameDialog({ title, initial, submitLabel, onClose, onSubmit, onDone }: NameDialogProps) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState<NameError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const problem = checkGroupName(name);
    if (problem !== null) {
      setError({ kind: "name", problem });
      return;
    }
    setBusy(true);
    const result = await onSubmit(name);
    setBusy(false);
    if (result.ok) {
      onDone(result.value);
      return;
    }
    const code = result.error.code;
    setError(
      code === "NombreVacio" || code === "CampoMuyLargo" || code === "CaracterNoPermitido"
        ? { kind: "name", problem: code }
        : { kind: "app", error: result.error },
    );
  }

  return (
    <Dialog title={title} onClose={onClose}>
      <form onSubmit={(event) => void submit(event)} noValidate>
        <div className="field">
          <label htmlFor="group-name">Nombre del grupo</label>
          <input
            id="group-name"
            type="text"
            value={name}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={error?.kind === "name"}
            aria-describedby="group-name-error"
            onChange={(event) => {
              setName(event.target.value);
            }}
          />
        </div>
        <p id="group-name-error" role="alert" className="error">
          {error === null ? "" : error.kind === "name" ? groupNameText(error.problem) : errorMessage(error.error)}
        </p>
        <div className="actions">
          <button type="submit" className="btn-primary" disabled={busy}>
            {submitLabel}
          </button>
          <button type="button" onClick={onClose}>
            Cancelar
          </button>
        </div>
      </form>
    </Dialog>
  );
}

interface MoveGroupDialogProps {
  readonly client: Pick<VaultClient, "groupMove">;
  readonly root: GroupNode;
  readonly group: GroupNode;
  readonly onClose: () => void;
  readonly onDone: (tree: GroupNode, targetName: string) => void;
}

const SELF_HELP = "No aparecen el propio grupo ni sus subgrupos: un grupo no puede ir dentro de sí mismo.";

/** RF-54, RF-55: destinations are every group but itself, its subgroups, its current parent and the recycle bin. */
function MoveGroupDialog({ client, root, group, onClose, onDone }: MoveGroupDialogProps) {
  const parentId = trailTo(root, group.id)?.at(-2)?.id;
  const options = groupOptions(root, (node) => node.id === group.id || node.kind === "recycle_bin").filter(
    (option) => option.node.id !== parentId,
  );
  const [target, setTarget] = useState(options[0]?.node.id ?? "");
  /** RF-55: its subgroups are left out on purpose; say so, or the missing destinations look like a fault. */
  const selfHelp = group.children.length > 0 ? SELF_HELP : null;
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    const chosen = options.find((option) => option.node.id === target);
    if (chosen === undefined || busy) return;
    setBusy(true);
    const result = await client.groupMove(group.id, chosen.node.id);
    setBusy(false);
    if (result.ok) onDone(result.value, nameOf(chosen.node));
    else setError(result.error);
  }

  return (
    <Dialog title={`Mover ${nameOf(group)}`} onClose={onClose}>
      <form onSubmit={(event) => void submit(event)}>
        {options.length === 0 ? (
          <>
            <p>No hay otro grupo al que moverlo.</p>
            {selfHelp !== null && <p className="path">{selfHelp}</p>}
          </>
        ) : (
          <div className="field">
            <label htmlFor="move-group-target">Grupo de destino</label>
            <select
              id="move-group-target"
              aria-describedby={selfHelp === null ? undefined : "move-group-help"}
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
            {selfHelp !== null && (
              <p id="move-group-help" className="path field-hint">
                {selfHelp}
              </p>
            )}
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

interface ConfirmDialogProps {
  readonly title: string;
  readonly text: string;
  readonly confirmLabel: string;
  readonly onClose: () => void;
  readonly onConfirm: () => Promise<Result<GroupNode>>;
  readonly onDone: (tree: GroupNode) => void;
}

/** A destructive action: says what is lost and waits for an explicit confirmation. */
function ConfirmDialog({ title, text, confirmLabel, onClose, onConfirm, onDone }: ConfirmDialogProps) {
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  async function confirm() {
    if (busy) return;
    setBusy(true);
    const result = await onConfirm();
    setBusy(false);
    if (result.ok) onDone(result.value);
    else setError(result.error);
  }

  return (
    <Dialog title={title} tone="danger" onClose={onClose}>
      <p>{text}</p>
      <p role="alert" className="error">
        {error === null ? "" : errorMessage(error)}
      </p>
      <div className="actions">
        <button type="button" className="btn-danger" disabled={busy} onClick={() => void confirm()}>
          {confirmLabel}
        </button>
        <button type="button" data-initial-focus="" onClick={onClose}>
          Cancelar
        </button>
      </div>
    </Dialog>
  );
}
