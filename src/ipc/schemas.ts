import { z } from "./zod";

/**
 * Wire schemas for the responses of the core (docs/specs/001-boveda/plan.md, "Contrato entre la ventana y el núcleo").
 * Responses arrive in `snake_case` and leave this module as camelCase domain types. Unknown extra keys are stripped,
 * so the core can add fields without breaking the window; a missing or mistyped field fails the parse.
 * Optional values are accepted as either a missing key or `null` (serde's default for `Option`).
 */

const nonEmpty = z.string().min(1);
const id = nonEmpty;
const timestamp = nonEmpty;

// ---------- Vault status ----------

/** Proposed contract: the plan names `recovery` but not its shape (RF-34). */
const recoveryWire = z
  .discriminatedUnion("kind", [
    z.object({ kind: z.literal("saved"), path: nonEmpty }),
    z.object({ kind: z.literal("lost") }),
  ])
  .nullish()
  .transform((value) => value ?? null);

export type Recovery = { readonly kind: "saved"; readonly path: string } | { readonly kind: "lost" };

/** Proposed contract: the plan names `state` but not its values; "deriving" is the RF-18 state. */
export const vaultStatusSchema = z
  .discriminatedUnion("state", [
    z.object({ state: z.literal("closed") }),
    z.object({ state: z.literal("locked"), path: nonEmpty, recovery: recoveryWire }),
    z.object({ state: z.literal("deriving"), path: nonEmpty }),
    z.object({
      state: z.literal("open"),
      path: nonEmpty,
      name: z.string().nullish(),
      read_only: z.boolean(),
      weak_kdf: z.boolean(),
      save_pending: z.boolean(),
      recovery: recoveryWire,
    }),
  ])
  .transform((wire): VaultStatus => {
    switch (wire.state) {
      case "closed":
        return { state: "closed" };
      case "locked":
        return { state: "locked", path: wire.path, recovery: wire.recovery };
      case "deriving":
        return { state: "deriving", path: wire.path };
      case "open":
        return {
          state: "open",
          path: wire.path,
          name: wire.name ?? "",
          readOnly: wire.read_only,
          weakKdf: wire.weak_kdf,
          savePending: wire.save_pending,
          recovery: wire.recovery,
        };
    }
  });

export type VaultStatus =
  | { readonly state: "closed" }
  | { readonly state: "locked"; readonly path: string; readonly recovery: Recovery | null }
  | { readonly state: "deriving"; readonly path: string }
  | {
      readonly state: "open";
      readonly path: string;
      /** Database name stored in the vault; may be empty. */
      readonly name: string;
      readonly readOnly: boolean;
      readonly weakKdf: boolean;
      readonly savePending: boolean;
      readonly recovery: Recovery | null;
    };

// ---------- Small results ----------

/** `()` in Rust serializes as `null`; the Tauri mock returns `undefined`. */
export const unitSchema = z.union([z.null(), z.undefined()]).transform(() => undefined);

/** `pick_*`: the path chosen in the native dialog, or nothing if it was cancelled. */
export const pickedPathSchema = nonEmpty.nullable();

export const recentListSchema = z.array(z.object({ path: nonEmpty, exists: z.boolean() }));
export type RecentVault = z.output<typeof recentListSchema>[number];

const score = z.int().min(0).max(4);
export const strengthSchema = z.object({ score });
export type Strength = z.output<typeof strengthSchema>;

// ---------- Groups ----------

export type GroupKind = "root" | "normal" | "recycle_bin";

interface GroupNodeWire {
  readonly id: string;
  readonly name: string;
  readonly entry_count: number;
  readonly kind: GroupKind;
  readonly children: readonly GroupNodeWire[];
}

export interface GroupNode {
  readonly id: string;
  readonly name: string;
  readonly entryCount: number;
  readonly kind: GroupKind;
  readonly children: readonly GroupNode[];
}

const groupNodeWire: z.ZodType<GroupNodeWire> = z.lazy(() =>
  z.object({
    id,
    name: z.string(),
    entry_count: z.int().min(0),
    kind: z.enum(["root", "normal", "recycle_bin"]),
    children: z.array(groupNodeWire),
  }),
);

