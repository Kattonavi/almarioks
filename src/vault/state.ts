import { assertNever } from "../assertNever";
import type { AppError } from "../ipc/errors";
import type { Recovery, VaultStatus } from "../ipc/schemas";

/** A vault file the window refers to: its path is metadata, never a secret. */
export interface VaultRef {
  readonly path: string;
  readonly fileName: string;
}

export function fileNameOf(path: string): string {
  return path.split(/[\\/]/).filter((part) => part !== "").at(-1) ?? path;
}

export function vaultRef(path: string): VaultRef {
  return { path, fileName: fileNameOf(path) };
}

/** What the window shows while the core has no vault (P1, the P2 placeholder, or P3 for a file just chosen). */
export type ClosedView =
  | { readonly kind: "welcome" }
  | { readonly kind: "create" }
  | { readonly kind: "unlock"; readonly vault: VaultRef; readonly error: AppError | null };

/**
 * The window's state. Only `open` holds data of the vault, and only its name: entries and groups are other screens.
 * `loading` is also the state right after a lock: nothing of the vault is mounted until the core says otherwise.
 */
export type AppState =
  | { readonly kind: "loading" }
  | { readonly kind: "unavailable"; readonly error: AppError }
  | { readonly kind: "closed"; readonly view: ClosedView }
  | {
      readonly kind: "locked";
      readonly vault: VaultRef;
      readonly recovery: Recovery | null;
      readonly error: AppError | null;
    }
  | {
      readonly kind: "deriving";
      readonly vault: VaultRef;
      /** `request`: this window asked to unlock; `status`: the core reported a derivation already running. */
      readonly origin: "request" | "status";
      /** Where an unlock that fails goes back to. */
      readonly back: "closed" | "locked";
      readonly recovery: Recovery | null;
    }
  | {
      readonly kind: "open";
      readonly vault: VaultRef;
      readonly name: string;
      readonly readOnly: boolean;
      readonly recovery: Recovery | null;
    };

export type AppAction =
  | { readonly type: "status"; readonly status: VaultStatus }
  | { readonly type: "status-failed"; readonly error: AppError }
  | { readonly type: "vault-locked" }
  | { readonly type: "vault-chosen"; readonly path: string }
  | { readonly type: "create-requested" }
  | { readonly type: "back-to-welcome" }
  | { readonly type: "unlock-started" }
  | { readonly type: "unlock-failed"; readonly error: AppError };

export const initialState: AppState = { kind: "loading" };

function fromStatus(status: VaultStatus): AppState {
  switch (status.state) {
    case "closed":
      return { kind: "closed", view: { kind: "welcome" } };
    case "locked":
      return { kind: "locked", vault: vaultRef(status.path), recovery: status.recovery, error: null };
    case "deriving":
      return {
        kind: "deriving",
        vault: vaultRef(status.path),
        origin: "status",
        back: "locked",
        recovery: null,
      };
    case "open":
      return {
        kind: "open",
        vault: vaultRef(status.path),
        name: status.name,
        readOnly: status.readOnly,
        recovery: status.recovery,
      };
    default:
      return assertNever(status);
  }
}

export function reducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "status":
      return fromStatus(action.status);
    case "status-failed":
      return { kind: "unavailable", error: action.error };
    case "vault-locked":
      // RF-32: drop everything at once, whatever was on screen, and wait for the core's status.
      return { kind: "loading" };
    case "vault-chosen":
      return state.kind === "closed"
        ? { kind: "closed", view: { kind: "unlock", vault: vaultRef(action.path), error: null } }
        : state;
    case "create-requested":
      return state.kind === "closed" ? { kind: "closed", view: { kind: "create" } } : state;
    case "back-to-welcome":
      return state.kind === "closed" ? { kind: "closed", view: { kind: "welcome" } } : state;
    case "unlock-started":
      if (state.kind === "locked") {
        return { kind: "deriving", vault: state.vault, origin: "request", back: "locked", recovery: state.recovery };
      }
      if (state.kind === "closed" && state.view.kind === "unlock") {
        return { kind: "deriving", vault: state.view.vault, origin: "request", back: "closed", recovery: null };
      }
      return state;
    case "unlock-failed":
      if (state.kind !== "deriving") return state;
      return state.back === "locked"
        ? { kind: "locked", vault: state.vault, recovery: state.recovery, error: action.error }
        : { kind: "closed", view: { kind: "unlock", vault: state.vault, error: action.error } };
    default:
      return assertNever(action);
  }
}
