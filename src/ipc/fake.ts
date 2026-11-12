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

export type FakeGroupKind = "root" | "normal" | "recycle_bin";

export interface FakeGroup {
  readonly id: string;
  readonly name: string;
  readonly kind?: FakeGroupKind;
  readonly children?: readonly FakeGroup[];
}

/** `null` in a standard field = the vault marks it protected: it never travels and never enters the search. */
export interface FakeEntry {
  readonly id: string;
  readonly groupId: string;
  readonly title: string | null;
  readonly username?: string | null;
  readonly url?: string | null;
  readonly notes?: string | null;
  readonly tags?: readonly string[];
  readonly modified?: string;
}

export interface FakeOptions {
  readonly vaults?: readonly FakeVaultFile[];
  /** Other files that already exist on disk (e.g. a key file), for `ArchivoExiste`. */
  readonly files?: readonly string[];
  readonly recent?: readonly { readonly path: string; readonly exists: boolean }[];
  readonly status?: WireStatus;
  /** Answers of the native file dialogs, in order; `null` = the person cancelled. */
  readonly picks?: Partial<Record<PickCommand, readonly (string | null)[]>>;
  /** Groups of the open vault; by default an empty root group. */
  readonly groups?: FakeGroup;
  readonly entries?: readonly FakeEntry[];
  /** Stand-in for zxcvbn; the default is a crude estimate by length and character classes. */
  readonly strength?: (password: string) => number;
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
  /** The next call to `command` waits until the returned function is called (`vault_create` stays deriving). */
  hold(command: string): () => void;
  /** The next call to `command` resolves with this raw value, unvalidated. */
  respondOnce(command: string, raw: unknown): void;
  /** The next call to `command` rejects with this value. */
  rejectOnce(command: string, rejection: unknown): void;
  /** The core locks on its own (timer, session): state `locked` and the `vault-locked` event. */
  lockFromCore(recovery?: WireRecovery): void;
}

const CORE_MESSAGE = "core text that the window must not show";
const DEFAULT_ROOT: FakeGroup = { id: "group-root", name: "Raíz", kind: "root" };
const DEFAULT_MODIFIED = "2026-11-11T10:00:00Z";

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

/** Spec 001, "Plegar": lowercase, NFKD and no combining marks. */
export function fold(text: string): string {
  return text.toLowerCase().normalize("NFKD").replace(/\p{M}/gu, "");
}

function byTitle(a: FakeEntry, b: FakeEntry): number {
  const ta = fold(a.title ?? "");
  const tb = fold(b.title ?? "");
  if (ta !== tb) return ta < tb ? -1 : 1;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

function defaultStrength(password: string): number {
  if (password.length < 8) return 0;
  const classes = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^a-zA-Z0-9]/].filter((re) => re.test(password)).length;
  return Math.min(4, classes);
}

function summaryOf(entry: FakeEntry) {
  return {
    id: entry.id,
    title: entry.title,
    username: entry.username ?? "",
    url: entry.url ?? "",
    tags: [...(entry.tags ?? [])],
    modified: entry.modified ?? DEFAULT_MODIFIED,
  };
}

