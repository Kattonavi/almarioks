import {
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";

import type { GroupNode } from "../ipc/schemas";
import { ListIcon, MoreIcon, PlusIcon, TrashIcon } from "../ui/icons";
import { liveEntryCount, shown } from "./format";
import { groupOrders, type GroupOrder } from "./GroupActions";
import type { GroupLine } from "./lines";

/** The recycle bin, wherever it hangs: a vault made elsewhere can have it under a group (RF-57). */
function findBin(node: GroupNode): GroupNode | undefined {
  if (node.kind === "recycle_bin") return node;
  for (const child of node.children) {
    const found = findBin(child);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** What every row of the rail needs, handed down the tree as is. */
interface RowContext {
  readonly root: GroupNode;
  readonly lines: ReadonlyMap<string, GroupLine>;
  /** The marked row: a group id, `null` for "Todas las entradas", `undefined` for none (a search is on). */
  readonly current: string | null | undefined;
  readonly readOnly: boolean;
  readonly onSelect: (id: string | null) => void;
  readonly onOrder: (group: GroupNode, order: GroupOrder) => void;
}

interface GroupRailProps {
  readonly root: GroupNode;
  readonly lines: ReadonlyMap<string, GroupLine>;
  /** The group the list shows; `null` is "Todas las entradas". */
  readonly selectedId: string | null;
  /** `false` while a search is on: the list shows results, so nothing is marked in the rail. */
  readonly marked: boolean;
  /** RF-14: the rail only navigates; nothing that changes a group is offered. */
  readonly readOnly: boolean;
  /** The "Grupos" heading: the focus target after a group leaves its place. */
  readonly titleRef: RefObject<HTMLHeadingElement | null>;
  readonly onSelect: (id: string | null) => void;
  readonly onOrder: (group: GroupNode, order: GroupOrder) => void;
  /** What the last group order left to say, between the groups and the recycle bin. */
  readonly children?: ReactNode;
}

/**
 * The rail of the open vault (RF-50, RF-102): "Todas las entradas" first, then the groups (the root is not one of
 * them), and the recycle bin pinned to the foot. It only navigates: what can be asked of a group is in its menu, and
 * a new group in the root is the "+" next to the heading.
 */
export function GroupRail({
  root,
  lines,
  selectedId,
  marked,
  readOnly,
  titleRef,
  onSelect,
  onOrder,
  children,
}: GroupRailProps) {
  const context: RowContext = {
    root,
    lines,
    current: marked ? selectedId : undefined,
    readOnly,
    onSelect,
    onOrder,
  };
  const bin = findBin(root);
  const total = liveEntryCount(root);
  return (
    <>
      <button
        type="button"
        className="rail-item rail-all"
        aria-current={context.current === null ? "true" : undefined}
        onClick={() => {
          onSelect(null);
        }}
      >
        <ListIcon />
        {/* The text of the button is "Todas las entradas · N entradas": the parts that are not drawn are hidden. */}
        <span className="rail-name">Todas las entradas</span> <span className="sr-only">·</span>{" "}
        <span className="rail-count">{total}</span>{" "}
        <span className="sr-only">{total === 1 ? "entrada" : "entradas"}</span>
      </button>
      <div className="rail-groups-head">
        <h3 id="groups-title" ref={titleRef} tabIndex={-1}>
          Grupos
        </h3>
        {!readOnly && (
          <button
            type="button"
            className="rail-add"
            aria-label="Nuevo grupo"
            title="Nuevo grupo"
            onClick={() => {
              onOrder(root, "create");
            }}
          >
            <PlusIcon />
          </button>
        )}
      </div>
      <div className="rail-groups">
        <GroupList nodes={root.children} depth={0} bin={false} context={context} />
      </div>
      {children}
      {bin !== undefined && (
        <div className="rail-bin">
          <GroupList nodes={[bin]} depth={0} bin context={context} />
        </div>
      )}
    </>
  );
}

interface GroupListProps {
  readonly nodes: readonly GroupNode[];
  /** 0: first level; deeper levels are indented. */
  readonly depth: number;
  /** The list of the recycle bin: the only one that shows it, with the groups that were sent to it. */
  readonly bin: boolean;
  readonly context: RowContext;
}

/** RF-50: nested lists of buttons; Tab moves, Enter or Space opens the group, `aria-current` marks the open one. */
function GroupList({ nodes, depth, bin, context }: GroupListProps) {
  const rows = bin ? nodes : nodes.filter((node) => node.kind !== "recycle_bin");
  if (rows.length === 0) return null;
  return (
    <ul className="groups">
      {rows.map((node) => {
        const name = shown(node.name, "(sin nombre)");
        const isBin = node.kind === "recycle_bin";
        const current = node.id === context.current;
        return (
          <li key={node.id}>
            <div className="rail-row" data-depth={Math.min(depth, 3)} data-current={current ? "" : undefined}>
              <button
                type="button"
                className="rail-item"
                title={name}
                aria-current={current ? "true" : undefined}
                onClick={() => {
                  context.onSelect(node.id);
                }}
              >
                {isBin ? (
                  <TrashIcon />
                ) : (
                  <span className="rail-dot" data-line={context.lines.get(node.id) ?? "neutral"} aria-hidden="true" />
                )}
                {/*
                  The text of the button stays "Nombre · N entradas" (its accessible name): the parts that are not
                  drawn are hidden, and every space is its own text node so it is not trimmed out of the name.
                */}
                <span className="rail-name">{name}</span>
                {isBin && " "}
                {isBin && <span className="sr-only">(papelera)</span>} <span className="sr-only">·</span>{" "}
                <span className="rail-count">{node.entryCount}</span>{" "}
                <span className="sr-only">{node.entryCount === 1 ? "entrada" : "entradas"}</span>
              </button>
              {!context.readOnly && <GroupMenuButton root={context.root} group={node} onOrder={context.onOrder} />}
            </div>
            <GroupList nodes={node.children} depth={depth + 1} bin={bin} context={context} />
          </li>
        );
      })}
    </ul>
  );
}

interface GroupMenuButtonProps {
  readonly root: GroupNode;
  readonly group: GroupNode;
  readonly onOrder: (group: GroupNode, order: GroupOrder) => void;
}

/**
 * The "more actions" button of a group and its menu. The button opens and closes it; choosing, Escape, a click
 * outside or the focus leaving close it. Arrows, Home and End move through the orders. On closing from the keyboard
 * or by choosing, the focus is back on the button.
 */
export function GroupMenuButton({ root, group, onOrder }: GroupMenuButtonProps) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLSpanElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const name = shown(group.name, "(sin nombre)");
  const items = groupOrders(root, group);

  useEffect(() => {
    if (!open) return;
    menuRef.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    function onPointerDown(event: PointerEvent) {
      if (event.target instanceof Node && wrapRef.current?.contains(event.target) === true) return;
      setOpen(false);
    }
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
    };
  }, [open]);

  function choose(order: GroupOrder) {
    // The focus goes first: the dialog that opens now gives it back to whatever has it (RF-51), and this menu item
    // is about to leave the page.
    buttonRef.current?.focus();
    setOpen(false);
    onOrder(group, order);
  }

  function onKeyDown(event: ReactKeyboardEvent<HTMLSpanElement>) {
    if (!open) {
      if (event.key !== "ArrowDown") return;
      event.preventDefault();
      setOpen(true);
      return;
    }
    const orders = [...(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
    const index = orders.findIndex((order) => order === document.activeElement);
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        // From the button (no order has the focus yet) down is the first one and up is the last one.
        const next = index === -1 ? (step === 1 ? 0 : orders.length - 1) : index + step;
        orders[(next + orders.length) % orders.length]?.focus();
        return;
      }
      case "Home":
        event.preventDefault();
        orders[0]?.focus();
        return;
      case "End":
        event.preventDefault();
        orders.at(-1)?.focus();
        return;
      case "Escape":
        event.preventDefault();
        buttonRef.current?.focus();
        setOpen(false);
        return;
      default:
        return;
    }
  }

  return (
    <span
      ref={wrapRef}
      className="rail-menu"
      onKeyDown={onKeyDown}
      onBlur={(event) => {
        // The focus left the button and its menu (Tab, or a click somewhere else).
        if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
        setOpen(false);
      }}
    >
      <button
        ref={buttonRef}
        type="button"
        className="rail-more"
        aria-label={`Más acciones de ${name}`}
        title="Más acciones"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => {
          setOpen((value) => !value);
        }}
      >
        <MoreIcon />
      </button>
      {open && (
        <div ref={menuRef} id={menuId} role="menu" aria-label={`Acciones de ${name}`} className="menu">
          {items.map((item) => (
            <button
              key={item.order}
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={item.danger ? "menu-danger" : undefined}
              onClick={() => {
                choose(item.order);
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </span>
  );
}
