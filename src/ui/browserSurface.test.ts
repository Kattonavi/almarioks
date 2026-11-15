import { fireEvent } from "@testing-library/dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeBrowserSurface, isBrowserKey, type KeyPress } from "./browserSurface";

function press(key: string, modifiers: Partial<Omit<KeyPress, "key">> = {}): KeyPress {
  return { key, ctrlKey: false, shiftKey: false, altKey: false, ...modifiers };
}

const ctrl = { ctrlKey: true };
const ctrlShift = { ctrlKey: true, shiftKey: true };

/** The keys the release build answered to, or that open a way out of the page (validacion-app.md, section 5). */
const BROWSER_KEYS: readonly [name: string, press: KeyPress][] = [
  ["F5", press("F5")],
  ["Ctrl+R", press("r", ctrl)],
  ["Ctrl+Shift+R", press("R", ctrlShift)],
  ["Ctrl+F5", press("F5", ctrl)],
  ["Ctrl+P", press("p", ctrl)],
  ["Ctrl+Shift+P", press("P", ctrlShift)],
  ["Ctrl+S", press("s", ctrl)],
  ["F3", press("F3")],
  ["Ctrl+G", press("g", ctrl)],
  ["Ctrl+J", press("j", ctrl)],
  ["Ctrl+U", press("u", ctrl)],
  ["F12", press("F12")],
  ["Ctrl+Shift+I", press("I", ctrlShift)],
  ["Ctrl+Shift+J", press("J", ctrlShift)],
  ["Ctrl+Shift+C", press("C", ctrlShift)],
];

/** What the app and its text fields need: none of it may be cancelled. */
const APP_KEYS: readonly [name: string, press: KeyPress][] = [
  ["Ctrl+K", press("k", ctrl)],
  ["Ctrl+V", press("v", ctrl)],
  ["Ctrl+C", press("c", ctrl)],
  ["Ctrl+X", press("x", ctrl)],
  ["Ctrl+Z", press("z", ctrl)],
  ["Ctrl+A", press("a", ctrl)],
  ["Tab", press("Tab")],
  ["Shift+Tab", press("Tab", { shiftKey: true })],
  ["Escape", press("Escape")],
  ["Enter", press("Enter")],
  ["ArrowDown", press("ArrowDown")],
  ["ArrowUp", press("ArrowUp")],
  ["p", press("p")],
  ["Shift+R", press("R", { shiftKey: true })],
  ["AltGr+E (€)", press("€", { ctrlKey: true, altKey: true })],
  ["AltGr+P", press("p", { ctrlKey: true, altKey: true })],
  ["Alt+F4", press("F4", { altKey: true })],
];

describe("isBrowserKey", () => {
  it.each(BROWSER_KEYS)("RF-77: %s belongs to the browser", (_name, key) => {
    expect(isBrowserKey(key)).toBe(true);
  });

  it.each(APP_KEYS)("RF-77: %s is left alone", (_name, key) => {
    expect(isBrowserKey(key)).toBe(false);
  });

  it("RF-77: the letter counts in either case, as Caps Lock or Shift leave it", () => {
    expect(isBrowserKey(press("P", ctrl))).toBe(true);
    expect(isBrowserKey(press("p", ctrlShift))).toBe(true);
  });

  it("RF-76: Ctrl+Shift+C opens the developer tools and Ctrl+C copies: only the first is the browser's", () => {
    expect(isBrowserKey(press("C", ctrlShift))).toBe(true);
    expect(isBrowserKey(press("c", ctrl))).toBe(false);
  });

  it("RF-77: Ctrl+F is the browser's too, so «Find on page» never opens with the vault locked", () => {
    expect(isBrowserKey(press("f", ctrl))).toBe(true);
  });
});

