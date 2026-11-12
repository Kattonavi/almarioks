/**
 * The window is a browser underneath (WebView2). Its context menu offers "Save as" and "Print", and its keys
 * print, reload and search the page: with an entry on screen, that takes its fields out in clear (RF-77, AMENAZAS
 * W1; defects D-9, D-10 and D-11 of `docs/specs/001-boveda/validacion-app.md`).
 *
 * The barrier that rules is native: `src-tauri/src/webview` switches both off in the webview itself. This is the
 * second one, in the page, in case a browser feature ever reaches it.
 *
 * Accepted consequence: with no context menu there is no "Paste" with the right button in the fields. Ctrl+V,
 * Ctrl+C, Ctrl+X, Ctrl+Z and Ctrl+A work as always.
 */

/** What decides whether a key press belongs to the browser. A `KeyboardEvent` fits. */
export interface KeyPress {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
}

/** Reload, find on page and developer tools, with or without Ctrl or Shift (F5, Ctrl+F5, Shift+F3...). */
const FUNCTION_KEYS: ReadonlySet<string> = new Set(["f3", "f5", "f12"]);

/**
 * With Ctrl alone: reload, print, save, find on page (F and G), downloads, view source.
 *
 * Ctrl+F is here although the vault uses it for its own search: the page only cancels what the browser would do
 * and lets the event go on, so the search of the vault still hears it. With the vault locked nobody does, and the
 * key does nothing, instead of opening "Find on page".
 */
const WITH_CTRL: ReadonlySet<string> = new Set(["r", "p", "s", "f", "g", "j", "u"]);

/** With Ctrl+Shift: hard reload, the system's print dialog, and the three ways into the developer tools. */
const WITH_CTRL_SHIFT: ReadonlySet<string> = new Set(["r", "p", "i", "j", "c"]);

/**
 * Whether the browser would do something of its own with this key press that the app does not want.
 *
 * Never true for what the app or a text field needs: Ctrl+K, Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+Z, Ctrl+A, Tab, Escape,
 * Enter, the arrows. Ctrl+Alt is not Ctrl: on Windows that is AltGr, which types characters such as "@" and "€"
 * that a master password may contain.
 */
export function isBrowserKey(press: KeyPress): boolean {
  // With Shift held the key arrives in upper case ("P", "R", "I").
  const key = press.key.toLowerCase();
  if (FUNCTION_KEYS.has(key)) return true;
  if (!press.ctrlKey || press.altKey) return false;
  return (press.shiftKey ? WITH_CTRL_SHIFT : WITH_CTRL).has(key);
}

/**
 * Cancels the context menu and the keys of the browser in the whole document, before any other listener sees
 * them (capture phase). It only cancels the default action: the event still reaches the app's own listeners.
 * Returns how to undo it.
 */
export function closeBrowserSurface(target: Document = document): () => void {
  function onContextMenu(event: Event) {
    event.preventDefault();
  }
  function onKeyDown(event: KeyboardEvent) {
    if (isBrowserKey(event)) event.preventDefault();
  }
  target.addEventListener("contextmenu", onContextMenu, { capture: true });
  target.addEventListener("keydown", onKeyDown, { capture: true });
  return () => {
    target.removeEventListener("contextmenu", onContextMenu, { capture: true });
    target.removeEventListener("keydown", onKeyDown, { capture: true });
  };
}
