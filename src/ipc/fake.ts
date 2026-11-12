import type { Transport } from "./client";

/**
 * In-memory stand-in for the core, at the wire level: it answers with raw snake_case payloads and rejects with
 * `{ code, message }`, so tests go through the same `safeParse` edge as the app. It follows the rules of spec 001 that
 * the screens depend on (validation order, recycle bin, patch semantics); any command it does not know rejects with a
 * plain `Error` (the client maps it to `Inesperado`).
 * Test-only: the data it holds are fixtures, never real secrets.
 */

export interface FakeVaultFile {
  readonly path: string;
  readonly name: string;
  readonly password?: string;
  readonly keyFilePath?: string;
  /** RF-14: a KDBX 3.x file opens read-only. */
  readonly readOnly?: boolean;
  /** RF-17: a KDF weaker than Argon2id with 64 MiB. */
  readonly weakKdf?: boolean;
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

export interface FakeCustomField {
  readonly name: string;
  readonly protected: boolean;
  readonly value: string;
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
  readonly created?: string;
  /** The password; only `entry_reveal` and `entry_copy` ever touch it. */
  readonly password?: string;
  /** Values of the standard fields that travel as `null` ("Title", "UserName", "URL", "Notes"). */
  readonly secrets?: Readonly<Record<string, string>>;
  readonly customFields?: readonly FakeCustomField[];
  readonly attachments?: readonly { readonly name: string; readonly size: number }[];
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
  /** RF-73: seconds until the clipboard is cleared (20 by default). */
  readonly clipboardSeconds?: number;
  /** RF-29: minutes of inactivity before locking (5 by default). */
  readonly autoLockMinutes?: number;
}

export interface FakeCall {
  readonly command: string;
  readonly args: Readonly<Record<string, unknown>> | undefined;
}

/** What the core holds for an entry, for assertions that the window cannot see. */
export interface FakeEntrySnapshot {
  readonly groupId: string;
  readonly title: string | null;
  readonly username: string | null;
  readonly url: string | null;
  readonly notes: string | null;
  readonly tags: readonly string[];
  readonly password: string;
  readonly modified: string;
  /** Versions in the history (RF-26). */
  readonly history: number;
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
  /** The entry as the core holds it, or `undefined` if it does not exist (deleted for good). */
  entry(id: string): FakeEntrySnapshot | undefined;
  /** Names of the groups from the root down to `id`, or `undefined` if it does not exist. */
  groupTrail(id: string): string[] | undefined;
  /** What `entry_copy` last put on the clipboard (inside the core, never through the window). */
  clipboard(): string | null;
  /** URLs that `entry_open_url` handed to the browser. */
  readonly openedUrls: readonly string[];
}

const CORE_MESSAGE = "core text that the window must not show";
const DEFAULT_ROOT: FakeGroup = { id: "group-root", name: "Raíz", kind: "root" };
const DEFAULT_MODIFIED = "2026-11-11T10:00:00Z";

/** KeePass keys of the standard fields. */
const STANDARD_FIELDS = ["Title", "UserName", "Password", "URL", "Notes"] as const;

/** Spec 001, RF-40. */
export const LIMITS = {
  title: 255,
  username: 255,
  url: 2048,
  password: 1024,
  notes: 100_000,
  tags: 50,
  tag: 50,
  groupName: 255,
} as const;

/** Tauri rejects `invoke` with the serialized `{ code, message }` object, not an `Error`; the fake does the same. */
function failWith(code: string, extra: { readonly field?: string; readonly limit?: number } = {}): never {
  // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors the shape Tauri rejects with.
  throw { code, message: CORE_MESSAGE, ...extra };
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

/** RF-80: unknown keys or a mistyped one reject the whole call. */
function onlyKeys(args: Record<string, unknown>, allowed: readonly string[]) {
  if (Object.keys(args).some((key) => !allowed.includes(key))) failWith("ArgumentoInvalido");
}

function requiredString(value: unknown): string {
  if (typeof value !== "string") failWith("ArgumentoInvalido");
  return value;
}

function stringOrUndefined(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  return requiredString(value);
}

function tagsOrUndefined(value: unknown): string[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || !value.every((tag): tag is string => typeof tag === "string")) {
    failWith("ArgumentoInvalido");
  }
  return [...value];
}

function booleanArg(value: unknown): boolean {
  if (typeof value !== "boolean") failWith("ArgumentoInvalido");
  return value;
}

/** Spec 001, "Plegar": lowercase, NFKD and no combining marks. */
export function fold(text: string): string {
  return text.toLowerCase().normalize("NFKD").replace(/\p{M}/gu, "");
}

/** Spec 001, "Recortar": drop `White_Space` characters at both ends. */
export function trimSpace(text: string): string {
  return text.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");
}

/** Spec 001, "Carácter": a code point. */
export function charCount(text: string): number {
  return Array.from(text).length;
}

/** Spec 001, "Carácter no permitido": control characters except tab and line feed. */
// eslint-disable-next-line no-control-regex -- the rule is precisely about control characters.
const FORBIDDEN = /[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/u;
export function hasForbidden(text: string): boolean {
  return FORBIDDEN.test(text);
}

function byTitle(a: { title: string | null; id: string }, b: { title: string | null; id: string }): number {
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

interface StoredEntry {
  id: string;
  groupId: string;
  title: string | null;
  username: string | null;
  url: string | null;
  notes: string | null;
  tags: string[];
  modified: string;
  created: string;
  password: string;
  secrets: Record<string, string>;
  customFields: FakeCustomField[];
  attachments: { name: string; size: number }[];
  history: number;
}

interface StoredGroup {
  id: string;
  name: string;
  kind: FakeGroupKind;
  children: StoredGroup[];
}

function storeGroup(group: FakeGroup): StoredGroup {
  return {
    id: group.id,
    name: group.name,
    kind: group.kind ?? "normal",
    children: (group.children ?? []).map(storeGroup),
  };
}

function storeEntry(entry: FakeEntry): StoredEntry {
  return {
    id: entry.id,
    groupId: entry.groupId,
    title: entry.title,
    // `undefined` = not filled in; `null` = protected, and it travels as `null`.
    username: entry.username === undefined ? "" : entry.username,
    url: entry.url === undefined ? "" : entry.url,
    notes: entry.notes === undefined ? "" : entry.notes,
    tags: [...(entry.tags ?? [])],
    modified: entry.modified ?? DEFAULT_MODIFIED,
    created: entry.created ?? entry.modified ?? DEFAULT_MODIFIED,
    password: entry.password ?? "",
    secrets: { ...(entry.secrets ?? {}) },
    customFields: [...(entry.customFields ?? [])],
    attachments: (entry.attachments ?? []).map((attachment) => ({ ...attachment })),
    history: 0,
  };
}

function summaryOf(entry: StoredEntry) {
  return {
    id: entry.id,
    title: entry.title,
    username: entry.username,
    url: entry.url,
    tags: [...entry.tags],
    modified: entry.modified,
  };
}

/** Names of the protected fields that have a value (plan, `EntryDetail.protected_fields`). */
function protectedFieldsOf(entry: StoredEntry): string[] {
  const names: string[] = [];
  if (entry.password !== "") names.push("Password");
  if (entry.title === null) names.push("Title");
  if (entry.username === null) names.push("UserName");
  if (entry.url === null) names.push("URL");
  if (entry.notes === null) names.push("Notes");
  for (const field of entry.customFields) if (field.protected) names.push(field.name);
  return names;
}

function detailOf(entry: StoredEntry) {
  return {
    ...summaryOf(entry),
    group_id: entry.groupId,
    notes: entry.notes,
    protected_fields: protectedFieldsOf(entry),
    // A protected custom field never carries its value (RF-43).
    custom_fields: entry.customFields.map((field) =>
      field.protected ? { name: field.name, protected: true } : { ...field },
    ),
    attachments: entry.attachments.map((attachment) => ({ ...attachment })),
    created: entry.created,
  };
}

/** The value a field holds in the core, or `undefined` if the entry has no such field. */
function fieldValue(entry: StoredEntry, field: string): string | undefined {
  switch (field) {
    case "Password":
      return entry.password;
    case "Title":
      return entry.title ?? entry.secrets.Title ?? "";
    case "UserName":
      return entry.username ?? entry.secrets.UserName ?? "";
    case "URL":
      return entry.url ?? entry.secrets.URL ?? "";
    case "Notes":
      return entry.notes ?? entry.secrets.Notes ?? "";
    default:
      return entry.customFields.find((custom) => custom.name === field)?.value;
  }
}

const CHARSETS = {
  lower: "abcdefghijklmnopqrstuvwxyz",
  upper: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
  digits: "0123456789",
  symbols: "!@#$%^&*()-_=+[]{};:,.<>?/~|",
} as const;
const LOOKALIKES = new Set(["0", "O", "o", "1", "l", "I", "|"]);

/** Test stand-in for RF-65 to RF-69 (the real one, without bias, lives in the core). */
function generate(length: number, sets: readonly string[]): string {
  const pick = (chars: string): string => {
    const buffer = new Uint32Array(1);
    crypto.getRandomValues(buffer);
    return chars[(buffer[0] ?? 0) % chars.length] ?? "";
  };
  const all = sets.join("");
  for (;;) {
    let password = "";
    for (let index = 0; index < length; index += 1) password += pick(all);
    if (sets.every((set) => Array.from(password).some((char) => set.includes(char)))) return password;
  }
}

export function createFakeBackend(options: FakeOptions = {}): FakeBackend {
  const vaults = [...(options.vaults ?? [])];
  const files = new Set(options.files ?? []);
  let recent = (options.recent ?? []).map((item) => ({ ...item }));
  let status: WireStatus = options.status ?? { state: "closed" };
  const picks = new Map<string, (string | null)[]>(
    Object.entries(options.picks ?? {}).map(([command, answers]) => [command, [...answers]]),
  );
  const root = storeGroup(options.groups ?? DEFAULT_ROOT);
  let entries = (options.entries ?? []).map(storeEntry);
  const strength = options.strength ?? defaultStrength;
  /** RF-82: kept outside the vault, readable while it is closed or locked. */
  let settings = { auto_lock_minutes: options.autoLockMinutes ?? 5, clipboard_seconds: options.clipboardSeconds ?? 20 };
  const calls: FakeCall[] = [];
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  const onceResponses = new Map<string, { kind: "resolve" | "reject"; value: unknown }>();
  const holds = new Map<string, Promise<void>>();
  const openedUrls: string[] = [];
  let clipboard: string | null = null;
  let sequence = 0;

  function nextId(prefix: string): string {
    sequence += 1;
    return `${prefix}-new-${String(sequence)}`;
  }

  function now(): string {
    sequence += 1;
    return new Date(Date.UTC(2026, 10, 11, 12, 0, sequence)).toISOString().replace(".000Z", "Z");
  }

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
    clipboard = null;
    emit("vault-locked");
  }

  function remember(path: string) {
    recent = [{ path, exists: true }, ...recent.filter((item) => item.path !== path)].slice(0, 5);
  }

  function openStatus(path: string, name: string, recovery: WireRecovery, file?: FakeVaultFile): WireStatus {
    return {
      state: "open",
      path,
      name,
      read_only: file?.readOnly ?? false,
      weak_kdf: file?.weakKdf ?? false,
      save_pending: false,
      recovery,
    };
  }

  type OpenWire = Extract<WireStatus, { state: "open" }>;

  function openNow(): OpenWire {
    if (status.state !== "open") failWith("Bloqueada");
    return status;
  }

  /** RF-23, RF-24: retry, save as and reload leave nothing pending; the test drives the events with `emit`. */
  function settle(next: Partial<OpenWire> = {}): WireStatus {
    status = { ...openNow(), save_pending: false, ...next };
    return status;
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
    const opened = openStatus(path, vault.name, recovery, vault);
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

  function requireWritable() {
    if (status.state === "open" && status.read_only) failWith("SoloLectura");
  }

  function groupWire(group: StoredGroup): unknown {
    return {
      id: group.id,
      name: group.name,
      kind: group.kind,
      entry_count: entries.filter((entry) => entry.groupId === group.id).length,
      children: group.children.map(groupWire),
    };
  }

  /** Groups from the root down to `id`, or `undefined` if the group does not exist. */
  function pathTo(id: string, group: StoredGroup = root, trail: readonly StoredGroup[] = []): StoredGroup[] | undefined {
    const here = [...trail, group];
    if (group.id === id) return here;
    for (const child of group.children) {
      const found = pathTo(id, child, here);
      if (found !== undefined) return found;
    }
    return undefined;
  }

  function requireGroup(id: unknown): StoredGroup[] {
    const trail = pathTo(requiredString(id));
    if (trail === undefined) failWith("NoEncontrado");
    return trail;
  }

  function requireEntry(id: unknown): StoredEntry {
    const wanted = requiredString(id);
    const entry = entries.find((candidate) => candidate.id === wanted);
    if (entry === undefined) failWith("NoEncontrado");
    return entry;
  }

  function inBin(trail: readonly StoredGroup[]): boolean {
    return trail.some((group) => group.kind === "recycle_bin");
  }

  function descendants(group: StoredGroup): StoredGroup[] {
    return group.children.flatMap((child) => [child, ...descendants(child)]);
  }

  function containsBin(group: StoredGroup): boolean {
    return group.kind === "recycle_bin" || descendants(group).some((child) => child.kind === "recycle_bin");
  }

  function findBin(): StoredGroup | undefined {
    return [root, ...descendants(root)].find((group) => group.kind === "recycle_bin");
  }

  /** RF-47: the recycle bin is created on the first delete if the vault has none. */
  function ensureBin(): StoredGroup {
    const existing = findBin();
    if (existing !== undefined) return existing;
    const bin: StoredGroup = { id: nextId("group-bin"), name: "Papelera de reciclaje", kind: "recycle_bin", children: [] };
    root.children.push(bin);
    return bin;
  }

  function detach(trail: readonly StoredGroup[]) {
    const group = trail.at(-1);
    const parent = trail.at(-2);
    if (group === undefined || parent === undefined) return;
    parent.children = parent.children.filter((child) => child.id !== group.id);
  }

  function removeForGood(group: StoredGroup) {
    const ids = new Set([group, ...descendants(group)].map((item) => item.id));
    entries = entries.filter((entry) => !ids.has(entry.groupId));
  }

  /** RF-52: the name of a group, trimmed. */
  function groupName(value: unknown): string {
    const name = trimSpace(requiredString(value));
    if (name === "") failWith("NombreVacio");
    if (charCount(name) > LIMITS.groupName) failWith("CampoMuyLargo", { field: "name", limit: LIMITS.groupName });
    if (hasForbidden(name)) failWith("CaracterNoPermitido", { field: "name" });
    return name;
  }

  /**
   * RF-57: the root cannot be moved or deleted (renaming it is allowed); the recycle bin cannot be moved, renamed or
   * deleted; a group that holds the bin cannot be deleted (it can be renamed or moved).
   */
  function requireMovable(trail: readonly StoredGroup[], action: "move" | "rename" | "delete") {
    const group = trail.at(-1);
    if (group === undefined) failWith("NoEncontrado");
    if (group.kind === "recycle_bin") failWith("GrupoProtegido");
    if (group.kind === "root" && action !== "rename") failWith("GrupoProtegido");
    if (action === "delete" && containsBin(group)) failWith("GrupoProtegido");
  }

  interface EntryInput {
    title?: string | undefined;
    username?: string | undefined;
    password?: string | undefined;
    url?: string | undefined;
    notes?: string | undefined;
    tags?: string[] | undefined;
  }

  const ENTRY_KEYS = ["title", "username", "password", "url", "notes", "tags"] as const;

  function readEntryInput(args: Record<string, unknown>): EntryInput {
    return {
      title: stringOrUndefined(args.title),
      username: stringOrUndefined(args.username),
      password: stringOrUndefined(args.password),
      url: stringOrUndefined(args.url),
      notes: stringOrUndefined(args.notes),
      tags: tagsOrUndefined(args.tags),
    };
  }

  /** RF-39, RF-40, RF-91 and "Carácter no permitido", in the spec's order. */
  function validateEntryInput(input: EntryInput) {
    if (input.title !== undefined && trimSpace(input.title) === "") failWith("TituloVacio");
    // plan.md, precision 15: the error names the field (argument name) and its limit.
    const texts = [
      ["title", input.title === undefined ? undefined : trimSpace(input.title), LIMITS.title],
      ["username", input.username, LIMITS.username],
      ["password", input.password, LIMITS.password],
      ["url", input.url, LIMITS.url],
      ["notes", input.notes, LIMITS.notes],
    ] as const;
    for (const [field, value, limit] of texts) {
      if (value !== undefined && charCount(value) > limit) failWith("CampoMuyLargo", { field, limit });
    }
    if (input.tags !== undefined && input.tags.length > LIMITS.tags) {
      failWith("CampoMuyLargo", { field: "tags", limit: LIMITS.tags });
    }
    if (input.tags?.some((tag) => charCount(tag) > LIMITS.tag) === true) {
      failWith("CampoMuyLargo", { field: "tags", limit: LIMITS.tag });
    }
    if (input.tags?.some((tag) => /[;,]/u.test(tag) || trimSpace(tag) === "") === true) {
      failWith("EtiquetaInvalida", { field: "tags" });
    }
    for (const [field, value] of texts) {
      if (value !== undefined && hasForbidden(value)) failWith("CaracterNoPermitido", { field });
    }
    if (input.tags?.some(hasForbidden) === true) failWith("CaracterNoPermitido", { field: "tags" });
  }

  function entryCreate(args: Record<string, unknown>) {
    onlyKeys(args, ["group_id", ...ENTRY_KEYS]);
    const input = readEntryInput(args);
    if (input.title === undefined) failWith("ArgumentoInvalido");
    requireWritable();
    const trail = requireGroup(args.group_id);
    validateEntryInput(input);
    const stamp = now();
    const entry: StoredEntry = {
      id: nextId("entry"),
      groupId: trail.at(-1)?.id ?? root.id,
      title: trimSpace(input.title),
      username: input.username ?? "",
      url: input.url ?? "",
      notes: input.notes ?? "",
      tags: input.tags ?? [],
      modified: stamp,
      created: stamp,
      // RF-41: exactly as written.
      password: input.password ?? "",
      secrets: {},
      customFields: [],
      attachments: [],
      history: 0,
    };
    entries.push(entry);
    return detailOf(entry);
  }

  /** RF-45: a missing field is left alone; a present one is replaced; no real change touches nothing. */
  function entryUpdate(args: Record<string, unknown>) {
    onlyKeys(args, ["id", ...ENTRY_KEYS]);
    const input = readEntryInput(args);
    requireWritable();
    const entry = requireEntry(args.id);
    validateEntryInput(input);
    const next: Partial<StoredEntry> = {
      ...(input.title === undefined ? {} : { title: trimSpace(input.title) }),
      ...(input.username === undefined ? {} : { username: input.username }),
      ...(input.url === undefined ? {} : { url: input.url }),
      ...(input.notes === undefined ? {} : { notes: input.notes }),
      ...(input.tags === undefined ? {} : { tags: input.tags }),
      ...(input.password === undefined ? {} : { password: input.password }),
    };
    const changed = (Object.keys(next) as (keyof StoredEntry)[]).some(
      (key) => JSON.stringify(next[key]) !== JSON.stringify(entry[key]),
    );
    if (changed) {
      Object.assign(entry, next);
      entry.history += 1;
      entry.modified = now();
    }
    return detailOf(entry);
  }

  function entryDelete(args: Record<string, unknown>) {
    onlyKeys(args, ["id"]);
    requireWritable();
    const entry = requireEntry(args.id);
    const trail = pathTo(entry.groupId) ?? [];
    if (inBin(trail)) {
      // RF-48: for good (the real core also records it in the deleted objects).
      entries = entries.filter((candidate) => candidate.id !== entry.id);
    } else {
      entry.groupId = ensureBin().id;
    }
    return null;
  }

  function protectedOrStandard(entry: StoredEntry, field: string): string {
    const known = protectedFieldsOf(entry).includes(field) || (STANDARD_FIELDS as readonly string[]).includes(field);
    const value = fieldValue(entry, field);
    if (!known || value === undefined) failWith("NoEncontrado");
    return value;
  }

  function entryOpenUrl(args: Record<string, unknown>) {
    onlyKeys(args, ["id"]);
    const entry = requireEntry(args.id);
    const url = trimSpace(fieldValue(entry, "URL") ?? "");
    const scheme = /^([a-z][a-z0-9+.-]*):/iu.exec(url)?.[1]?.toLowerCase();
    if (scheme !== "http" && scheme !== "https") failWith("UrlNoPermitida");
    openedUrls.push(url);
    return null;
  }

  function generatePassword(args: Record<string, unknown>) {
    onlyKeys(args, ["length", "lower", "upper", "digits", "symbols", "avoid_lookalikes"]);
    const length = args.length;
    if (typeof length !== "number") failWith("ArgumentoInvalido");
    const chosen = {
      lower: booleanArg(args.lower),
      upper: booleanArg(args.upper),
      digits: booleanArg(args.digits),
      symbols: booleanArg(args.symbols),
    };
    const avoid = booleanArg(args.avoid_lookalikes);
    const sets = (Object.keys(chosen) as (keyof typeof chosen)[])
      .filter((kind) => chosen[kind])
      .map((kind) => Array.from(CHARSETS[kind]).filter((char) => !avoid || !LOOKALIKES.has(char)).join(""));
    if (!Number.isInteger(length) || length < 8 || length > 128 || sets.length === 0) failWith("ReglasImposibles");
    const password = generate(length, sets);
    return { password, score: strength(password) };
  }

  function search(query: string) {
    const words = fold(query).split(/\s+/u).filter((word) => word !== "");
    if (words.length === 0) return [];
    const hits = entries
      .map((entry) => ({ entry, trail: pathTo(entry.groupId) }))
      .filter(({ trail }) => trail !== undefined && !inBin(trail))
      .filter(({ entry }) => {
        // RF-61: a protected field (`null`) never enters the search.
        const texts = [entry.title, entry.username, entry.url, entry.notes, ...entry.tags]
          .filter((text): text is string => typeof text === "string")
          .map(fold);
        return words.every((word) => texts.some((text) => text.includes(word)));
      });
    const titleHasAll = (entry: StoredEntry) => words.every((word) => fold(entry.title ?? "").includes(word));
    hits.sort((a, b) => {
      const rank = Number(titleHasAll(b.entry)) - Number(titleHasAll(a.entry));
      return rank !== 0 ? rank : byTitle(a.entry, b.entry);
    });
    return hits.map(({ entry, trail }) => ({
      ...summaryOf(entry),
      group_path: (trail ?? []).map((group) => group.name),
    }));
  }

  function countContent(group: StoredGroup) {
    const all = [group, ...descendants(group)];
    const ids = new Set(all.map((item) => item.id));
    return { entries: entries.filter((entry) => ids.has(entry.groupId)).length, subgroups: all.length - 1 };
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
        clipboard = null;
        emit("vault-locked");
        return null;
      case "vault_close":
        status = { state: "closed" };
        clipboard = null;
        emit("vault-locked");
        return null;
      case "vault_retry_save":
      case "vault_reload":
        return settle();
      case "vault_save_as": {
        onlyKeys(args, ["path"]);
        const path = requiredString(args.path);
        openNow();
        files.add(path);
        remember(path);
        return settle({ path });
      }
      case "vault_convert": {
        onlyKeys(args, ["new_path"]);
        const path = requiredString(args.new_path);
        openNow();
        if (files.has(path) || vaults.some((vault) => vault.path === path)) failWith("ArchivoExiste");
        // RF-15: a new KDBX 4 file; the original one stays as it was.
        files.add(path);
        remember(path);
        return settle({ path, read_only: false, weak_kdf: false });
      }
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
      case "group_create": {
        requireOpen();
        onlyKeys(args, ["parent_id", "name"]);
        requireWritable();
        const parent = requireGroup(args.parent_id).at(-1);
        const name = groupName(args.name);
        parent?.children.push({ id: nextId("group"), name, kind: "normal", children: [] });
        return groupWire(root);
      }
      case "group_rename": {
        requireOpen();
        onlyKeys(args, ["id", "name"]);
        requireWritable();
        const trail = requireGroup(args.id);
        requireMovable(trail, "rename");
        const name = groupName(args.name);
        const group = trail.at(-1);
        if (group !== undefined) group.name = name;
        return groupWire(root);
      }
      case "group_move": {
        requireOpen();
        onlyKeys(args, ["id", "parent_id"]);
        requireWritable();
        const trail = requireGroup(args.id);
        const target = requireGroup(args.parent_id);
        requireMovable(trail, "move");
        const group = trail.at(-1);
        // RF-55: the same group or one of its descendants.
        if (group === undefined || target.some((step) => step.id === group.id)) failWith("MovimientoInvalido");
        detach(trail);
        target.at(-1)?.children.push(group);
        return groupWire(root);
      }
      case "group_delete_preview": {
        requireOpen();
        onlyKeys(args, ["id"]);
        const trail = requireGroup(args.id);
        requireMovable(trail, "delete");
        const group = trail.at(-1);
        return group === undefined ? { entries: 0, subgroups: 0 } : countContent(group);
      }
      case "group_delete": {
        requireOpen();
        onlyKeys(args, ["id"]);
        requireWritable();
        const trail = requireGroup(args.id);
        requireMovable(trail, "delete");
        const group = trail.at(-1);
        if (group === undefined) failWith("NoEncontrado");
        detach(trail);
        // RF-58: a group already in the bin goes for good; otherwise RF-56 moves it there with its content.
        if (inBin(trail)) removeForGood(group);
        else ensureBin().children.push(group);
        return groupWire(root);
      }
      case "recycle_bin_empty": {
        requireOpen();
        requireWritable();
        const bin = findBin();
        if (bin !== undefined) {
          for (const child of bin.children) removeForGood(child);
          bin.children = [];
          entries = entries.filter((entry) => entry.groupId !== bin.id);
        }
        return groupWire(root);
      }
      case "entry_list": {
        requireOpen();
        const groupId = optionalString(args.group_id) ?? "";
        if (pathTo(groupId) === undefined) failWith("NoEncontrado");
        return entries.filter((entry) => entry.groupId === groupId).sort(byTitle).map(summaryOf);
      }
      case "entry_get":
        requireOpen();
        onlyKeys(args, ["id"]);
        return detailOf(requireEntry(args.id));
      case "entry_create":
        requireOpen();
        return entryCreate(args);
      case "entry_update":
        requireOpen();
        return entryUpdate(args);
      case "entry_move": {
        requireOpen();
        onlyKeys(args, ["id", "group_id"]);
        requireWritable();
        const entry = requireEntry(args.id);
        const target = requireGroup(args.group_id).at(-1);
        // RF-46: no new version in the history.
        if (target !== undefined) entry.groupId = target.id;
        return null;
      }
      case "entry_delete":
        requireOpen();
        return entryDelete(args);
      case "entry_reveal": {
        requireOpen();
        onlyKeys(args, ["id", "field"]);
        const entry = requireEntry(args.id);
        const field = requiredString(args.field);
        if (!protectedFieldsOf(entry).includes(field)) failWith("NoEncontrado");
        return { value: protectedOrStandard(entry, field) };
      }
      case "entry_copy": {
        requireOpen();
        onlyKeys(args, ["id", "field"]);
        const entry = requireEntry(args.id);
        const field = requiredString(args.field);
        clipboard = protectedOrStandard(entry, field);
        // plan.md, precision 16: a field that is not protected is not cleared on its own.
        return { clears_in_seconds: protectedFieldsOf(entry).includes(field) ? settings.clipboard_seconds : null };
      }
      case "settings_get":
        return { ...settings };
      case "settings_set": {
        onlyKeys(args, ["auto_lock_minutes", "clipboard_seconds"]);
        const minutes = args.auto_lock_minutes;
        const seconds = args.clipboard_seconds;
        // RF-29: 1, 5, 15 or 30 (there is no "never"); RF-73: 10 to 120 s.
        if (typeof minutes !== "number" || ![1, 5, 15, 30].includes(minutes)) {
          failWith("ArgumentoInvalido", { field: "auto_lock_minutes" });
        }
        if (typeof seconds !== "number" || !Number.isInteger(seconds) || seconds < 10 || seconds > 120) {
          failWith("ArgumentoInvalido", { field: "clipboard_seconds" });
        }
        settings = { auto_lock_minutes: minutes, clipboard_seconds: seconds };
        return { ...settings };
      }
      case "entry_open_url":
        requireOpen();
        return entryOpenUrl(args);
      case "generate_password":
        return generatePassword(args);
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
    entry(id) {
      const entry = entries.find((candidate) => candidate.id === id);
      if (entry === undefined) return undefined;
      return {
        groupId: entry.groupId,
        title: entry.title,
        username: entry.username,
        url: entry.url,
        notes: entry.notes,
        tags: [...entry.tags],
        password: entry.password,
        modified: entry.modified,
        history: entry.history,
      };
    },
    groupTrail: (id) => pathTo(id)?.map((group) => group.name),
    clipboard: () => clipboard,
    openedUrls,
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
