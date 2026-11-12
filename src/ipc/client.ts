import { toAppError, type Result } from "./errors";
import {
  copySchema,
  deletePreviewSchema,
  entryDetailSchema,
  entryHistorySchema,
  entryListSchema,
  generatedSchema,
  groupTreeSchema,
  pickedPathSchema,
  recentListSchema,
  revealSchema,
  saveStateSchema,
  searchResultsSchema,
  settingsSchema,
  strengthSchema,
  unitSchema,
  vaultStatusSchema,
  type AutoLockMinutes,
  type DeletePreview,
  type EntryDetail,
  type EntrySummary,
  type EntryVersion,
  type Generated,
  type GroupNode,
  type RecentVault,
  type SaveState,
  type SearchResult,
  type Settings,
  type Strength,
  type VaultStatus,
} from "./schemas";
import type { z } from "./zod";

/**
 * The raw channel to the core: `invoke` and `listen` from Tauri in the app, an in-memory fake in the tests.
 * Everything that comes back through it is `unknown` until a schema says otherwise.
 */
export interface Transport {
  invoke(command: string, payload?: Record<string, unknown>): Promise<unknown>;
  listen(event: string, handler: (payload: unknown) => void): Promise<() => void>;
}

export type Unlisten = () => void;

/** Proposed contract: the plan lists `none | existing{path} | generate{path}`; this is serde's internal tagging. */
export type KeyFileChoice =
  | { readonly kind: "none" }
  | { readonly kind: "existing"; readonly path: string }
  | { readonly kind: "generate"; readonly path: string };

export interface CreateVaultInput {
  readonly path: string;
  readonly name: string;
  /** Read from the field right before the call and never stored (ADR-0002). */
  readonly password?: string;
  readonly keyFile: KeyFileChoice;
}

export interface OpenVaultInput {
  readonly path: string;
  readonly password?: string;
  readonly keyFilePath?: string;
}

export interface EntryFields {
  readonly title: string;
  readonly username: string;
  readonly password: string;
  readonly url: string;
  readonly notes: string;
  readonly tags: readonly string[];
}

/** RF-45: a missing field is not changed; a present one replaces the stored value, also with an empty one. */
export type EntryPatch = Partial<EntryFields>;

export interface GenerateOptions {
  readonly length: number;
  readonly lower: boolean;
  readonly upper: boolean;
  readonly digits: boolean;
  readonly symbols: boolean;
  readonly avoidLookalikes: boolean;
}

export interface SettingsInput {
  readonly autoLockMinutes: AutoLockMinutes;
  readonly clipboardSeconds: number;
}

/**
 * Proposed contract: the name of a field for `entry_reveal` / `entry_copy` is the one the core lists in
 * `protected_fields` or `custom_fields` (KeePass keys: "Password", "UserName", … or the custom name).
 */
export type FieldName = string;

/** One method per command of plan.md. Every response is validated with `safeParse` before it reaches the UI. */
export interface VaultClient {
  vaultStatus(): Promise<Result<VaultStatus>>;
  vaultCreate(input: CreateVaultInput): Promise<Result<VaultStatus>>;
  vaultOpen(input: OpenVaultInput): Promise<Result<VaultStatus>>;
  vaultLock(): Promise<Result<undefined>>;
  vaultClose(): Promise<Result<undefined>>;
  vaultConvert(newPath: string): Promise<Result<VaultStatus>>;
  vaultRetrySave(): Promise<Result<VaultStatus>>;
  vaultSaveAs(path: string): Promise<Result<VaultStatus>>;
  vaultReload(): Promise<Result<VaultStatus>>;

  pickVaultToOpen(): Promise<Result<string | null>>;
  pickVaultToCreate(): Promise<Result<string | null>>;
  pickKeyFile(): Promise<Result<string | null>>;
  pickNewKeyFile(): Promise<Result<string | null>>;

  recentList(): Promise<Result<RecentVault[]>>;
  recentRemove(path: string): Promise<Result<RecentVault[]>>;

  passwordStrength(password: string): Promise<Result<Strength>>;

  groupTree(): Promise<Result<GroupNode>>;
  groupCreate(parentId: string, name: string): Promise<Result<GroupNode>>;
  groupRename(id: string, name: string): Promise<Result<GroupNode>>;
  groupMove(id: string, parentId: string): Promise<Result<GroupNode>>;
  groupDeletePreview(id: string): Promise<Result<DeletePreview>>;
  groupDelete(id: string): Promise<Result<GroupNode>>;
  recycleBinEmpty(): Promise<Result<GroupNode>>;

  entryList(groupId: string): Promise<Result<EntrySummary[]>>;
  entryGet(id: string): Promise<Result<EntryDetail>>;
  entryCreate(groupId: string, fields: EntryFields): Promise<Result<EntryDetail>>;
  entryUpdate(id: string, patch: EntryPatch): Promise<Result<EntryDetail>>;
  entryMove(id: string, groupId: string): Promise<Result<undefined>>;
  entryDelete(id: string): Promise<Result<undefined>>;
  entryReveal(id: string, field: FieldName): Promise<Result<string>>;
  /** `clearsInSeconds` is `null` for a field that is not protected: the clipboard is not cleared on its own. */
  entryCopy(id: string, field: FieldName): Promise<Result<{ readonly clearsInSeconds: number | null }>>;
  entryOpenUrl(id: string): Promise<Result<undefined>>;
  entryHistory(id: string): Promise<Result<EntryVersion[]>>;

  search(query: string): Promise<Result<SearchResult[]>>;
  generatePassword(options: GenerateOptions): Promise<Result<Generated>>;
  settingsGet(): Promise<Result<Settings>>;
  settingsSet(settings: SettingsInput): Promise<Result<Settings>>;
  /** RF-29: resets the inactivity timer in the core. */
  activity(): Promise<Result<undefined>>;

