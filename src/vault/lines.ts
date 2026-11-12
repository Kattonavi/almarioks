import type { GroupNode } from "../ipc/schemas";

/**
 * The ordered family of line colours (tokens `--line-1` … `--line-4`). A line only names a group: red is never one
 * (it means danger), and the recycle bin is always neutral.
 */
export const LINES = ["1", "2", "3", "4"] as const;

export type LineId = (typeof LINES)[number];

/** `station`: the root group, where every line meets. `neutral`: the recycle bin and whatever hangs from it. */
export type GroupLine = LineId | "station" | "neutral";

/**
 * The line of every group, by id. First-level groups take the lines in order, by position, and start over when
 * there are more groups than colours (the badge still tells them apart by its initial). The recycle bin does not
 * take a position. Subgroups inherit the line of their first-level group.
 */
export function assignLines(root: GroupNode): ReadonlyMap<string, GroupLine> {
  const lines = new Map<string, GroupLine>();

  function paint(node: GroupNode, line: GroupLine) {
    lines.set(node.id, line);
    for (const child of node.children) paint(child, line);
  }

  lines.set(root.id, "station");
  let position = 0;
  for (const child of root.children) {
    if (child.kind === "recycle_bin") {
      paint(child, "neutral");
      continue;
    }
    paint(child, LINES[position % LINES.length] ?? "neutral");
    position += 1;
  }
  return lines;
}

/**
 * The line of the group a search result sits in, from its path of names (root first). Two sibling groups with the
 * same name resolve to the first one; an unknown path is neutral.
 */
export function lineOfPath(
  root: GroupNode,
  lines: ReadonlyMap<string, GroupLine>,
  path: readonly string[],
): GroupLine {
  let node: GroupNode | undefined = root;
  for (const name of path.slice(1)) {
    node = node.children.find((child) => child.name === name);
    if (node === undefined) return "neutral";
  }
  return lines.get(node.id) ?? "neutral";
}

/** The letter (or digit) a badge shows: the first one of the name, in upper case; empty when the name has none. */
export function initialOf(name: string): string {
  const first = /[\p{L}\p{N}]/u.exec(name)?.[0] ?? "";
  return first.toLocaleUpperCase("es");
}

/** What the circle of an entry shows when its title has no letter or digit to offer: empty, protected, or only signs. */
export const NO_INITIAL = "#";

/** RF-42: the circle of an entry carries the initial of its own title; a protected title (`null`) gives nothing away. */
export function entryInitial(title: string | null): string {
  const initial = title === null ? "" : initialOf(title);
  return initial === "" ? NO_INITIAL : initial;
}

/**
 * The colour under the circle of an entry: the line of its group. An entry that hangs from the root itself, or from
 * a group the tree no longer has, is neutral: the white of the station is not the colour of any group.
 */
export function entryLine(line: GroupLine | undefined): GroupLine {
  return line === undefined || line === "station" ? "neutral" : line;
}
