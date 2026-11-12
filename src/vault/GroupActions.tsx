import { useState, type SubmitEvent } from "react";

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

type GroupDialog =
  | { readonly kind: "create" }
  | { readonly kind: "rename" }
  | { readonly kind: "move" }
  /** `groupId`: the preview belongs to this group; a late one for another selection is not shown. */
  | { readonly kind: "delete"; readonly groupId: string; readonly preview: DeletePreview; readonly forGood: boolean }
  | { readonly kind: "empty-bin" };

/** What changed, for the screen that owns the tree. */
export interface GroupChange {
  readonly root: GroupNode;
  /** The group to open next. */
  readonly select: string;
  /** Entries left the view (deleted, moved): the detail is closed and focus goes back to the tree. */
  readonly removed: boolean;
}

interface GroupActionsProps {
  readonly client: GroupClient;
  readonly root: GroupNode;
  readonly group: GroupNode;
  readonly onChange: (change: GroupChange) => void;
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
 * P7 (RF-51 to RF-58): create, rename, move and delete the selected group, and empty the recycle bin. Every action is
 * a button and a native dialog: no dragging. The core decides what is protected; the buttons only hide what can never
 * apply (moving the root, renaming the bin).
 */
export function GroupActions({ client, root, group, onChange }: GroupActionsProps) {
  const [dialog, setDialog] = useState<GroupDialog | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const inBin = isInBin(root, group.id);
  const isBin = group.kind === "recycle_bin";
  const isRoot = group.kind === "root";

  function open(next: GroupDialog) {
    setError(null);
    setStatus("");
    setDialog(next);
  }

  async function askDelete() {
    setError(null);
    setStatus("");
    setBusy(true);
    const result = await client.groupDeletePreview(group.id);
    setBusy(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setDialog({ kind: "delete", groupId: group.id, preview: result.value, forGood: inBin });
  }

  function done(root: GroupNode, message: string, select: string, removed: boolean) {
    setDialog(null);
    setStatus(message);
    onChange({ root, select, removed });
  }

  return (
    <div className="group-actions">
      <div className="actions" role="group" aria-label={`Acciones del grupo ${nameOf(group)}`}>
        {!isBin && !inBin && (
          <button
            type="button"
            onClick={() => {
              open({ kind: "create" });
            }}
          >
            Nuevo grupo…
          </button>
        )}
        {!isBin && (
          <button
            type="button"
            onClick={() => {
              open({ kind: "rename" });
            }}
          >
            Renombrar grupo…
          </button>
        )}
        {!isBin && !isRoot && (
          <button
            type="button"
            onClick={() => {
              open({ kind: "move" });
            }}
          >
            Mover grupo…
          </button>
        )}
        {!isBin && !isRoot && (
          <button type="button" disabled={busy} onClick={() => void askDelete()}>
            Borrar grupo…
          </button>
        )}
        {isBin && (
          <button
            type="button"
            onClick={() => {
              open({ kind: "empty-bin" });
            }}
          >
            Vaciar papelera…
          </button>
        )}
      </div>
      <div aria-live="polite">{status !== "" && <p>{status}</p>}</div>
      <p role="alert" className="error">
        {error === null || dialog !== null ? "" : errorMessage(error)}
      </p>
      {renderDialog()}
    </div>
  );

  function close() {
    setDialog(null);
  }

  function renderDialog() {
    if (dialog === null) return null;
    switch (dialog.kind) {
      case "create":
        return (
          <NameDialog
            title={`Nuevo grupo dentro de ${nameOf(group)}`}
            initial=""
            submitLabel="Crear grupo"
            onClose={close}
            onSubmit={(name) => client.groupCreate(group.id, name)}
            onDone={(tree) => {
              done(tree, "Grupo creado.", group.id, false);
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
            onDone={(tree) => {
              done(tree, "Grupo renombrado.", group.id, false);
            }}
          />
        );
      case "move":
        return (
          <MoveGroupDialog
            client={client}
            root={root}
            group={group}
            onClose={close}
            onDone={(tree, targetName) => {
              done(tree, `${nameOf(group)} se movió a ${targetName}.`, group.id, false);
            }}
          />
        );
      case "delete":
        if (dialog.groupId !== group.id) return null;
        return (
          <ConfirmDialog
            title={dialog.forGood ? `Borrar ${nameOf(group)} de forma definitiva` : `Borrar ${nameOf(group)}`}
            text={
              dialog.forGood
                ? `${nameOf(group)} está en la papelera con ${contentText(dialog.preview)}. Si lo borras, se elimina todo para siempre y no se puede deshacer.`
                : `${nameOf(group)} contiene ${contentText(dialog.preview)}. Al borrarlo, todo pasa a la papelera y desde ahí se puede recuperar.`
            }
            confirmLabel={dialog.forGood ? "Borrar definitivamente" : "Mover a la papelera"}
            onClose={close}
            onConfirm={() => client.groupDelete(group.id)}
            onDone={(tree) => {
              done(
                tree,
                dialog.forGood ? `Se borró ${nameOf(group)} de forma definitiva.` : `${nameOf(group)} se movió a la papelera.`,
                root.id,
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
            onDone={(tree) => {
              done(tree, "La papelera quedó vacía.", group.id, true);
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
          <button type="submit" disabled={busy}>
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

/** RF-54, RF-55: destinations are every group but itself, its subgroups, its current parent and the recycle bin. */
function MoveGroupDialog({ client, root, group, onClose, onDone }: MoveGroupDialogProps) {
  const parentId = trailTo(root, group.id)?.at(-2)?.id;
  const options = groupOptions(root, (node) => node.id === group.id || node.kind === "recycle_bin").filter(
    (option) => option.node.id !== parentId,
  );
  const [target, setTarget] = useState(options[0]?.node.id ?? "");
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
          <p>No hay otro grupo al que moverlo.</p>
        ) : (
          <div className="field">
            <label htmlFor="move-group-target">Grupo de destino</label>
            <select
              id="move-group-target"
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
          <button type="submit" disabled={options.length === 0 || busy}>
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
    <Dialog title={title} onClose={onClose}>
      <p>{text}</p>
      <p role="alert" className="error">
        {error === null ? "" : errorMessage(error)}
      </p>
      <div className="actions">
        <button type="button" disabled={busy} onClick={() => void confirm()}>
          {confirmLabel}
        </button>
        <button type="button" onClick={onClose}>
          Cancelar
        </button>
      </div>
    </Dialog>
  );
}
