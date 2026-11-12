import { useEffect, useId, useRef, type ReactNode } from "react";

import { WarningIcon } from "../ui/icons";

interface DialogProps {
  readonly title: string;
  /** `danger`: the dialog confirms something that cannot be undone, and says so with the warning sign. */
  readonly tone?: "danger";
  /** Escape or a close button: the owner unmounts the dialog. */
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/**
 * A native modal `<dialog>`: mounted only while open, so nothing of it stays in the DOM after closing. It traps focus
 * (the browser does) and gives it back to the control that opened it when it goes away.
 */
export function Dialog({ title, tone, onClose, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    const dialog = ref.current;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog?.showModal();
    // The browser focuses the first control. Where that would be the destructive action, the dialog marks the way
    // out instead, so Enter never destroys anything by default.
    dialog?.querySelector<HTMLElement>("[data-initial-focus]")?.focus();
    return () => {
      if (dialog?.open === true) dialog.close();
      if (opener?.isConnected === true) opener.focus();
    };
  }, []);

  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      data-tone={tone}
      onCancel={(event) => {
        // Escape: React decides, so the dialog is unmounted instead of left closed in the DOM.
        event.preventDefault();
        onClose();
      }}
    >
      <h3 id={titleId}>
        {tone === "danger" && <WarningIcon />}
        {title}
      </h3>
      {children}
    </dialog>
  );
}