describe("closeBrowserSurface", () => {
  let open: () => void;
  let field: HTMLInputElement;

  beforeEach(() => {
    open = closeBrowserSurface();
    field = document.createElement("input");
    document.body.append(field);
  });

  afterEach(() => {
    open();
    field.remove();
  });

  /** `fireEvent` answers `false` when the default action was cancelled. */
  function cancelled(target: Element, key: KeyPress): boolean {
    return !fireEvent.keyDown(target, key);
  }

  it("RF-77: the context menu is cancelled everywhere: on the page, on text and on a field", () => {
    const text = document.createElement("p");
    document.body.append(text);
    for (const target of [document.body, text, field]) {
      expect(fireEvent.contextMenu(target)).toBe(false);
    }
    text.remove();
  });

  it.each(BROWSER_KEYS)("RF-77: %s is cancelled, also inside a field", (_name, key) => {
    expect(cancelled(document.body, key)).toBe(true);
    expect(cancelled(field, key)).toBe(true);
  });

  it.each(APP_KEYS)("RF-77: %s is not cancelled", (_name, key) => {
    expect(cancelled(document.body, key)).toBe(false);
    expect(cancelled(field, key)).toBe(false);
  });

  it("RF-77: Ctrl+K, Ctrl+V and Tab are not cancelled", () => {
    for (const key of [press("k", ctrl), press("v", ctrl), press("Tab")]) {
      expect(cancelled(field, key)).toBe(false);
    }
  });

  it("RF-77: a cancelled key still reaches the app, so Ctrl+F goes on to the search of the vault", () => {
    const heard = vi.fn();
    window.addEventListener("keydown", heard);
    expect(cancelled(document.body, press("f", ctrl))).toBe(true);
    expect(cancelled(document.body, press("F5"))).toBe(true);
    window.removeEventListener("keydown", heard);
    expect(heard).toHaveBeenCalledTimes(2);
  });

  it("RF-77: it is cancelled before a listener of the page could stop the event", () => {
    function swallow(event: Event) {
      event.stopPropagation();
    }
    field.addEventListener("keydown", swallow);
    field.addEventListener("contextmenu", swallow);
    expect(cancelled(field, press("p", ctrl))).toBe(true);
    expect(fireEvent.contextMenu(field)).toBe(false);
  });

  it("RF-77: undoing it gives the page back", () => {
    open();
    expect(cancelled(document.body, press("p", ctrl))).toBe(false);
    expect(fireEvent.contextMenu(document.body)).toBe(true);
  });

  describe("a protected value on screen", () => {
    function show(): { secret: HTMLElement; title: HTMLElement } {
      document.body.innerHTML = '<p id="title">Banco</p><span class="secret">fake-shown-value</span>';
      const secret = document.querySelector<HTMLElement>(".secret");
      const title = document.querySelector<HTMLElement>("#title");
      if (secret === null || title === null) throw new Error("fixture");
      return { secret, title };
    }

    function select(node: Node) {
      const selection = document.getSelection();
      if (selection === null) throw new Error("no selection");
      selection.removeAllRanges();
      selection.selectAllChildren(node);
    }

    it("RF-72: copying or cutting a selection that includes a shown value is cancelled", () => {
      const { secret } = show();
      select(secret);
      expect(fireEvent.copy(secret)).toBe(false);
      expect(fireEvent.cut(secret)).toBe(false);
    });

    it("RF-72: selecting the whole page does not take the value out either", () => {
      show();
      select(document.body);
      expect(fireEvent.copy(document.body)).toBe(false);
    });

    it("RF-72: dragging a shown value out is cancelled, selected or not", () => {
      const { secret } = show();
      document.getSelection()?.removeAllRanges();
      expect(fireEvent.dragStart(secret)).toBe(false);
    });

    it("RF-72: copying any other text of the page still works", () => {
      const { title } = show();
      select(title);
      expect(fireEvent.copy(title)).toBe(true);
      document.getSelection()?.removeAllRanges();
      expect(fireEvent.copy(title)).toBe(true);
    });

    it("RF-72: undoing it gives the copy back", () => {
      const { secret } = show();
      select(secret);
      open();
      expect(fireEvent.copy(secret)).toBe(true);
    });
  });
});