  /** RF-32: the core locked the vault; the window must drop every vault datum. */
  onVaultLocked(handler: () => void): Promise<Unlisten>;
  onSaveState(handler: (state: SaveState) => void): Promise<Unlisten>;
}

/**
 * Proposed contract: a command with input takes ONE Rust parameter named `args`, a struct with
 * `#[serde(deny_unknown_fields)]` and snake_case fields (RF-80). With top-level parameters Tauri would expect camelCase
 * keys and silently ignore unknown ones. Changing this convention means changing only this function.
 */
function payloadFor(args: Record<string, unknown> | undefined): Record<string, unknown> | undefined {
  return args === undefined ? undefined : { args };
}

export function createClient(transport: Transport): VaultClient {
  async function call<S extends z.ZodType>(
    command: string,
    schema: S,
    args?: Record<string, unknown>,
  ): Promise<Result<z.output<S>>> {
    let raw: unknown;
    try {
      raw = await transport.invoke(command, payloadFor(args));
    } catch (rejection: unknown) {
      return { ok: false, error: toAppError(rejection) };
    }
    const parsed = schema.safeParse(raw);
    return parsed.success
      ? { ok: true, value: parsed.data }
      : { ok: false, error: { code: "RespuestaInvalida" } };
  }

  async function reveal(id: string, field: FieldName): Promise<Result<string>> {
    const result = await call("entry_reveal", revealSchema, { id, field });
    return result.ok ? { ok: true, value: result.value.value } : result;
  }

  return {
    vaultStatus: () => call("vault_status", vaultStatusSchema),
    vaultCreate: ({ path, name, password, keyFile }) =>
      call("vault_create", vaultStatusSchema, {
        path,
        name,
        ...(password === undefined ? {} : { password }),
        key_file: keyFile,
      }),
    vaultOpen: ({ path, password, keyFilePath }) =>
      call("vault_open", vaultStatusSchema, {
        path,
        ...(password === undefined ? {} : { password }),
        ...(keyFilePath === undefined ? {} : { key_file_path: keyFilePath }),
      }),
    vaultLock: () => call("vault_lock", unitSchema),
    vaultClose: () => call("vault_close", unitSchema),
    vaultConvert: (newPath) => call("vault_convert", vaultStatusSchema, { new_path: newPath }),
    vaultRetrySave: () => call("vault_retry_save", vaultStatusSchema),
    vaultSaveAs: (path) => call("vault_save_as", vaultStatusSchema, { path }),
    vaultReload: () => call("vault_reload", vaultStatusSchema),

    pickVaultToOpen: () => call("pick_vault_to_open", pickedPathSchema),
    pickVaultToCreate: () => call("pick_vault_to_create", pickedPathSchema),
    pickKeyFile: () => call("pick_key_file", pickedPathSchema),
    pickNewKeyFile: () => call("pick_new_key_file", pickedPathSchema),

    recentList: () => call("recent_list", recentListSchema),
    recentRemove: (path) => call("recent_remove", recentListSchema, { path }),

    passwordStrength: (password) => call("password_strength", strengthSchema, { password }),

    groupTree: () => call("group_tree", groupTreeSchema),
    groupCreate: (parentId, name) => call("group_create", groupTreeSchema, { parent_id: parentId, name }),
    groupRename: (id, name) => call("group_rename", groupTreeSchema, { id, name }),
    groupMove: (id, parentId) => call("group_move", groupTreeSchema, { id, parent_id: parentId }),
    groupDeletePreview: (id) => call("group_delete_preview", deletePreviewSchema, { id }),
    groupDelete: (id) => call("group_delete", groupTreeSchema, { id }),
    recycleBinEmpty: () => call("recycle_bin_empty", groupTreeSchema),

    entryList: (groupId) => call("entry_list", entryListSchema, { group_id: groupId }),
    entryGet: (id) => call("entry_get", entryDetailSchema, { id }),
    entryCreate: (groupId, fields) =>
      call("entry_create", entryDetailSchema, { group_id: groupId, ...fields }),
    entryUpdate: (id, patch) => call("entry_update", entryDetailSchema, { id, ...patch }),
    entryMove: (id, groupId) => call("entry_move", unitSchema, { id, group_id: groupId }),
    entryDelete: (id) => call("entry_delete", unitSchema, { id }),
    entryReveal: reveal,
    entryCopy: (id, field) => call("entry_copy", copySchema, { id, field }),
    entryOpenUrl: (id) => call("entry_open_url", unitSchema, { id }),
    entryHistory: (id) => call("entry_history", entryHistorySchema, { id }),

    search: (query) => call("search", searchResultsSchema, { query }),
    generatePassword: ({ length, lower, upper, digits, symbols, avoidLookalikes }) =>
      call("generate_password", generatedSchema, {
        length,
        lower,
        upper,
        digits,
        symbols,
        avoid_lookalikes: avoidLookalikes,
      }),
    settingsGet: () => call("settings_get", settingsSchema),
    settingsSet: ({ autoLockMinutes, clipboardSeconds }) =>
      call("settings_set", settingsSchema, {
        auto_lock_minutes: autoLockMinutes,
        clipboard_seconds: clipboardSeconds,
      }),
    activity: () => call("activity", unitSchema),

    // The payload of `vault-locked` carries nothing the window needs: the event alone means "drop everything".
    onVaultLocked: (handler) => transport.listen("vault-locked", () => { handler(); }),
    onSaveState: (handler) =>
      transport.listen("vault-save-state", (payload) => {
        const parsed = saveStateSchema.safeParse(payload);
        if (parsed.success) handler(parsed.data);
      }),
  };
}
