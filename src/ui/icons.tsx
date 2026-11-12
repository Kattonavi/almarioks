import type { ReactNode } from "react";

/**
 * The app's own icons: one 1.8 stroke in `currentColor` on a 24 × 24 grid. They are decoration next to a text label
 * (or to a visually hidden one), so every icon is hidden from assistive technology.
 */
function Icon({ children }: { readonly children: ReactNode }) {
  return (
    <svg
      className="icon"
      viewBox="0 0 24 24"
      width="24"
      height="24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

export function SearchIcon() {
  return (
    <Icon>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-4-4" />
    </Icon>
  );
}

export function CopyIcon() {
  return (
    <Icon>
      <rect x="9" y="9" width="11" height="11" rx="1.5" />
      <path d="M5 15V5h10" />
    </Icon>
  );
}

export function ShowIcon() {
  return (
    <Icon>
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" />
      <circle cx="12" cy="12" r="3" />
    </Icon>
  );
}

export function HideIcon() {
  return (
    <Icon>
      <path d="M10 5.2A9.3 9.3 0 0 1 12 5c6 0 10 7 10 7a17.6 17.6 0 0 1-3.1 3.9" />
      <path d="M6.6 6.7C3.7 8.6 2 12 2 12s4 7 10 7a9.4 9.4 0 0 0 4.6-1.2" />
      <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" />
      <path d="m4 4 16 16" />
    </Icon>
  );
}

export function LockIcon() {
  return (
    <Icon>
      <rect x="5" y="11" width="14" height="9" rx="1.5" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </Icon>
  );
}

export function PlusIcon() {
  return (
    <Icon>
      <path d="M12 5v14M5 12h14" />
    </Icon>
  );
}

export function EditIcon() {
  return (
    <Icon>
      <path d="M4 20h4L19 9l-4-4L4 16v4Z" />
      <path d="m13 7 4 4" />
    </Icon>
  );
}

export function MoveIcon() {
  return (
    <Icon>
      <path d="M4 12h12" />
      <path d="m12 7 5 5-5 5" />
      <path d="M20.5 5v14" />
    </Icon>
  );
}

export function TrashIcon() {
  return (
    <Icon>
      <path d="M4 7h16" />
      <path d="M9 7V4h6v3" />
      <path d="m6 7 1 13h10l1-13" />
      <path d="M10 11v5M14 11v5" />
    </Icon>
  );
}

export function SettingsIcon() {
  return (
    <Icon>
      <path d="M4 7h9M19 7h1M4 17h1M11 17h9" />
      <circle cx="16" cy="7" r="3" />
      <circle cx="8" cy="17" r="3" />
    </Icon>
  );
}

export function CloseIcon() {
  return (
    <Icon>
      <path d="m6 6 12 12M18 6 6 18" />
    </Icon>
  );
}

export function OpenLinkIcon() {
  return (
    <Icon>
      <path d="M14 4h6v6" />
      <path d="m20 4-9 9" />
      <path d="M10 6H6a1.5 1.5 0 0 0-1.5 1.5v10A1.5 1.5 0 0 0 6 19h10a1.5 1.5 0 0 0 1.5-1.5V14" />
    </Icon>
  );
}

export function GenerateIcon() {
  return (
    <Icon>
      <path d="M19.5 12a7.5 7.5 0 0 1-13.4 4.6" />
      <path d="M4.5 12a7.5 7.5 0 0 1 13.4-4.6" />
      <path d="M18 3v4.5h-4.5" />
      <path d="M6 21v-4.5h4.5" />
    </Icon>
  );
}

export function WarningIcon() {
  return (
    <Icon>
      <path d="M12 4 2.5 20h19L12 4Z" />
      <path d="M12 10v4.5" />
      <path d="M12 17.4v.1" />
    </Icon>
  );
}

export function CheckIcon() {
  return (
    <Icon>
      <path d="m5 12.5 4.5 4.5L19 7.5" />
    </Icon>
  );
}
