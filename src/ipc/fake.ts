import type { Transport } from "./client";

/**
 * In-memory stand-in for the core, at the wire level: it answers with raw snake_case payloads and rejects with
 * `{ code, message }`, so tests go through the same `safeParse` edge as the app. It only knows the commands the
 * screens use so far; any other command rejects with a plain `Error` (the client maps it to `Inesperado`).
 * Test-only: the data it holds are fixtures, never real secrets.
 */

export interface FakeVaultFile {
  readonly path: string;
  readonly name: string;
  readonly password?: string;
  readonly keyFilePath?: string;
}

type WireRecovery = { kind: "saved"; path: string } | { kind: "lost" } | null;

type WireStatus =
  | { state: "closed" }
  | { state: "locked"; path: string; recovery: WireRecovery }
  | { state: "deriving"; path: string }
  | {
      state: "open";
      path: string;
      name: string;
      read_only: boolean;
      weak_kdf: boolean;
      save_pending: boolean;
      recovery: WireRecovery;
    };

type PickCommand = "pick_vault_to_open" | "pick_vault_to_create" | "pick_key_file" | "pick_new_key_file";

export interface FakeOptions {
  readonly vaults?: readonly FakeVaultFile[];
  readonly recent?: readonly { readonly path: string; readonly exists: boolean }[];
  readonly status?: WireStatus;
  /** Answers of the native file dialogs, in order; `null` = the person cancelled. */
  readonly picks?: Partial<Record<PickCommand, readonly (string | null)[]>>;
}

export interface FakeCall {
  readonly command: string;
  readonly args: Readonly<Record<string, unknown>> | undefined;
}

export interface FakeBackend {
  readonly transport: Transport;
  readonly calls: readonly FakeCall[];
  count(command: string): number;
  /** What the core would send as an event. */
  emit(event: string, payload?: unknown): void;
  /** The next `vault_open` stays deriving until the returned function is called. */
  holdUnlock(): () => void;
  /** The next call to `command` resolves with this raw value, unvalidated. */
  respondOnce(command: string, raw: unknown): void;
  /** The next call to `command` rejects with this value. */
  rejectOnce(command: string, rejection: unknown): void;
  /** The core locks on its own (timer, session): state `locked` and the `vault-locked` event. */
  lockFromCore(recovery?: WireRecovery): void;
}

const CORE_MESSAGE = "core text that the window must not show";

/** Tauri rejects `invoke` with the serialized `{ code, message }` object, not an `Error`; the fake does the same. */
function failWith(code: string): never {
  // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors the shape Tauri rejects with.
  throw { code, message: CORE_MESSAGE };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function argsOf(payload: Record<string, unknown> | undefined): Record<string, unknown> | undefined {
  const args = payload?.args;
  return isRecord(args) ? args : undefined;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

export function createFakeBackend(options: FakeOptions = {}): FakeBackend {
  const vaults = [...(options.vaults ?? [])];
  let recent = (options.recent ?? []).map((item) => ({ ...item }));
  let status: WireStatus = options.status ?? { state: "closed" };
  const picks = new Map<string, (string | null)[]>(
    Object.entries(options.picks ?? {}).map(([command, answers]) => [command, [...answers]]),
  );
  const calls: FakeCall[] = [];
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  const onceResponses = new Map<string, { kind: "resolve" | "reject"; value: unknown }>();
  let gate: Promise<void> | null = null;

  /** Read through a call so the compiler does not narrow `status` across the `await` above. */
  function stateNow(): WireStatus["state"] {
    return status.state;
  }

  function emit(event: string, payload?: unknown) {
    for (const handler of [...(listeners.get(event) ?? [])]) handler(payload);
  }

  function lockFromCore(recovery: WireRecovery = null) {
    if (status.state !== "closed") status = { state: "locked", path: status.path, recovery };
    emit("vault-locked");
  }

  async function vaultOpen(args: Record<string, unknown>): Promise<WireStatus> {
    if (status.state === "deriving") failWith("Ocupada");
    const path = optionalString(args.path) ?? "";
    const password = optionalString(args.password);
    const keyFilePath = optionalString(args.key_file_path);
    const before = status;
    const recovery = before.state === "locked" ? before.recovery : null;
    status = { state: "deriving", path };
    const waiting = gate;
    gate = null;
    if (waiting !== null) await waiting;

    const vault = vaults.find((candidate) => candidate.path === path);
    if (vault === undefined) {
      status = before;
      failWith("ArchivoNoEncontrado");
    }
    if (vault.password !== password || vault.keyFilePath !== keyFilePath) {
      status = before;
      failWith("LlaveIncorrecta");
    }
    const opened: WireStatus = {
      state: "open",
      path,
      name: vault.name,
      read_only: false,
      weak_kdf: false,
      save_pending: false,
      recovery,
    };
    // Locked while deriving: the core stays locked, but the late answer still reaches the window (a race to survive).
    if (stateNow() !== "deriving") return opened;
    status = opened;
    recent = [{ path, exists: true }, ...recent.filter((item) => item.path !== path)].slice(0, 5);
    return status;
  }

  async function handle(command: string, args: Record<string, unknown>): Promise<unknown> {
    switch (command) {
      case "vault_status":
        return status;
      case "vault_open":
        return vaultOpen(args);
      case "vault_lock":
        if (status.state === "open") status = { state: "locked", path: status.path, recovery: null };
        emit("vault-locked");
        return null;
      case "vault_close":
        status = { state: "closed" };
        emit("vault-locked");
        return null;
      case "recent_list":
        return recent;
      case "recent_remove":
        recent = recent.filter((item) => item.path !== args.path);
        return recent;
      case "pick_vault_to_open":
      case "pick_vault_to_create":
      case "pick_key_file":
      case "pick_new_key_file":
        return picks.get(command)?.shift() ?? null;
      case "activity":
        return null;
      default:
        throw new Error(`fake core: ${command} is not faked`);
    }
  }

  const transport: Transport = {
    async invoke(command, payload) {
      const args = argsOf(payload);
      calls.push({ command, args });
      const once = onceResponses.get(command);
      if (once !== undefined) {
        onceResponses.delete(command);
        if (once.kind === "reject") throw once.value;
        return once.value;
      }
      return handle(command, args ?? {});
    },
    listen(event, handler) {
      const set = listeners.get(event) ?? new Set();
      set.add(handler);
      listeners.set(event, set);
      return Promise.resolve(() => {
        set.delete(handler);
      });
    },
  };

  return {
    transport,
    calls,
    count: (command) => calls.filter((call) => call.command === command).length,
    emit,
    holdUnlock() {
      let release!: () => void;
      gate = new Promise<void>((resolve) => {
        release = resolve;
      });
      return release;
    },
    respondOnce(command, raw) {
      onceResponses.set(command, { kind: "resolve", value: raw });
    },
    rejectOnce(command, rejection) {
      onceResponses.set(command, { kind: "reject", value: rejection });
    },
    lockFromCore,
  };
}
