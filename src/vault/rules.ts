import type { EntryFields } from "../ipc/client";
import { errorMessage } from "../ipc/errors";

/**
 * The window's copy of the rules of spec 001 that it needs to say WHICH field is wrong (RF-40 asks for the field and
 * its limit, and the core's error carries only a code). The core still decides: these checks only avoid a round trip
 * and place the message next to the field.
 */

/** RF-40 and RF-52. */
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

/** "Recortar": `White_Space` characters at both ends. */
export function trimSpace(text: string): string {
  return text.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");
}

/** "Carácter": a code point, so 255 emoji fit in a title. */
export function charCount(text: string): number {
  return Array.from(text).length;
}

// eslint-disable-next-line no-control-regex -- the rule is precisely about control characters.
const FORBIDDEN = /[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/u;

/** "Carácter no permitido": control characters except tab and line feed. */
export function hasForbidden(text: string): boolean {
  return FORBIDDEN.test(text);
}

export type EntryFieldKey = keyof EntryFields;

export type FieldProblem =
  | { readonly field: EntryFieldKey; readonly code: "TituloVacio" | "EtiquetaInvalida" | "CaracterNoPermitido" }
  /** `limit`: the one the core reported (plan.md, precision 15); without it, the window's copy of RF-40. */
  | { readonly field: EntryFieldKey; readonly code: "CampoMuyLargo"; readonly limit?: number };

/**
 * The entry field the core named in an error. It accepts the argument name (`title`, `username`…) and the KeePass key
 * (`Title`, `UserName`…); anything else is `null` and the window falls back to its own deduction.
 */
export function entryFieldOf(name: string | undefined): EntryFieldKey | null {
  switch (name?.toLowerCase()) {
    case "title":
      return "title";
    case "username":
      return "username";
    case "password":
      return "password";
    case "url":
      return "url";
    case "notes":
      return "notes";
    case "tags":
      return "tags";
    default:
      return null;
  }
}

const ORDER: readonly EntryFieldKey[] = ["title", "username", "password", "url", "notes", "tags"];

function tooLong(field: EntryFieldKey, input: Partial<EntryFields>): boolean {
  switch (field) {
    case "title":
      return input.title !== undefined && charCount(trimSpace(input.title)) > LIMITS.title;
    case "tags":
      return (
        input.tags !== undefined &&
        (input.tags.length > LIMITS.tags || input.tags.some((tag) => charCount(tag) > LIMITS.tag))
      );
    default: {
      const value = input[field];
      return value !== undefined && charCount(value) > LIMITS[field];
    }
  }
}

/** Spec order: `TituloVacio` › `CampoMuyLargo` › `EtiquetaInvalida` › `CaracterNoPermitido`. Absent fields are skipped. */
export function checkEntry(input: Partial<EntryFields>): FieldProblem | null {
  if (input.title !== undefined && trimSpace(input.title) === "") return { field: "title", code: "TituloVacio" };
  for (const field of ORDER) if (tooLong(field, input)) return { field, code: "CampoMuyLargo" };
  if (input.tags?.some((tag) => /[;,]/u.test(tag) || trimSpace(tag) === "") === true) {
    return { field: "tags", code: "EtiquetaInvalida" };
  }
  for (const field of ORDER) {
    const value = input[field];
    const texts = value === undefined ? [] : typeof value === "string" ? [value] : value;
    if (texts.some(hasForbidden)) return { field, code: "CaracterNoPermitido" };
  }
  return null;
}

/** "100 000": the thousands separated by a space, as the spec writes them. */
function count(limit: number): string {
  return String(limit).replace(/\B(?=(\d{3})+(?!\d))/gu, " ");
}

const TOO_LONG: Readonly<Record<EntryFieldKey, (limit: number) => string>> = {
  title: (limit) => `El título admite hasta ${count(limit)} caracteres. Acórtalo.`,
  username: (limit) => `El usuario admite hasta ${count(limit)} caracteres. Acórtalo.`,
  password: (limit) => `La contraseña admite hasta ${count(limit)} caracteres. Usa una más corta.`,
  url: (limit) => `La URL admite hasta ${count(limit)} caracteres. Acórtala.`,
  notes: (limit) => `Las notas admiten hasta ${count(limit)} caracteres. Acórtalas.`,
  // Two limits (how many and how long): the core's single number cannot say which, so the text names both.
  tags: () =>
    `Caben hasta ${String(LIMITS.tags)} etiquetas de ${String(LIMITS.tag)} caracteres cada una. Quita o acorta alguna.`,
};

export function problemText(problem: FieldProblem): string {
  if (problem.code !== "CampoMuyLargo") return errorMessage({ code: problem.code });
  const limit = problem.limit ?? (problem.field === "tags" ? LIMITS.tags : LIMITS[problem.field]);
  return TOO_LONG[problem.field](limit);
}

/** RF-91: one tag per line; blank lines are not tags and each tag is sent without its surrounding spaces. */
export function parseTags(text: string): string[] {
  return text
    .split("\n")
    .map(trimSpace)
    .filter((tag) => tag !== "");
}

export type GroupNameProblem = "NombreVacio" | "CampoMuyLargo" | "CaracterNoPermitido";

/** RF-52. */
export function checkGroupName(name: string): GroupNameProblem | null {
  const trimmed = trimSpace(name);
  if (trimmed === "") return "NombreVacio";
  if (charCount(trimmed) > LIMITS.groupName) return "CampoMuyLargo";
  if (hasForbidden(trimmed)) return "CaracterNoPermitido";
  return null;
}

export function groupNameText(problem: GroupNameProblem): string {
  return problem === "CampoMuyLargo"
    ? `El nombre del grupo admite hasta ${String(LIMITS.groupName)} caracteres. Acórtalo.`
    : errorMessage({ code: problem });
}
