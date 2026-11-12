import type { GroupNode } from "../ipc/schemas";

export function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

/** A protected standard field arrives as `null` and is shown as "(protegido)", never asked for. */
export function shown(value: string | null, empty = ""): string {
  if (value === null) return "(protegido)";
  return value === "" ? empty : value;
}

const DATE_FORMAT = new Intl.DateTimeFormat("es-CO", { dateStyle: "medium", timeStyle: "short" });

export function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : DATE_FORMAT.format(date);
}

const DAY_FORMAT = new Intl.DateTimeFormat("es-CO", { day: "numeric", month: "short", year: "numeric" });

/** The day alone, for a row of the entry list; the full date and time stay in the detail. */
export function formatDay(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  // "10 nov 2026": the parts alone, without the "de" and the period the locale adds.
  return DAY_FORMAT.formatToParts(date)
    .filter((part) => part.type !== "literal")
    .map((part) => part.value.replace(".", ""))
    .join(" ");
}

export function strengthLabel(score: number): string {
  return ["muy débil", "débil", "regular", "buena", "muy buena"][score] ?? "";
}

export function findGroup(node: GroupNode, id: string): GroupNode | undefined {
  if (node.id === id) return node;
  for (const child of node.children) {
    const found = findGroup(child, id);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** Groups from the root down to `id`, or `undefined` if it is not in the tree. */
export function trailTo(node: GroupNode, id: string): GroupNode[] | undefined {
  if (node.id === id) return [node];
  for (const child of node.children) {
    const found = trailTo(child, id);
    if (found !== undefined) return [node, ...found];
  }
  return undefined;
}

export function isInBin(root: GroupNode, id: string): boolean {
  return (trailTo(root, id) ?? []).some((group) => group.kind === "recycle_bin");
}

export interface GroupOption {
  readonly node: GroupNode;
  /** "Cuentas › Trabajo › Clientes". */
  readonly label: string;
}

/** Every group in tree order with its path, for a destination `<select>`. `skip` drops a group and its subtree. */
export function groupOptions(root: GroupNode, skip: (node: GroupNode) => boolean): GroupOption[] {
  const out: GroupOption[] = [];
  function walk(node: GroupNode, trail: readonly string[]) {
    if (skip(node)) return;
    const here = [...trail, shown(node.name, "(sin nombre)")];
    out.push({ node, label: here.join(" › ") });
    for (const child of node.children) walk(child, here);
  }
  walk(root, []);
  return out;
}

/** Entries outside the recycle bin; `entry_count` is per group, so the sum works whether it is direct or not. */
export function liveEntryCount(node: GroupNode): number {
  if (node.kind === "recycle_bin") return 0;
  return node.entryCount + node.children.reduce((sum, child) => sum + liveEntryCount(child), 0);
}

/** Seconds as "4:12", for the lock countdown. */
export function formatCountdown(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${String(Math.floor(whole / 60))}:${String(whole % 60).padStart(2, "0")}`;
}
