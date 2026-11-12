import { useEffect, useId, useRef, type ReactNode } from "react";

import { CloseIcon, WarningIcon } from "../ui/icons";

/** The dialog draws its own head: the usual case. */
interface OwnHead {
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
}

/**
 * `entry`: the window of an entry (RF-43). Its content brings the head (title, actions and the close button), so the
 * dialog draws none and takes its name from the heading with the id `labelledBy`.
 */
interface ContentHead {
  readonly layout: "entry";
  readonly labelledBy: string;
}

type DialogProps = (OwnHead | ContentHead) & {
  /** Escape or a close button: the owner unmounts the dialog. */
  readonly onClose: () => void;
  readonly children: ReactNode;
};

/**
 * A native modal `<dialog>`: mounted only while open, so nothing of it stays in the DOM after closing. It traps focus
 * (the browser does) and gives it back to the control that opened it when it goes away. A click on the backdrop does
 * not close it.
 */
export function Dialog(props: DialogProps) {
  const { onClose, children } = props;
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
      aria-labelledby={props.layout === "entry" ? props.labelledBy : titleId}
      data-tone={props.layout === "entry" ? undefined : props.tone}
      data-layout={props.layout}
      onCancel={(event) => {
        // React carries `cancel` and `close` up its own tree: a dialog mounted inside this one (the form or an order
        // over the window of an entry) answers for itself, or one Escape would close both.
        if (event.target !== event.currentTarget) return;
        // Escape: React decides, so the dialog is unmounted instead of left closed in the DOM.
        event.preventDefault();
        onClose();
      }}
      onClose={(event) => {
        if (event.target !== event.currentTarget) return;
        // The browser closed it on its own (Chromium lets a repeated Escape through even when the first one was
        // cancelled). It is still mounted, so it opens again with everything in it and the owner is asked, the same
        // as with Escape: what was written is never lost in silence.
        const dialog = ref.current;
        if (closing.current || dialog === null || dialog.open) return;
        dialog.showModal();
        onClose();
      }}
    >
      {props.layout === "entry" ? null : props.layout === "form" ? (
        <header className="dialog-head">
          <div>
            <h2 id={titleId}>{props.title}</h2>
            {props.lead}
          </div>
          <button type="button" className="dialog-close" aria-label="Cerrar" onClick={onClose}>
            <CloseIcon />
          </button>
        </header>
      ) : (
        <h3 id={titleId}>
          {props.tone === "danger" && <WarningIcon />}
          {props.title}
        </h3>
      )}
      {children}
    </dialog>
  );
}
