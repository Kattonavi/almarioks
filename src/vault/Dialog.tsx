import { useEffect, useId, useRef, type ReactNode } from "react";

import { CloseIcon, WarningIcon } from "../ui/icons";

interface DialogProps {
  readonly title: string;
  /** `danger`: the dialog confirms something that cannot be undone, and says so with the warning sign. */
  readonly tone?: "danger";
  /**
   * `form`: a wide sheet for a long form (`entry-dialog.css`). The head (title, `lead` and a close button) stays
   * still; the children are a form whose body scrolls and whose foot stays still too.
   */
  readonly layout?: "form";
  /** With `layout="form"`: the line under the title. */
  readonly lead?: ReactNode;
  /** Escape or a close button: the owner unmounts the dialog. */
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/**
 * A native modal `<dialog>`: mounted only while open, so nothing of it stays in the DOM after closing. It traps focus
 * (the browser does) and gives it back to the control that opened it when it goes away. A click on the backdrop does
 * not close it.
 */
export function Dialog({ title, tone, layout, lead, onClose, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
  /** This component is closing the dialog itself (it is going away): that `close` is not the browser's doing. */
  const closing = useRef(false);
  const titleId = useId();

  useEffect(() => {
    const dialog = ref.current;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    closing.current = false;
    dialog?.showModal();
    // The browser focuses the first control. Where that would be the destructive action, the dialog marks the way
    // out instead, so Enter never destroys anything by default.
    dialog?.querySelector<HTMLElement>("[data-initial-focus]")?.focus();
    return () => {
      closing.current = true;
      if (dialog?.open === true) dialog.close();
      if (opener?.isConnected === true) opener.focus();
    };
  }, []);

  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      data-tone={tone}
      data-layout={layout}
      onCancel={(event) => {
        // Escape: React decides, so the dialog is unmounted instead of left closed in the DOM.
        event.preventDefault();
        onClose();
      }}
      onClose={() => {
        // The browser closed it on its own (Chromium lets a repeated Escape through even when the first one was
        // cancelled). It is still mounted, so it opens again with everything in it and the owner is asked, the same
        // as with Escape: what was written is never lost in silence.
        const dialog = ref.current;
        if (closing.current || dialog === null || dialog.open) return;
        dialog.showModal();
        onClose();
      }}
    >
      {layout === "form" ? (
        <header className="dialog-head">
          <div>
            <h2 id={titleId}>{title}</h2>
            {lead}
          </div>
          <button type="button" className="dialog-close" aria-label="Cerrar" onClick={onClose}>
            <CloseIcon />
          </button>
        </header>
      ) : (
        <h3 id={titleId}>
          {tone === "danger" && <WarningIcon />}
          {title}
        </h3>
      )}
      {children}
    </dialog>
  );
}
