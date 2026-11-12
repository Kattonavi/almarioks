import type { EntryFields, VaultClient } from "../ipc/client";
import type { AppError, Result } from "../ipc/errors";
import type { EntryDetail } from "../ipc/schemas";
import { checkEntry, entryFieldOf, parseTags, type FieldProblem } from "./rules";

/**
 * The draft of the entry dialog without its screen: what it holds, what travels to the core and how a core error
 * finds its field. No React here.
 */
export type EntryTextField = "title" | "username" | "url" | "notes";

export interface EntryValues {
  readonly title: string;
  readonly username: string;
  readonly url: string;
  readonly notes: string;
  /** As written: one tag per line. */
  readonly tags: string;
}

/** Reads a secret field and empties it at once: the value lives only in the caller's stack (ADR-0002). */
export function take(field: HTMLInputElement | null): string {
  const value = field?.value ?? "";
  if (field !== null) field.value = "";
  return value;
}

export function initialValues(entry: EntryDetail | null): EntryValues {
  if (entry === null) return { title: "", username: "", url: "", notes: "", tags: "" };
  // A protected field (`null`) is never preloaded: like the password, it is sent only if something is written.
  return {
    title: entry.title ?? "",
    username: entry.username ?? "",
    url: entry.url ?? "",
    notes: entry.notes ?? "",
    tags: entry.tags.join("\n"),
  };
}

/**
 * Which core error belongs to which field when the core rejects what the window let through. The core names the field
 * and the limit (plan.md, precision 15); the window's own deduction is the fallback when it does not.
 */
export function fieldOf(error: AppError, input: Partial<EntryFields>): FieldProblem | null {
  const local = (): FieldProblem | null => {
    const found = checkEntry(input);
    return found !== null && found.code === error.code ? found : null;
  };
  switch (error.code) {
    case "TituloVacio":
      return { field: "title", code: "TituloVacio" };
    case "EtiquetaInvalida":
      return { field: entryFieldOf(error.field) ?? "tags", code: "EtiquetaInvalida" };
    case "CampoMuyLargo": {
      const field = entryFieldOf(error.field);
      if (field === null) return local();
      return { field, code: "CampoMuyLargo", ...(error.limit === undefined ? {} : { limit: error.limit }) };
    }
    case "CaracterNoPermitido": {
      const field = entryFieldOf(error.field);
      return field === null ? local() : { field, code: "CaracterNoPermitido" };
    }
    default:
      return null;
  }
}

/** What travels: everything on create; on edit, only what changed, and the password only if one was written (RF-45). */
export function buildInput(values: EntryValues, editing: EntryDetail | null, password: string): Partial<EntryFields> {
  const tags = parseTags(values.tags);
  if (editing === null) return { ...values, tags, password };
  const changed = (field: EntryTextField) => {
    const before = editing[field];
    return before === null ? values[field] !== "" : values[field] !== before;
  };
  // Compared as written, like the other fields: tags the owner did not touch are not sent again.
  const tagsChanged = values.tags !== editing.tags.join("\n");
  return {
    ...(changed("title") ? { title: values.title } : {}),
    ...(changed("username") ? { username: values.username } : {}),
    ...(password === "" ? {} : { password }),
    ...(changed("url") ? { url: values.url } : {}),
    ...(changed("notes") ? { notes: values.notes } : {}),
    ...(tagsChanged ? { tags } : {}),
  };
}

export type EntryTarget =
  | { readonly kind: "create"; readonly groupId: string }
  | { readonly kind: "edit"; readonly id: string };

export function saveEntry(
  client: Pick<VaultClient, "entryCreate" | "entryUpdate">,
  target: EntryTarget,
  input: Partial<EntryFields>,
): Promise<Result<EntryDetail>> {
  if (target.kind === "edit") return client.entryUpdate(target.id, input);
  return client.entryCreate(target.groupId, {
    title: input.title ?? "",
    username: input.username ?? "",
    password: input.password ?? "",
    url: input.url ?? "",
    notes: input.notes ?? "",
    tags: input.tags ?? [],
  });
}