export function createFakeBackend(options: FakeOptions = {}): FakeBackend {
  const vaults = [...(options.vaults ?? [])];
  const files = new Set(options.files ?? []);
  let recent = (options.recent ?? []).map((item) => ({ ...item }));
  let status: WireStatus = options.status ?? { state: "closed" };
  const picks = new Map<string, (string | null)[]>(
    Object.entries(options.picks ?? {}).map(([command, answers]) => [command, [...answers]]),
  );
  const root = options.groups ?? DEFAULT_ROOT;
  const entries = [...(options.entries ?? [])];
  const strength = options.strength ?? defaultStrength;
  const calls: FakeCall[] = [];
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  const onceResponses = new Map<string, { kind: "resolve" | "reject"; value: unknown }>();
  const holds = new Map<string, Promise<void>>();

  /** Read through a call so the compiler does not narrow `status` across the `await` above. */
  function stateNow(): WireStatus["state"] {
    return status.state;
  }

  function hold(command: string): () => void {
    let release!: () => void;
    holds.set(
      command,
      new Promise<void>((resolve) => {
        release = resolve;
      }),
    );
    return release;
  }

  async function waitHold(command: string) {
    const waiting = holds.get(command);
    holds.delete(command);
    if (waiting !== undefined) await waiting;
  }

  function emit(event: string, payload?: unknown) {
    for (const handler of [...(listeners.get(event) ?? [])]) handler(payload);
  }

  function lockFromCore(recovery: WireRecovery = null) {
    if (status.state !== "closed") status = { state: "locked", path: status.path, recovery };
    emit("vault-locked");
  }

  function remember(path: string) {
    recent = [{ path, exists: true }, ...recent.filter((item) => item.path !== path)].slice(0, 5);
  }

  function openStatus(path: string, name: string, recovery: WireRecovery): WireStatus {
    return { state: "open", path, name, read_only: false, weak_kdf: false, save_pending: false, recovery };
  }

  async function vaultOpen(args: Record<string, unknown>): Promise<WireStatus> {
    if (status.state === "deriving") failWith("Ocupada");
    const path = optionalString(args.path) ?? "";
    const password = optionalString(args.password);
    const keyFilePath = optionalString(args.key_file_path);
    const before = status;
    const recovery = before.state === "locked" ? before.recovery : null;
    status = { state: "deriving", path };
    await waitHold("vault_open");

    const vault = vaults.find((candidate) => candidate.path === path);
    if (vault === undefined) {
      status = before;
      failWith("ArchivoNoEncontrado");
    }
    if (vault.password !== password || vault.keyFilePath !== keyFilePath) {
      status = before;
      failWith("LlaveIncorrecta");
    }
    const opened = openStatus(path, vault.name, recovery);
    // Locked while deriving: the core stays locked, but the late answer still reaches the window (a race to survive).
    if (stateNow() !== "deriving") return opened;
    status = opened;
    remember(path);
    return status;
  }

  async function vaultCreate(args: Record<string, unknown>): Promise<WireStatus> {
    if (status.state === "deriving") failWith("Ocupada");
    const path = optionalString(args.path) ?? "";
    const name = optionalString(args.name) ?? "";
    const password = optionalString(args.password);
    const keyFile = isRecord(args.key_file) ? args.key_file : {};
    const kind = optionalString(keyFile.kind);
    const keyPath = optionalString(keyFile.path);
    if (kind !== "none" && kind !== "existing" && kind !== "generate") failWith("ArgumentoInvalido");
    if ((password === undefined || password === "") && kind === "none") failWith("LlaveVacia");
    if (files.has(path) || vaults.some((vault) => vault.path === path)) failWith("ArchivoExiste");
    if (kind === "generate" && keyPath !== undefined && files.has(keyPath)) failWith("ArchivoExiste");
    if (kind === "existing" && (keyPath === undefined || !files.has(keyPath))) failWith("ArchivoLlaveNoEncontrado");

    const before = status;
    status = { state: "deriving", path };
    await waitHold("vault_create");
    if (stateNow() !== "deriving") {
      status = before;
      failWith("Bloqueada");
    }
    vaults.push({
      path,
      name,
      ...(password === undefined ? {} : { password }),
      ...(kind === "none" || keyPath === undefined ? {} : { keyFilePath: keyPath }),
    });
    files.add(path);
    if (kind === "generate" && keyPath !== undefined) files.add(keyPath);
    status = openStatus(path, name, null);
    remember(path);
    return status;
  }

  function requireOpen() {
    if (status.state !== "open") failWith("Bloqueada");
  }

  function groupWire(group: FakeGroup): unknown {
    return {
      id: group.id,
      name: group.name,
      kind: group.kind ?? "normal",
      entry_count: entries.filter((entry) => entry.groupId === group.id).length,
      children: (group.children ?? []).map(groupWire),
    };
  }

  /** Path of names from the root down to `id`, or `undefined` if the group does not exist. */
  function pathTo(id: string, group: FakeGroup = root, trail: readonly FakeGroup[] = []): FakeGroup[] | undefined {
    const here = [...trail, group];
    if (group.id === id) return here;
    for (const child of group.children ?? []) {
      const found = pathTo(id, child, here);
      if (found !== undefined) return found;
    }
    return undefined;
  }

  function search(query: string) {
    const words = fold(query).split(/\s+/u).filter((word) => word !== "");
    if (words.length === 0) return [];
    const hits = entries
      .map((entry) => ({ entry, trail: pathTo(entry.groupId) }))
      .filter(({ trail }) => trail !== undefined && !trail.some((group) => group.kind === "recycle_bin"))
      .filter(({ entry }) => {
        // RF-61: a protected field (`null`) never enters the search.
        const texts = [entry.title, entry.username, entry.url, entry.notes, ...(entry.tags ?? [])]
          .filter((text): text is string => typeof text === "string")
          .map(fold);
        return words.every((word) => texts.some((text) => text.includes(word)));
      });
    const titleHasAll = (entry: FakeEntry) => words.every((word) => fold(entry.title ?? "").includes(word));
    hits.sort((a, b) => {
      const rank = Number(titleHasAll(b.entry)) - Number(titleHasAll(a.entry));
      return rank !== 0 ? rank : byTitle(a.entry, b.entry);
    });
    return hits.map(({ entry, trail }) => ({
      ...summaryOf(entry),
      group_path: (trail ?? []).map((group) => group.name),
    }));
  }

  async function handle(command: string, args: Record<string, unknown>): Promise<unknown> {
    switch (command) {
      case "vault_status":
        return status;
      case "vault_open":
        return vaultOpen(args);
      case "vault_create":
        return vaultCreate(args);
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
      case "password_strength":
        return { score: strength(optionalString(args.password) ?? "") };
      case "group_tree":
        requireOpen();
        return groupWire(root);
      case "entry_list": {
        requireOpen();
        const groupId = optionalString(args.group_id) ?? "";
        if (pathTo(groupId) === undefined) failWith("NoEncontrado");
        return entries.filter((entry) => entry.groupId === groupId).sort(byTitle).map(summaryOf);
      }
      case "search":
        requireOpen();
        return search(optionalString(args.query) ?? "");
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
      // The derivations wait inside, after the core reports them as running.
      if (command !== "vault_open" && command !== "vault_create") await waitHold(command);
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
    holdUnlock: () => hold("vault_open"),
    hold,
    respondOnce(command, raw) {
      onceResponses.set(command, { kind: "resolve", value: raw });
    },
    rejectOnce(command, rejection) {
      onceResponses.set(command, { kind: "reject", value: rejection });
    },
    lockFromCore,
  };
}

// ---------- Sample vault (spec 001, example 2, plus groups, a protected field and the recycle bin) ----------

export const SAMPLE_GROUPS: FakeGroup = {
  id: "group-root",
  name: "Cuentas",
  kind: "root",
  children: [
    { id: "group-personal", name: "Personal" },
    { id: "group-work", name: "Trabajo", children: [{ id: "group-clients", name: "Clientes" }] },
    { id: "group-archive", name: "Archivo" },
    { id: "group-bin", name: "Papelera de reciclaje", kind: "recycle_bin" },
  ],
};

export const SAMPLE_ENTRIES: readonly FakeEntry[] = [
  {
    id: "entry-song",
    groupId: "group-personal",
    title: "Canción favorita",
    username: "ana",
    url: "https://musica.example",
    tags: ["música"],
    modified: "2026-11-02T09:30:00Z",
  },
  {
    id: "entry-bank",
    groupId: "group-personal",
    title: "Banco",
    username: "cliente-01",
    url: "https://banco.example",
    notes: "cancion de cuna",
    tags: ["finanzas"],
  },
  { id: "entry-years", groupId: "group-personal", title: "Años", username: "", url: "" },
  {
    id: "entry-github",
    groupId: "group-work",
    title: "GitHub",
    username: "ana@example.com",
    url: "https://github.com/login",
    tags: ["código", "trabajo"],
  },
  { id: "entry-vpn", groupId: "group-work", title: "VPN", username: null, url: "vpn.example" },
  { id: "entry-banana", groupId: "group-clients", title: "Banana", username: "compras" },
  { id: "entry-old", groupId: "group-bin", title: "Canción borrada", username: "ana" },
];