function toGroupNode(wire: GroupNodeWire): GroupNode {
  return {
    id: wire.id,
    name: wire.name,
    entryCount: wire.entry_count,
    kind: wire.kind,
    children: wire.children.map(toGroupNode),
  };
}

export const groupTreeSchema = groupNodeWire.transform(toGroupNode);

export const deletePreviewSchema = z.object({ entries: z.int().min(0), subgroups: z.int().min(0) });
export type DeletePreview = z.output<typeof deletePreviewSchema>;

// ---------- Entries ----------

/** A protected standard field travels as `null` and the window shows "(protegido)". */
const maybeProtected = z.string().nullable();

const entrySummaryShape = {
  id,
  title: maybeProtected,
  username: maybeProtected,
  url: maybeProtected,
  tags: z.array(z.string()),
  modified: timestamp,
};

export const entrySummarySchema = z.object(entrySummaryShape);
export type EntrySummary = z.output<typeof entrySummarySchema>;
export const entryListSchema = z.array(entrySummarySchema);

export const entryDetailSchema = z
  .object({
    ...entrySummaryShape,
    group_id: id,
    notes: maybeProtected,
    protected_fields: z.array(z.string()),
    custom_fields: z.array(
      z.object({ name: z.string(), protected: z.boolean(), value: z.string().nullish() }),
    ),
    attachments: z.array(z.object({ name: z.string(), size: z.int().min(0) })),
    created: timestamp,
  })
  .transform((wire) => ({
    id: wire.id,
    title: wire.title,
    username: wire.username,
    url: wire.url,
    tags: wire.tags,
    modified: wire.modified,
    groupId: wire.group_id,
    notes: wire.notes,
    protectedFields: wire.protected_fields,
    customFields: wire.custom_fields.map((field) => ({
      name: field.name,
      protected: field.protected,
      // A protected custom field never brings its value, whatever the core sends.
      value: field.protected ? null : (field.value ?? null),
    })),
    attachments: wire.attachments,
    created: wire.created,
  }));
export type EntryDetail = z.output<typeof entryDetailSchema>;

/** Proposed contract: "EntrySummary + ruta del grupo" as `group_path`, the group names from the root down. */
export const searchResultsSchema = z.array(
  z
    .object({ ...entrySummaryShape, group_path: z.array(z.string()) })
    .transform(({ group_path, ...summary }) => ({ ...summary, groupPath: group_path })),
);
export type SearchResult = z.output<typeof searchResultsSchema>[number];

/** Proposed contract (recortable RF-85): each version with its index and the same limits as `entry_get`. */
export const entryHistorySchema = z.array(
  z.object({
    index: z.int().min(0),
    modified: timestamp,
    title: maybeProtected,
    username: maybeProtected,
    url: maybeProtected,
    tags: z.array(z.string()),
    notes: maybeProtected,
  }),
);
export type EntryVersion = z.output<typeof entryHistorySchema>[number];

export const revealSchema = z.object({ value: z.string() });

export const copySchema = z
  .object({ clears_in_seconds: z.int().min(1) })
  .transform((wire) => ({ clearsInSeconds: wire.clears_in_seconds }));

// ---------- Generator and settings ----------

export const generatedSchema = z.object({ password: z.string(), score });
export type Generated = z.output<typeof generatedSchema>;

export const AUTO_LOCK_MINUTES = [1, 5, 15, 30] as const;
export type AutoLockMinutes = (typeof AUTO_LOCK_MINUTES)[number];

export const settingsSchema = z
  .object({
    auto_lock_minutes: z.literal(AUTO_LOCK_MINUTES),
    clipboard_seconds: z.int().min(10).max(120),
  })
  .transform((wire) => ({
    autoLockMinutes: wire.auto_lock_minutes,
    clipboardSeconds: wire.clipboard_seconds,
  }));
export type Settings = z.output<typeof settingsSchema>;

// ---------- Events ----------

export const saveStateSchema = z.enum(["saved", "failed", "external-change"]);
export type SaveState = z.output<typeof saveStateSchema>;
