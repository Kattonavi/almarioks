import { describe, expect, it } from "vitest";

import type { GroupNode } from "../ipc/schemas";
import { assignLines, entryInitial, entryLine, initialOf, lineOfPath, LINES, NO_INITIAL } from "./lines";

function group(id: string, children: GroupNode[] = [], kind: GroupNode["kind"] = "normal"): GroupNode {
  return { id, name: id, entryCount: 0, kind, children };
}

describe("line colours of the groups", () => {
  it("gives the first-level groups the lines in order, by position", () => {
    const root = group("root", [group("a"), group("b"), group("c"), group("d")], "root");
    const lines = assignLines(root);
    expect(["a", "b", "c", "d"].map((id) => lines.get(id))).toEqual(["1", "2", "3", "4"]);
  });

  it("is deterministic: the same tree always gets the same lines", () => {
    const root = group("root", [group("a"), group("b", [group("b1")])], "root");
    expect([...assignLines(root)]).toEqual([...assignLines(root)]);
  });

  it("makes the root the station and keeps the recycle bin neutral without taking a position", () => {
    const root = group("root", [group("a"), group("bin", [group("old")], "recycle_bin"), group("b")], "root");
    const lines = assignLines(root);
    expect(lines.get("root")).toBe("station");
    expect(lines.get("bin")).toBe("neutral");
    expect(lines.get("old")).toBe("neutral");
    expect(lines.get("a")).toBe("1");
    expect(lines.get("b")).toBe("2");
  });

  it("lets every subgroup inherit the line of its first-level group", () => {
    const root = group("root", [group("a"), group("b", [group("b1", [group("b2")])])], "root");
    const lines = assignLines(root);
    expect(lines.get("b1")).toBe("2");
    expect(lines.get("b2")).toBe("2");
  });

  it("starts the cycle over when there are more groups than line colours", () => {
    const ids = ["a", "b", "c", "d", "e", "f"];
    const lines = assignLines(group("root", ids.map((id) => group(id)), "root"));
    expect(lines.get("e")).toBe("1");
    expect(lines.get("f")).toBe("2");
    expect(new Set(ids.map((id) => lines.get(id)))).toEqual(new Set(LINES));
  });

  it("finds the line of a search result from its path of names", () => {
    const root = group("root", [group("a"), group("b", [group("b1")]), group("bin", [], "recycle_bin")], "root");
    const lines = assignLines(root);
    expect(lineOfPath(root, lines, ["root"])).toBe("station");
    expect(lineOfPath(root, lines, ["root", "b", "b1"])).toBe("2");
    expect(lineOfPath(root, lines, ["root", "bin"])).toBe("neutral");
    expect(lineOfPath(root, lines, ["root", "gone"])).toBe("neutral");
  });

  it("takes the first letter or digit of a name as the badge initial", () => {
    expect(initialOf("trabajo")).toBe("T");
    expect(initialOf("  ñandú")).toBe("Ñ");
    expect(initialOf("(sin nombre)")).toBe("S");
    expect(initialOf("2026")).toBe("2");
    expect(initialOf("···")).toBe("");
  });

  it("RF-42: the circle of an entry takes the initial of its title, and a neutral sign when there is none to show", () => {
    expect(entryInitial("github")).toBe("G");
    expect(entryInitial("")).toBe(NO_INITIAL);
    expect(entryInitial("···")).toBe(NO_INITIAL);
    // A protected title arrives as `null`: nothing of it is drawn.
    expect(entryInitial(null)).toBe(NO_INITIAL);
  });

  it("RF-42: the circle of an entry is neutral outside a group of its own (the root, or a group that is gone)", () => {
    expect(entryLine("2")).toBe("2");
    expect(entryLine("neutral")).toBe("neutral");
    expect(entryLine("station")).toBe("neutral");
    expect(entryLine(undefined)).toBe("neutral");
  });
});
