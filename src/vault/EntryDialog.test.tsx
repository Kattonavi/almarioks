import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeEntry } from "../ipc/fake";
import { EntryDialog, type EntryDialogGroup, type EntryDialogMode } from "./EntryDialog";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const OPEN = {
  state: "open",
  path: "D:/bovedas/cuentas.kdbx",
  name: "Cuentas",
  read_only: false,
  weak_kdf: false,
  save_pending: false,
  recovery: null,
} as const;

/** Spec 001, example 8: "GitHub" has the password `tercera-clave`. */
const PASSWORD = "tercera-clave";

const GITHUB: FakeEntry = {
  id: "entry-github",
  groupId: "group-work",
  title: "GitHub",
  username: "ana@example.com",
  url: "https://github.com/login",
  notes: "cuenta del trabajo",
  tags: ["código", "trabajo"],
  password: PASSWORD,
  customFields: [
    { name: "Recuperación", protected: false, value: "pregunta: mascota" },
    { name: "PIN", protected: true, value: "4321" },
  ],
};

const ENTRIES: readonly FakeEntry[] = [
  ...SAMPLE_ENTRIES.filter((entry) => entry.id !== "entry-github"),
  GITHUB,
  // Own fields but no tags.
  {
    id: "entry-door",
    groupId: "group-work",
    title: "Puerta",
    username: "ana",
    customFields: [{ name: "Clave del portón", protected: true, value: "9876" }],
  },
];

const PERSONAL: EntryDialogGroup = { id: "group-personal", name: "Personal", line: "1" };
const WORK: EntryDialogGroup = { id: "group-work", name: "Trabajo", line: "2" };
const NEW: EntryDialogMode = { kind: "new", group: PERSONAL };

function core() {
  const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: ENTRIES });
  return { backend, client: createClient(backend.transport) };
}

function open(mode: EntryDialogMode = NEW, from = core()) {
  const onClose = vi.fn<() => void>();
  const onSaved = vi.fn<(entryId: string, changed: boolean) => void>();
  const view = render(
    <EntryDialog
      client={from.client}
      mode={mode}
      onClose={onClose}
      onSaved={(entry, changed) => {
        onSaved(entry.id, changed);
      }}
    />,
  );
  return { ...from, onClose, onSaved, view };
}

/** The entry as the core gives it, never built by hand. */
async function openEdit(id = "entry-github") {
  const from = core();
  const found = await from.client.entryGet(id);
  if (!found.ok) throw new Error(`fixture: ${id} is not in the fake core`);
  return open({ kind: "edit", entry: found.value, group: WORK }, from);
}

const lastArgs = (backend: { calls: readonly { command: string; args: unknown }[] }, command: string) =>
  backend.calls.filter((call) => call.command === command).at(-1)?.args;

const sheet = (name = "Nueva entrada") => screen.getByRole<HTMLDialogElement>("dialog", { name });
const title = () => screen.getByLabelText<HTMLInputElement>("Título");
const passwordField = () => screen.getByLabelText<HTMLInputElement>("Contraseña");
const confirmation = () => screen.queryByRole("dialog", { name: "¿Descartar los cambios?" });

function typePassword(value: string) {
  fireEvent.input(passwordField(), { target: { value } });
}

/** Escape on a modal dialog: the browser sends `cancel`; jsdom does not, so the test does. */
function pressEscape(dialog: HTMLElement) {
  fireEvent(dialog, new Event("cancel", { cancelable: true }));
}

const WAYS_OUT: readonly (readonly [string, () => void])[] = [
  [
    "Esc",
    () => {
      pressEscape(sheet());
    },
  ],
  [
    "×",
    () => {
      fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    },
  ],
  [
    "Cancelar",
    () => {
      fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    },
  ],
];

function bodyContains(value: string): boolean {
  if (document.body.innerHTML.includes(value)) return true;
  return [...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea")].some((field) =>
    field.value.includes(value),
  );
}

