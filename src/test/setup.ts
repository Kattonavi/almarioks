/**
 * jsdom has `HTMLDialogElement` but not `showModal` / `close`. WebView2 (Chromium) has both; this stand-in only
 * toggles `open`, which is what the tests read. It is loaded by Vitest alone, never by the app.
 */
HTMLDialogElement.prototype.showModal = function showModal(this: HTMLDialogElement) {
  this.open = true;
};
HTMLDialogElement.prototype.close = function close(this: HTMLDialogElement) {
  this.open = false;
  this.dispatchEvent(new Event("close"));
};
