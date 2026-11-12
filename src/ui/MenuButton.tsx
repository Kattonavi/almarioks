import { useEffect, useId, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type RefObject } from "react";

import { MoreIcon } from "./icons";

export interface MenuItem<Order extends string> {
  readonly order: Order;
  readonly label: string;
  /** Drawn in the danger role; by convention it goes last. */
  readonly danger?: boolean;
}

interface MenuButtonProps<Order extends string> {
  /** The name of the button ("Más acciones de la entrada"): its `aria-label` and the help under the pointer. */
  readonly label: string;
  /** The name of the menu ("Acciones de la entrada"). */
  readonly menuLabel: string;
  readonly items: readonly MenuItem<Order>[];
  readonly onChoose: (order: Order) => void;
  /** The wrapper: the stylesheet hangs the menu from the button inside it. */
  readonly className: string;
  readonly buttonClassName: string;
  /** For whoever has to hand the focus back to the button. */
  readonly buttonRef?: RefObject<HTMLButtonElement | null>;
}

/**
 * A "more actions" button and its menu, with the contract of the group menu of the rail (`GroupMenuButton`): the
 * button opens and closes it; choosing, Escape, a click outside or the focus leaving close it. Arrows, Home and End
 * move through the orders. On closing from the keyboard or by choosing, the focus is back on the button. The menu is
 * the shared `.menu` surface; where it hangs is the business of the wrapper's stylesheet.
 */
export function MenuButton<Order extends string>({
  label,
  menuLabel,
  items,
  onChoose,
  className,
  buttonClassName,
  buttonRef,
}: MenuButtonProps<Order>) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLSpanElement>(null);
  const ownButtonRef = useRef<HTMLButtonElement>(null);
  const button = buttonRef ?? ownButtonRef;
  const menuRef = useRef<HTMLDivElement>(null);
  const menuId = useId();

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

  function choose(order: Order) {
    // The focus goes first: a dialog that opens now gives it back to whatever has it, and this menu item is about to
    // leave the page.
    button.current?.focus();
    setOpen(false);
    onChoose(order);
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
        button.current?.focus();
        setOpen(false);
        return;
      default:
        return;
    }
  }

  return (
    <span
      ref={wrapRef}
      className={className}
      onKeyDown={onKeyDown}
      onBlur={(event) => {
        // The focus left the button and its menu (Tab, or a click somewhere else).
        if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
        setOpen(false);
      }}
    >
      <button
        ref={button}
        type="button"
        className={buttonClassName}
        aria-label={label}
        title={label}
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
        <div ref={menuRef} id={menuId} role="menu" aria-label={menuLabel} className="menu">
          {items.map((item) => (
            <button
              key={item.order}
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={item.danger === true ? "menu-danger" : undefined}
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