describe("Diálogo de entrada — abrir y cerrar", () => {
  it("RF-100: abre un diálogo modal vacío, con el foco en Título, y dice en qué grupo queda la entrada", () => {
    open();

    const dialog = sheet();
    expect(dialog.open).toBe(true);
    expect(within(dialog).getByRole("heading", { name: "Nueva entrada" })).toBeDefined();
    expect(within(dialog).getByText("En el grupo Personal")).toBeDefined();
    expect(document.activeElement).toBe(title());
    expect(title().placeholder).toBe("GitHub");
    for (const label of ["Título", "Usuario o correo", "Contraseña", "Sitio", "Notas"]) {
      expect(within(dialog).getByLabelText<HTMLInputElement>(label).value).toBe("");
    }
    expect(within(dialog).getByRole("button", { name: "Guardar entrada" })).toBeDefined();
  });

  it("RF-100: sin cambios, Esc, × y Cancelar cierran sin preguntar", () => {
    for (const [, leave] of WAYS_OUT) {
      const { onClose } = open();
      leave();
      expect(confirmation()).toBeNull();
      expect(onClose).toHaveBeenCalledTimes(1);
      cleanup();
    }
  });

  it("RF-100: con cambios, Esc, × y Cancelar preguntan; «Seguir editando» conserva el borrador y «Descartar» cierra", () => {
    for (const [, leave] of WAYS_OUT) {
      const { onClose, onSaved } = open();
      fireEvent.change(title(), { target: { value: "A medias" } });

      leave();
      const question = confirmation();
      if (question === null) throw new Error("the dialog did not ask");
      expect(onClose).not.toHaveBeenCalled();
      // The way out that loses nothing has the focus.
      expect(document.activeElement).toBe(within(question).getByRole("button", { name: "Seguir editando" }));

      fireEvent.click(within(question).getByRole("button", { name: "Seguir editando" }));
      expect(confirmation()).toBeNull();
      expect(title().value).toBe("A medias");
      expect(onClose).not.toHaveBeenCalled();

      leave();
      fireEvent.click(screen.getByRole("button", { name: "Descartar" }));
      expect(onClose).toHaveBeenCalledTimes(1);
      expect(onSaved).not.toHaveBeenCalled();
      cleanup();
    }
  });

  it("RF-100: una contraseña escrita cuenta como cambio, y borrarla deja de contar", () => {
    const { onClose } = open();
    typePassword("escrita-sin-guardar");
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(confirmation()).not.toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Seguir editando" }));

    typePassword("");
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(confirmation()).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("RF-100: un clic en el fondo no cierra ni pregunta", () => {
    const { onClose } = open();
    fireEvent.change(title(), { target: { value: "A medias" } });

    fireEvent.click(sheet());

    expect(confirmation()).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
    expect(title().value).toBe("A medias");
  });

  it("RF-32: cuando la bóveda se bloquea y el diálogo se desmonta, no queda nada del borrador en el DOM", () => {
    const { view } = open();
    fireEvent.change(title(), { target: { value: "borrador-a-medias" } });
    typePassword("escrita-sin-guardar");

    view.rerender(<p>Desbloquear</p>);

    for (const value of ["borrador-a-medias", "escrita-sin-guardar"]) expect(bodyContains(value)).toBe(false);
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("Diálogo de entrada — crear", () => {
  it("RF-39: guardar sin título lo dice bajo el campo, le devuelve el foco y no llama al núcleo", () => {
    const { backend, onClose, onSaved } = open();
    fireEvent.change(screen.getByLabelText("Usuario o correo"), { target: { value: "ana" } });
    screen.getByLabelText<HTMLInputElement>("Usuario o correo").focus();

    for (const value of ["", "   "]) {
      fireEvent.change(title(), { target: { value } });
      fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

      const message = screen.getByText("Ponle un título para poder encontrarla después.");
      expect(message.getAttribute("role")).toBe("alert");
      expect(title().getAttribute("aria-invalid")).toBe("true");
      expect(title().getAttribute("aria-describedby")).toContain(message.id);
      expect(document.activeElement).toBe(title());
    }
    expect(backend.count("entry_create")).toBe(0);
    expect(onSaved).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(sheet().open).toBe(true);
  });

  it("RF-38: RF-100: guardar crea la entrada en el grupo, llama a onSaved con su id y cierra", async () => {
    const { backend, onClose, onSaved } = open();
    fireEvent.change(title(), { target: { value: "  Correo  " } });
    fireEvent.change(screen.getByLabelText("Usuario o correo"), { target: { value: "ana@example.com" } });
    typePassword("  Clave Con Espacios  ");
    fireEvent.change(screen.getByLabelText("Sitio"), { target: { value: "https://correo.example" } });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "línea 1\nlínea 2" } });
    fireEvent.click(screen.getByRole("button", { name: "Más campos" }));
    fireEvent.change(screen.getByLabelText("Etiquetas (una por línea)"), { target: { value: "correo\npersonal" } });

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    const id = onSaved.mock.calls[0]?.[0] ?? "";
    expect(onSaved).toHaveBeenCalledWith(id, true);
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(lastArgs(backend, "entry_create")).toMatchObject({ group_id: "group-personal" });
    // RF-41: the password travels as written; RF-91: one tag per line.
    expect(backend.entry(id)).toMatchObject({
      groupId: "group-personal",
      title: "Correo",
      username: "ana@example.com",
      password: "  Clave Con Espacios  ",
      url: "https://correo.example",
      notes: "línea 1\nlínea 2",
      tags: ["correo", "personal"],
    });
  });

  it("RF-38: mientras guarda lo dice, no envía dos veces y no deja cerrar", async () => {
    const { backend, onClose, onSaved } = open();
    const release = backend.hold("entry_create");
    fireEvent.change(title(), { target: { value: "Lenta" } });
    typePassword("clave-en-camino");

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    const busy = await screen.findByRole("button", { name: "Guardando…" });
    expect(busy.getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByRole("status").textContent).toBe("Guardando la entrada…");
    // The password already left the field.
    expect(bodyContains("clave-en-camino")).toBe(false);

    fireEvent.click(busy);
    fireEvent.submit(busy.closest("form") ?? busy);
    pressEscape(sheet());
    fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Cancelar" }).disabled).toBe(true);
    expect(confirmation()).toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    release();
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(backend.count("entry_create")).toBe(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("RF-23: un error del núcleo se queda en el diálogo, sin cerrarlo, y pide escribir otra vez la contraseña", async () => {
    const { backend, onClose, onSaved } = open();
    backend.rejectOnce("entry_create", { code: "SoloLectura", message: "core text" });
    fireEvent.change(title(), { target: { value: "No se guarda" } });
    typePassword("clave-perdida");

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await within(sheet()).findByText(ERROR_MESSAGES.SoloLectura)).toBeDefined();
    expect(screen.queryByText(/core text/)).toBeNull();
    expect(sheet().open).toBe(true);
    expect(onClose).not.toHaveBeenCalled();
    expect(onSaved).not.toHaveBeenCalled();
    expect(title().value).toBe("No se guarda");
    expect(passwordField().value).toBe("");
    expect(screen.getByText(/La contraseña se borró del formulario por seguridad/)).toBeDefined();

    // It can be sent again.
    typePassword("clave-nueva");
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(backend.entry(onSaved.mock.calls[0]?.[0] ?? "")?.password).toBe("clave-nueva");
  });

  it("RF-40: RF-91: un título largo o una etiqueta con ; se rechazan junto a su campo, y el de etiquetas abre «Más campos»", () => {
    const { backend } = open();
    fireEvent.change(title(), { target: { value: "a".repeat(256) } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    expect(title().getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById("entry-dialog-title-error")?.textContent).toMatch(/255/);

    fireEvent.change(title(), { target: { value: "Con etiquetas" } });
    fireEvent.click(screen.getByRole("button", { name: "Más campos" }));
    fireEvent.change(screen.getByLabelText("Etiquetas (una por línea)"), { target: { value: "uno;dos" } });
    fireEvent.click(screen.getByRole("button", { name: "Más campos" }));
    expect(screen.queryByLabelText("Etiquetas (una por línea)")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    const tags = screen.getByLabelText("Etiquetas (una por línea)");
    expect(tags.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById("entry-dialog-tags-error")?.textContent).not.toBe("");
    expect(document.activeElement).toBe(tags);
    expect(title().getAttribute("aria-invalid")).toBe("false");
    expect(backend.count("entry_create")).toBe(0);
  });
});

describe("Diálogo de entrada — editar", () => {
  it("RF-45: ejemplo 8 — carga los datos, la contraseña nace vacía y cambiar solo el título envía solo el título", async () => {
    const { backend, onClose, onSaved } = await openEdit();

    const dialog = sheet("Editar entrada");
    expect(within(dialog).getByText("En el grupo Trabajo")).toBeDefined();
    expect(document.activeElement).toBe(title());
    expect(title().value).toBe("GitHub");
    expect(screen.getByLabelText<HTMLInputElement>("Usuario o correo").value).toBe("ana@example.com");
    expect(screen.getByLabelText<HTMLInputElement>("Sitio").value).toBe("https://github.com/login");
    expect(screen.getByLabelText<HTMLTextAreaElement>("Notas").value).toBe("cuenta del trabajo");
    expect(passwordField().value).toBe("");
    expect(screen.getByText(/Déjala vacía para conservar la contraseña actual/)).toBeDefined();
    expect(bodyContains(PASSWORD)).toBe(false);

    fireEvent.change(title(), { target: { value: "GitHub (trabajo)" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith("entry-github", true);
    });
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", title: "GitHub (trabajo)" });
    expect(backend.entry("entry-github")).toMatchObject({ title: "GitHub (trabajo)", password: PASSWORD });
  });

  it("RF-45: guardar sin haber cambiado nada no llama al núcleo, y cerrar tampoco pregunta", async () => {
    const { backend, onClose, onSaved } = await openEdit();

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(onSaved).toHaveBeenCalledWith("entry-github", false);
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(backend.count("entry_update")).toBe(0);

    pressEscape(sheet("Editar entrada"));
    expect(confirmation()).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("RF-45: escribir una contraseña nueva la envía, y solo eso", async () => {
    const { backend, onSaved } = await openEdit();
    typePassword("cuarta-clave");

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith("entry-github", true);
    });
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", password: "cuarta-clave" });
    expect(backend.entry("entry-github")?.password).toBe("cuarta-clave");
  });

  it("RF-45: «Más campos» nace abierto si la entrada tiene etiquetas o campos propios, y cerrado si no", async () => {
    await openEdit();
    expect(screen.getByRole("button", { name: "Más campos" }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByLabelText<HTMLTextAreaElement>("Etiquetas (una por línea)").value).toBe("código\ntrabajo");
    // Own fields are named, never shown: the window has no command to edit them.
    expect(screen.getByText(/Campos propios de esta entrada: Recuperación, PIN\./)).toBeDefined();
    expect(bodyContains("4321")).toBe(false);
    cleanup();

    await openEdit("entry-door");
    expect(screen.getByRole("button", { name: "Más campos" }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText(/Campos propios de esta entrada: Clave del portón\./)).toBeDefined();
    cleanup();

    await openEdit("entry-years");
    expect(screen.getByRole("button", { name: "Más campos" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByLabelText("Etiquetas (una por línea)")).toBeNull();
    cleanup();

    open();
    const more = screen.getByRole("button", { name: "Más campos" });
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(more.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByLabelText<HTMLTextAreaElement>("Etiquetas (una por línea)").value).toBe("");
  });
});

const KIND_LABELS = ["Minúsculas", "Mayúsculas", "Números", "Símbolos"] as const;
const DEFAULT_RULES = { length: 20, lower: true, upper: true, digits: true, symbols: true, avoid_lookalikes: false };

/** One click on "Generar", and what the field holds once the core answers. */
async function generateOnce(): Promise<string> {
  const before = passwordField().value;
  fireEvent.click(screen.getByRole("button", { name: "Generar" }));
  await waitFor(() => {
    expect(passwordField().value).not.toBe(before);
  });
  return passwordField().value;
}

function openOptions() {
  fireEvent.click(screen.getByRole("button", { name: "Opciones del generador" }));
  return screen.getByRole("group", { name: "Caracteres de la contraseña generada" });
}

describe("Diálogo de entrada — contraseña y generador en el campo", () => {
  it("RF-101: RF-65: RF-70: un clic en «Generar» pide la contraseña al núcleo, llena el campo, la deja a la vista y muestra su fortaleza", async () => {
    const { backend } = open();
    expect(passwordField().type).toBe("password");

    const generated = await generateOnce();

    expect(Array.from(generated)).toHaveLength(20);
    expect(backend.count("generate_password")).toBe(1);
    expect(lastArgs(backend, "generate_password")).toEqual(DEFAULT_RULES);
    expect(passwordField().type).toBe("text");
    expect(screen.getByRole("button", { name: "Ocultar contraseña" })).toBeDefined();
    expect(screen.getByText(/^Fortaleza: /)).toBeDefined();
    // It is shown inside the input alone: no text node, title, label or data attribute carries it.
    expect(document.body.innerHTML.includes(generated)).toBe(false);
  });

  it("RF-101: crear una entrada con contraseña generada son Título, un clic en «Generar» y Guardar", async () => {
    const { backend, onSaved } = open();
    fireEvent.change(title(), { target: { value: "Con generada" } });
    const generated = await generateOnce();

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(backend.entry(onSaved.mock.calls[0]?.[0] ?? "")?.password).toBe(generated);
    expect(bodyContains(generated)).toBe(false);
  });

  it("RF-101: el ojo muestra y oculta lo escrito, y su nombre cambia", () => {
    open();
    typePassword("escrita-a-mano");
    expect(passwordField().type).toBe("password");

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(passwordField().type).toBe("text");
    expect(passwordField().value).toBe("escrita-a-mano");
    expect(document.body.innerHTML.includes("escrita-a-mano")).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Ocultar contraseña" }));
    expect(passwordField().type).toBe("password");
    expect(passwordField().value).toBe("escrita-a-mano");
    expect(screen.getByRole("button", { name: "Mostrar contraseña" })).toBeDefined();
  });

  it("RF-70: muestra la fortaleza de la contraseña escrita y la quita al vaciar el campo", async () => {
    const { backend } = open();
    typePassword("abcdefgh1");

    expect(await screen.findByText(/^Fortaleza: /)).toBeDefined();
    expect(lastArgs(backend, "password_strength")).toEqual({ password: "abcdefgh1" });

    typePassword("");
    expect(screen.queryByText(/^Fortaleza: /)).toBeNull();
  });

  it("RF-65: RF-67: las opciones nacen plegadas y con los valores por defecto: longitud 20 entre 8 y 128, los cuatro tipos y sin evitar parecidos", () => {
    open();
    const toggle = screen.getByRole("button", { name: "Opciones del generador" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByLabelText("Longitud")).toBeNull();

    const kinds = openOptions();

    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(toggle.textContent).toBe("Ocultar opciones del generador");
    const length = screen.getByLabelText<HTMLInputElement>("Longitud");
    expect([length.type, length.min, length.max, length.value]).toEqual(["range", "8", "128", "20"]);
    for (const label of KIND_LABELS) expect(within(kinds).getByLabelText<HTMLInputElement>(label).checked).toBe(true);
    expect(within(kinds).getByLabelText<HTMLInputElement>("Evitar caracteres parecidos (0 O o 1 l I |)").checked).toBe(false);

    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.textContent).toBe("Opciones del generador");
    expect(screen.queryByLabelText("Longitud")).toBeNull();
  });

  it("RF-66: RF-67: ejemplo 5 — longitud 8, cuatro tipos y evitar parecidos: así viaja la petición y así sale", async () => {
    const { backend } = open();
    openOptions();
    fireEvent.change(screen.getByLabelText("Longitud"), { target: { value: "8" } });
    fireEvent.click(screen.getByLabelText("Evitar caracteres parecidos (0 O o 1 l I |)"));
    expect(backend.count("generate_password")).toBe(0);

    const password = await generateOnce();

    expect(lastArgs(backend, "generate_password")).toEqual({ ...DEFAULT_RULES, length: 8, avoid_lookalikes: true });
    expect(Array.from(password)).toHaveLength(8);
    expect(password).toMatch(/[a-z]/);
    expect(password).toMatch(/[A-Z]/);
    expect(password).toMatch(/[0-9]/);
    expect(password).toMatch(/[^a-zA-Z0-9]/);
    expect(password).not.toMatch(/[0Oo1lI|]/);
  });

  it("RF-101: cambiar una opción vuelve a generar si lo que hay en el campo salió del generador", async () => {
    const { backend } = open();
    const first = await generateOnce();
    openOptions();

    fireEvent.change(screen.getByLabelText("Longitud"), { target: { value: "12" } });
    await waitFor(() => {
      expect(Array.from(passwordField().value)).toHaveLength(12);
    });
    expect(backend.count("generate_password")).toBe(2);
    expect(lastArgs(backend, "generate_password")).toEqual({ ...DEFAULT_RULES, length: 12 });
    expect(passwordField().value).not.toBe(first);

    const second = passwordField().value;
    fireEvent.click(screen.getByLabelText("Símbolos"));
    await waitFor(() => {
      expect(passwordField().value).not.toBe(second);
    });
    expect(backend.count("generate_password")).toBe(3);
    expect(passwordField().value).toMatch(/^[a-zA-Z0-9]{12}$/);
    expect(passwordField().type).toBe("text");
  });

  it("RF-101: cambiar una opción no pisa una contraseña escrita ni una generada que se editó a mano", async () => {
    const { backend } = open();
    typePassword("escrita-a-mano");
    openOptions();
    fireEvent.change(screen.getByLabelText("Longitud"), { target: { value: "30" } });
    fireEvent.click(screen.getByLabelText("Símbolos"));
    expect(backend.count("generate_password")).toBe(0);
    expect(passwordField().value).toBe("escrita-a-mano");

    const generated = await generateOnce();
    expect(Array.from(generated)).toHaveLength(30);
    typePassword(`${generated}-y-algo-mío`);
    fireEvent.click(screen.getByLabelText("Mayúsculas"));
    fireEvent.change(screen.getByLabelText("Longitud"), { target: { value: "9" } });

    // Whatever the core had to say would have arrived by now.
    await screen.findByText(/^Fortaleza: /);
    expect(backend.count("generate_password")).toBe(1);
    expect(passwordField().value).toBe(`${generated}-y-algo-mío`);
  });

  it("RF-101: una contraseña generada que llega tarde no pisa lo que se escribió mientras tanto", async () => {
    const { backend } = open();
    const release = backend.hold("generate_password");
    fireEvent.click(screen.getByRole("button", { name: "Generar" }));
    typePassword("escrita-mientras-tanto");

    release();
    await waitFor(() => {
      expect(backend.count("generate_password")).toBe(1);
    });
    await screen.findByText(/^Fortaleza: /);

    expect(passwordField().value).toBe("escrita-mientras-tanto");
    expect(passwordField().type).toBe("password");
  });

  it("RF-68: sin ningún tipo marcado no genera y lo dice junto a las opciones; un rechazo del núcleo también se dice ahí", async () => {
    const { backend } = open();
    const kinds = openOptions();
    for (const label of KIND_LABELS) fireEvent.click(within(kinds).getByLabelText(label));
    const options = kinds.parentElement ?? kinds;
    expect(within(options).getByRole("alert").textContent).toBe(ERROR_MESSAGES.ReglasImposibles);

    // Folded away, "Generar" still says why: it opens the options with the reason.
    fireEvent.click(screen.getByRole("button", { name: "Ocultar opciones del generador" }));
    fireEvent.click(screen.getByRole("button", { name: "Generar" }));
    expect(screen.getByText(ERROR_MESSAGES.ReglasImposibles)).toBeDefined();
    expect(screen.getByRole("button", { name: "Ocultar opciones del generador" })).toBeDefined();
    expect(backend.count("generate_password")).toBe(0);
    expect(passwordField().value).toBe("");

    fireEvent.click(screen.getByLabelText("Números"));
    expect(screen.queryByText(ERROR_MESSAGES.ReglasImposibles)).toBeNull();
    backend.rejectOnce("generate_password", { code: "ReglasImposibles", message: "core text" });
    fireEvent.click(screen.getByRole("button", { name: "Generar" }));
    expect(await screen.findByText(ERROR_MESSAGES.ReglasImposibles)).toBeDefined();
    expect(backend.count("generate_password")).toBe(1);
    expect(screen.queryByText(/core text/)).toBeNull();
    expect(passwordField().value).toBe("");
  });

  it("RF-65: Enter en una opción del generador genera y no guarda la entrada", async () => {
    const { backend, onSaved } = open();
    fireEvent.change(title(), { target: { value: "No guardar" } });
    openOptions();

    const notCancelled = fireEvent.keyDown(screen.getByLabelText("Longitud"), { key: "Enter" });

    expect(notCancelled).toBe(false);
    await waitFor(() => {
      expect(backend.count("generate_password")).toBe(1);
    });
    expect(backend.count("entry_create")).toBe(0);
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("RF-45: al editar, «Generar» reemplaza la contraseña: viaja la generada y solo ella", async () => {
    const { backend, onSaved } = await openEdit();
    const generated = await generateOnce();

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith("entry-github", true);
    });
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", password: generated });
    expect(backend.entry("entry-github")?.password).toBe(generated);
  });

  it("RF-65: RF-32: cerrar el diálogo suelta la contraseña generada: pregunta antes y no queda en el DOM", async () => {
    const { onClose, view } = open();
    const generated = await generateOnce();

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(confirmation()).not.toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Descartar" }));
    expect(onClose).toHaveBeenCalledTimes(1);

    view.rerender(<p>Bóveda</p>);
    expect(bodyContains(generated)).toBe(false);
  });
});

/**
 * What Chromium can do on a repeated Escape: close the dialog on its own, without `cancel` and without React. The
 * element is still mounted; only `open` is gone and `close` fires.
 */
function closedByTheBrowser(dialog: HTMLDialogElement) {
  dialog.open = false;
  fireEvent(dialog, new Event("close"));
}

describe("Diálogo de entrada — el navegador lo cierra por su cuenta", () => {
  it("RF-100: cerrado por el navegador con cambios sin guardar, vuelve a abrirse con lo escrito y pregunta antes de descartar", () => {
    const { onClose, onSaved } = open();
    fireEvent.change(title(), { target: { value: "A medias" } });
    typePassword("escrita-sin-guardar");
    const dialog = sheet();

    closedByTheBrowser(dialog);

    expect(dialog.open).toBe(true);
    expect(title().value).toBe("A medias");
    expect(passwordField().value).toBe("escrita-sin-guardar");
    expect(confirmation()).not.toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Seguir editando" }));
    expect(confirmation()).toBeNull();
    expect(dialog.open).toBe(true);
    expect(title().value).toBe("A medias");
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("RF-100: cerrado por el navegador sin cambios, se va como con Esc: una sola petición de cierre y ninguna pregunta", () => {
    const { onClose } = open();

    closedByTheBrowser(sheet());

    expect(confirmation()).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("RF-100: cerrado por el navegador mientras guarda, vuelve a abrirse sin preguntar y el guardado termina", async () => {
    const { backend, onClose, onSaved } = open();
    const release = backend.hold("entry_create");
    fireEvent.change(title(), { target: { value: "Lenta" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    await screen.findByRole("button", { name: "Guardando…" });
    const dialog = sheet();

    closedByTheBrowser(dialog);

    expect(dialog.open).toBe(true);
    expect(confirmation()).toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    release();
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(backend.count("entry_create")).toBe(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("RF-100: al desmontarse, el cierre que hace el propio diálogo no cuenta como cierre del navegador", () => {
    const { onClose, view } = open();
    fireEvent.change(title(), { target: { value: "A medias" } });

    view.rerender(<p>Bóveda</p>);

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("Diálogo de entrada — Esc repetido", () => {
  it("RF-100: si el navegador avisa con un `cancel` que ya no se puede anular y cierra el diálogo, la pregunta queda encima del diálogo reabierto y sus dos salidas funcionan", () => {
    const { onClose } = open();
    fireEvent.change(title(), { target: { value: "A medias" } });
    typePassword("escrita-sin-guardar");
    const dialog = sheet();
    // The top layer is a stack: the last dialog shown is the one on top, and the only one that can be used.
    const shown: HTMLDialogElement[] = [];
    const spy = vi.spyOn(HTMLDialogElement.prototype, "showModal").mockImplementation(function (this: HTMLDialogElement) {
      shown.push(this);
      this.open = true;
    });

    fireEvent(dialog, new Event("cancel", { cancelable: false }));
    closedByTheBrowser(dialog);

    const question = confirmation();
    expect(dialog.open).toBe(true);
    expect(question).not.toBeNull();
    expect(shown).toContain(dialog);
    expect(shown.at(-1)).toBe(question);
    expect(screen.getAllByRole("dialog", { name: "¿Descartar los cambios?" })).toHaveLength(1);
    spy.mockRestore();

    fireEvent.click(screen.getByRole("button", { name: "Seguir editando" }));
    expect(confirmation()).toBeNull();
    expect(title().value).toBe("A medias");
    expect(passwordField().value).toBe("escrita-sin-guardar");
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    fireEvent.click(screen.getByRole("button", { name: "Descartar" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("RF-72: el campo de contraseña del diálogo va marcado como valor protegido, a la vista u oculto", () => {
    open();
    expect(passwordField().hasAttribute("data-secret")).toBe(true);
    typePassword("escrita-a-mano");
    expect(passwordField().hasAttribute("data-secret")).toBe(true);
  });
});
