import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeEntry, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const PATH = "D:/bovedas/cuentas.kdbx";
const OPEN = {
  state: "open",
  path: PATH,
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
  created: "2026-10-01T08:00:00Z",
  modified: "2026-11-01T08:00:00Z",
  customFields: [
    { name: "Recuperación", protected: false, value: "pregunta: mascota" },
    { name: "PIN", protected: true, value: "4321" },
  ],
  attachments: [{ name: "codigos.txt", size: 120 }],
};

const ENTRIES: readonly FakeEntry[] = [
  ...SAMPLE_ENTRIES.filter((entry) => entry.id !== "entry-github" && entry.id !== "entry-vpn"),
  GITHUB,
  {
    id: "entry-vpn",
    groupId: "group-work",
    title: "VPN",
    username: null,
    url: "vpn.example",
    password: "clave-vpn",
    secrets: { UserName: "usuario-vpn" },
  },
];

async function setup(options: FakeOptions = {}) {
  const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: ENTRIES, ...options });
  const view = render(<VaultRoot client={createClient(backend.transport)} />);
  await screen.findByRole("heading", { name: "Cuentas" });
  return { backend, container: view.container };
}

/** RF-43: the window of the entry, the modal dialog its panel is shown in. */
function detailZone(): HTMLElement {
  const entryWindow = document.querySelector<HTMLElement>('dialog[data-layout="entry"]');
  if (entryWindow === null) throw new Error("the window of the entry is not open");
  return entryWindow;
}
const lastArgs = (backend: { calls: readonly { command: string; args: unknown }[] }, command: string) =>
  backend.calls.filter((call) => call.command === command).at(-1)?.args;

async function openGroup(name: string) {
  fireEvent.click(await screen.findByRole("button", { name: new RegExp(`^${name} `) }));
  await screen.findByRole("heading", { name: `Entradas de ${name}` });
  await waitFor(() => {
    expect(screen.queryByText("Cargando las entradas…")).toBeNull();
  });
}

/** RF-106: chooses the row of that entry (a click) and opens its window (a double click on its title). */
async function openRow(title: string) {
  const row = within(screen.getByRole("table")).getByRole("button", { name: title });
  fireEvent.click(row);
  fireEvent.doubleClick(row);
  await within(detailZone()).findByRole("heading", { name: title });
}

async function openEntry(group: string, title: string) {
  await openGroup(group);
  await openRow(title);
  await within(detailZone()).findByRole("button", { name: "Editar" });
}

/** RF-108: "Cerrar" in the head of the window. Behind a modal window the list is out of reach, so it goes first. */
function closeWindow() {
  fireEvent.click(within(detailZone()).getByRole("button", { name: "Cerrar" }));
}

const moreButton = () => within(detailZone()).getByRole("button", { name: "Más acciones de la entrada" });

/** Opens the menu of the entry and chooses one of its orders, the way a person does with the pointer. */
function chooseOrder(label: string) {
  fireEvent.click(moreButton());
  fireEvent.click(screen.getByRole("menuitem", { name: label }));
}

/** The row of the card with that label. */
function rowOf(label: string): HTMLElement {
  const row = within(detailZone()).getByText(label, { selector: "dt" }).closest<HTMLElement>(".card-row");
  if (row === null) throw new Error(`missing row ${label}`);
  return row;
}

/** The copy notice: said in the window of the entry while it is open, and at the foot of the list when it is not. */
const copyNotice = () =>
  [...document.querySelectorAll('.toast-zone[role="status"]')].map((zone) => zone.textContent).join("");

async function startNewEntry(group = "Personal") {
  await openGroup(group);
  fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
  return screen.findByRole("dialog", { name: "Nueva entrada" });
}

/** The tags sit under "Más campos", which is folded on a new entry. */
function tagsField() {
  const more = screen.getByRole("button", { name: "Más campos" });
  if (more.getAttribute("aria-expanded") !== "true") fireEvent.click(more);
  return screen.getByLabelText<HTMLTextAreaElement>("Etiquetas (una por línea)");
}

function passwordField() {
  return screen.getByLabelText<HTMLInputElement>("Contraseña");
}

function typePassword(value: string) {
  fireEvent.input(passwordField(), { target: { value } });
}

function bodyContains(value: string): boolean {
  if (document.body.innerHTML.includes(value)) return true;
  return [...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea")].some((field) =>
    field.value.includes(value),
  );
}

describe("P5 Entrada — ver, mostrar y copiar", () => {
  it("RF-43: éxito — muestra los campos no protegidos, los nombres de campos propios y adjuntos, y nunca la contraseña", async () => {
    const { backend } = await setup();

    await openEntry("Trabajo", "GitHub");

    const detail = detailZone();
    expect(lastArgs(backend, "entry_get")).toEqual({ id: "entry-github" });
    expect(within(detail).getByRole("heading", { name: "GitHub" })).toBeDefined();
    expect(within(detail).getByText("ana@example.com")).toBeDefined();
    expect(within(detail).getByText("https://github.com/login")).toBeDefined();
    expect(within(detail).getByText("cuenta del trabajo")).toBeDefined();
    expect(within(detail).getByText("código")).toBeDefined();
    expect(within(detail).getByText("Recuperación")).toBeDefined();
    expect(within(detail).getByText("pregunta: mascota")).toBeDefined();
    expect(within(detail).getByText("PIN")).toBeDefined();
    expect(within(detail).getByText("codigos.txt")).toBeDefined();
    expect(detail.querySelector('time[datetime="2026-10-01T08:00:00Z"]')).not.toBeNull();
    expect(within(detail).getByRole("button", { name: "Mostrar contraseña" })).toBeDefined();
    expect(within(detail).getByRole("button", { name: "Mostrar «PIN»" })).toBeDefined();
    // RF-76: neither the password nor the protected custom value reached the window.
    expect(bodyContains(PASSWORD)).toBe(false);
    expect(bodyContains("4321")).toBe(false);
  });

  it("RF-43: un campo estándar protegido (null) se ve oculto tras la barra, en su propia fila, y solo se muestra a pedido", async () => {
    await setup();

    await openEntry("Trabajo", "VPN");

    const detail = detailZone();
    // One row for the user, not two: its value is behind the bar of fixed width, with show and copy next to it.
    const row = rowOf("Usuario");
    expect(within(detail).getAllByText("Usuario")).toHaveLength(1);
    expect(row.querySelector(".secret[data-masked]")?.textContent).toBe("(oculto)");
    expect(within(row).getByRole("button", { name: "Mostrar usuario" })).toBeDefined();
    expect(within(row).getByRole("button", { name: "Copiar usuario" })).toBeDefined();
    expect(within(detail).queryByText("usuario-vpn")).toBeNull();

    fireEvent.click(within(row).getByRole("button", { name: "Mostrar usuario" }));
    expect((await within(row).findByText("usuario-vpn")).className).toBe("secret");
  });

  it("RF-44: mostrar pide solo ese campo, lo oculta con el botón y al cambiar de entrada", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(await screen.findByText(PASSWORD)).toBeDefined();
    expect(lastArgs(backend, "entry_reveal")).toEqual({ id: "entry-github", field: "Password" });
    expect(bodyContains("4321")).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Ocultar contraseña" }));
    expect(screen.queryByText(PASSWORD)).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(await screen.findByText(PASSWORD)).toBeDefined();
    // RF-108: closing the window hides what was shown, and so does opening another entry.
    closeWindow();
    expect(bodyContains(PASSWORD)).toBe(false);
    await openRow("VPN");
    await within(detailZone()).findByRole("button", { name: "Editar" });
    expect(bodyContains(PASSWORD)).toBe(false);
  });

  it("RF-44: un valor mostrado se oculta solo a los 20 s", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(await screen.findByText(PASSWORD)).toBeDefined();

    await act(() => vi.advanceTimersByTimeAsync(19_000));
    expect(screen.queryByText(PASSWORD)).not.toBeNull();
    await act(() => vi.advanceTimersByTimeAsync(1_500));
    expect(screen.queryByText(PASSWORD)).toBeNull();
  });

  it("RF-71: RF-74: copiar va por el núcleo, el valor no pasa por la ventana y cuenta los segundos", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar contraseña" }));

    expect(await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "Password" });
    expect(backend.clipboard()).toBe(PASSWORD);
    expect(bodyContains(PASSWORD)).toBe(false);
    expect(backend.count("entry_reveal")).toBe(0);

    await act(() => vi.advanceTimersByTimeAsync(5_000));
    expect(screen.getByText("Contraseña copiada. Se borra del portapapeles en 15 s")).toBeDefined();
    // One source: the notice is the only thing on screen that speaks of the copy.
    expect(screen.getAllByText(/copiad/i)).toHaveLength(1);
    expect(copyNotice()).toBe("Contraseña copiada. Se borra del portapapeles en 15 s");
    await act(() => vi.advanceTimersByTimeAsync(15_000));
    expect(screen.getByText(/limpió el portapapeles/)).toBeDefined();
    expect(screen.queryByText(/Se borra del portapapeles/)).toBeNull();
  });

  it("RF-74: copiar el usuario también va por entry_copy; como no es protegido, no hay cuenta regresiva", async () => {
    const { backend } = await setup({ clipboardSeconds: 45 });
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar usuario" }));

    // plan.md, precision 16: `clears_in_seconds` is `null` for a field that is not protected.
    expect(await screen.findByText("Usuario copiado")).toBeDefined();
    expect(copyNotice()).toBe("Usuario copiado");
    expect(screen.queryByText(/Se borra del portapapeles/)).toBeNull();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "UserName" });
    expect(backend.clipboard()).toBe("ana@example.com");
  });

  it("RF-83: error — si copiar falla lo dice sin el texto del núcleo", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    backend.rejectOnce("entry_copy", { code: "Bloqueada", message: "core text" });

    fireEvent.click(screen.getByRole("button", { name: "Copiar contraseña" }));

    expect(await screen.findByText(ERROR_MESSAGES.Bloqueada)).toBeDefined();
    expect(screen.queryByText(/copiad/i)).toBeNull();
    expect(copyNotice()).toBe("");
  });

  it("RF-49: abre una URL http(s) por el núcleo y explica UrlNoPermitida sin anteponer https://", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Abrir el sitio en el navegador" }));
    expect(await screen.findByText("La dirección se abrió en el navegador.")).toBeDefined();
    expect(backend.openedUrls).toEqual(["https://github.com/login"]);

    closeWindow();
    await openRow("VPN");
    await within(detailZone()).findByRole("button", { name: "Editar" });
    fireEvent.click(screen.getByRole("button", { name: "Abrir el sitio en el navegador" }));

    expect(await screen.findByText(ERROR_MESSAGES.UrlNoPermitida)).toBeDefined();
    expect(lastArgs(backend, "entry_open_url")).toEqual({ id: "entry-vpn" });
    expect(backend.openedUrls).toEqual(["https://github.com/login"]);
  });

  it("RF-78: notas, campos propios, adjuntos y un valor mostrado se pintan como texto literal", async () => {
    const payloads = {
      notes: "<img src=x onerror=alert(1)>",
      customName: "<b onclick=alert(2)>campo</b>",
      customValue: "<script>alert(3)</script>",
      attachment: "<iframe src=javascript:alert(4)>",
      password: "<a href=javascript:alert(5)>clave</a>",
    };
    const { container } = await setup({
      entries: [
        {
          id: "x",
          groupId: "group-personal",
          title: "Rara",
          notes: payloads.notes,
          password: payloads.password,
          customFields: [{ name: payloads.customName, protected: false, value: payloads.customValue }],
          attachments: [{ name: payloads.attachment, size: 1 }],
        },
      ],
    });
    await openEntry("Personal", "Rara");
    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    await screen.findByText(payloads.password);

    for (const text of Object.values(payloads)) {
      for (const node of screen.getAllByText(text)) {
        expect([...node.childNodes].every((child) => child.nodeType === Node.TEXT_NODE)).toBe(true);
      }
    }
    expect(container.querySelector("script, img, b, a, iframe")).toBeNull();
    expect(container.querySelector('[href^="javascript"], [onerror], [onclick]')).toBeNull();
  });

  it("RF-43: carga y error —mientras llega la entrada lo dice; si falla deja reintentar", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");
    const release = backend.hold("entry_get");

    await openRow("GitHub");
    expect(await screen.findByText("Cargando la entrada…")).toBeDefined();
    await act(async () => {
      release();
      await Promise.resolve();
    });
    await within(detailZone()).findByRole("button", { name: "Editar" });

    backend.rejectOnce("entry_get", { code: "NoEncontrado", message: "core text" });
    closeWindow();
    await openRow("VPN");
    expect(await screen.findByText(ERROR_MESSAGES.NoEncontrado)).toBeDefined();
    fireEvent.click(within(detailZone()).getByRole("button", { name: "Reintentar" }));
    expect(await within(detailZone()).findByRole("heading", { name: "VPN" })).toBeDefined();
  });
});

describe("P5 Entrada — crear y editar", () => {
  it("RF-38: vacío y éxito — «Nueva entrada» abre un formulario vacío y crea la entrada en el grupo elegido", async () => {
    const { backend } = await setup();
    await startNewEntry("Personal");

    expect(screen.getByLabelText<HTMLInputElement>("Título").value).toBe("");
    expect(document.activeElement).toBe(screen.getByLabelText("Título"));
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Correo" } });
    fireEvent.change(screen.getByLabelText("Usuario o correo"), { target: { value: "ana" } });
    typePassword("clave larga 123");
    fireEvent.change(screen.getByLabelText("Sitio"), { target: { value: "https://correo.example" } });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "personal" } });
    fireEvent.change(tagsField(), { target: { value: "correo\n\n hogar \n" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();
    expect(lastArgs(backend, "entry_create")).toEqual({
      group_id: "group-personal",
      title: "Correo",
      username: "ana",
      password: "clave larga 123",
      url: "https://correo.example",
      notes: "personal",
      tags: ["correo", "hogar"],
    });
    // The new entry is the chosen row of the list; its window does not open by itself (RF-106).
    await waitFor(() => {
      const row = within(screen.getByRole("table")).getByRole("button", { name: "Correo" });
      expect(row.getAttribute("aria-current")).toBe("true");
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(bodyContains("clave larga 123")).toBe(false);
  });

  it("RF-38: carga — mientras guarda lo dice y la contraseña ya salió del campo", async () => {
    const { backend } = await setup();
    await startNewEntry();
    const release = backend.hold("entry_create");

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Correo" } });
    typePassword("secreta");
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Guardando la entrada…")).toBeDefined();
    expect(passwordField().value).toBe("");
    await act(async () => {
      release();
      await Promise.resolve();
    });
    expect(await screen.findByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();
  });

  it("RF-41: la contraseña viaja tal cual: sin recortar, sin cambiar mayúsculas ni normalizar", async () => {
    const { backend } = await setup();
    await startNewEntry();

    const written = "  Clave\u0301 Ñ  ";
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Exacta" } });
    typePassword(written);
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();
    expect((lastArgs(backend, "entry_create") as { password: string }).password).toBe(written);
  });

  it("RF-39: RF-40: ejemplo 9 —título vacío o largo junto al campo; recortado; 255 emoji caben", async () => {
    const { backend } = await setup();
    await startNewEntry();
    const title = () => screen.getByLabelText("Título");
    const save = () => {
      fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    };

    for (const empty of ["   ", " ".repeat(300)]) {
      fireEvent.change(title(), { target: { value: empty } });
      save();
      expect(await screen.findByText("Ponle un título para poder encontrarla después.")).toBeDefined();
      expect(title().getAttribute("aria-invalid")).toBe("true");
      expect(title().getAttribute("aria-describedby")).toContain("entry-dialog-title-error");
      expect(document.activeElement).toBe(title());
    }

    fireEvent.change(title(), { target: { value: "a".repeat(256) } });
    save();
    expect(await screen.findByText(/El título admite hasta 255 caracteres/)).toBeDefined();
    expect(backend.count("entry_create")).toBe(0);

    fireEvent.change(title(), { target: { value: "\u{1F511}".repeat(255) } });
    save();
    expect(await screen.findByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    fireEvent.change(await screen.findByLabelText("Título"), { target: { value: "  Banco  " } });
    save();
    await waitFor(() => {
      expect(backend.count("entry_create")).toBe(2);
    });
    const created = backend.calls.filter((call) => call.command === "entry_create").at(-1);
    expect(created?.args?.title).toBe("  Banco  ");
    // The core trims it: the new entry is the chosen row of the list, with the title as the core left it.
    await waitFor(() => {
      const rows = within(screen.getByRole("table")).getAllByRole("button", { name: "Banco" });
      expect(rows.filter((row) => row.getAttribute("aria-current") === "true")).toHaveLength(1);
    });
  });

  it("RF-91: una etiqueta con ; o , se rechaza junto al campo sin llamar al núcleo", async () => {
    const { backend } = await setup();
    await startNewEntry();

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Etiquetas" } });
    fireEvent.change(tagsField(), { target: { value: "uno, dos" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText(ERROR_MESSAGES.EtiquetaInvalida)).toBeDefined();
    expect(tagsField().getAttribute("aria-invalid")).toBe("true");
    expect(backend.count("entry_create")).toBe(0);
  });

  it("RF-40: un carácter de control se rechaza junto al campo; si el núcleo rechaza, el error va a su campo", async () => {
    const { backend } = await setup();
    await startNewEntry();

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Bien" } });
    fireEvent.change(screen.getByLabelText("Usuario o correo"), { target: { value: "ana\u0007" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    expect(await screen.findByText(ERROR_MESSAGES.CaracterNoPermitido)).toBeDefined();
    expect(screen.getByLabelText("Usuario o correo").getAttribute("aria-invalid")).toBe("true");
    expect(backend.count("entry_create")).toBe(0);

    fireEvent.change(screen.getByLabelText("Usuario o correo"), { target: { value: "ana" } });
    backend.rejectOnce("entry_create", { code: "TituloVacio", message: "core text" });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    expect(await screen.findByText("Ponle un título para poder encontrarla después.")).toBeDefined();
    expect(screen.getByLabelText("Título").getAttribute("aria-invalid")).toBe("true");
  });

  it("RF-23: error — si no se pudo guardar lo dice y pide escribir otra vez la contraseña", async () => {
    const { backend } = await setup();
    await startNewEntry();
    backend.rejectOnce("entry_create", { code: "GuardadoFallido", message: "core text" });

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Correo" } });
    typePassword("secreta");
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(screen.getByText(/vuelve a escribirla o genérala/)).toBeDefined();
    expect(passwordField().value).toBe("");
    expect(screen.getByLabelText<HTMLInputElement>("Título").value).toBe("Correo");
  });

  it("RF-70: muestra la fortaleza de la contraseña escrita", async () => {
    const { backend } = await setup();
    await startNewEntry();

    typePassword("Abcdef12!");

    expect(await screen.findByText(/^Fortaleza: muy buena$/)).toBeDefined();
    expect(lastArgs(backend, "password_strength")).toEqual({ password: "Abcdef12!" });
    expect(passwordField().type).toBe("password");
    expect(passwordField().getAttribute("autocomplete")).toBe("new-password");
    expect(passwordField().getAttribute("spellcheck")).toBe("false");
  });

  it("RF-45: ejemplo 8 — cambiar solo el título envía solo el título y la contraseña sigue igual", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    expect(passwordField().value).toBe("");
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "GitHub personal" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", title: "GitHub personal" });
    expect(backend.entry("entry-github")?.password).toBe(PASSWORD);
    expect(backend.entry("entry-github")?.history).toBe(1);
    const modified = backend.entry("entry-github")?.modified;
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Editar" }));

    // Confirm again without changing anything: neither the date nor the history move.
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("No había cambios que guardar.")).toBeDefined();
    expect(backend.count("entry_update")).toBe(1);
    expect(backend.entry("entry-github")?.history).toBe(1);
    expect(backend.entry("entry-github")?.modified).toBe(modified);
  });

  it("RF-45: escribir una contraseña nueva la envía; un campo protegido no se precarga ni se borra", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "VPN");

    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    expect(screen.getByLabelText<HTMLInputElement>("Usuario o correo").value).toBe("");
    expect(screen.getByText(/Este campo está protegido en la bóveda/)).toBeDefined();
    typePassword("nueva-clave");
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-vpn", password: "nueva-clave" });
    expect(backend.entry("entry-vpn")?.username).toBeNull();
    expect(backend.entry("entry-vpn")?.password).toBe("nueva-clave");
  });

  it("RF-45: un campo vaciado a propósito viaja vacío y reemplaza el valor", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "" } });
    fireEvent.change(tagsField(), { target: { value: "código" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", notes: "", tags: ["código"] });
  });
});

describe("P5 Entrada — mover y borrar", () => {
  it("RF-46: mover abre un diálogo con el destino y devuelve el foco al cancelar", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    // The order is in the menu of the entry: what opened the dialog, and gets the focus back, is the menu button.
    const opener = moreButton();

    chooseOrder("Mover a otro grupo…");
    const dialog = await screen.findByRole("dialog", { name: "Mover «GitHub»" });
    const options = within(dialog).getAllByRole("option").map((option) => option.textContent);
    expect(options).not.toContain("Cuentas › Trabajo");
    expect(options.some((label) => label.includes("Papelera"))).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    // Only the window of the entry is left, and the focus is back inside it.
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);
    expect(document.activeElement).toBe(opener);

    chooseOrder("Mover a otro grupo…");
    const again = await screen.findByRole("dialog", { name: "Mover «GitHub»" });
    fireEvent.change(within(again).getByLabelText("Grupo de destino"), { target: { value: "group-personal" } });
    fireEvent.click(within(again).getByRole("button", { name: "Mover" }));

    expect(await screen.findByText("«GitHub» se movió a Personal.")).toBeDefined();
    expect(lastArgs(backend, "entry_move")).toEqual({ id: "entry-github", group_id: "group-personal" });
    expect(backend.entry("entry-github")?.history).toBe(0);
    await waitFor(() => {
      expect(within(screen.getByRole("table")).queryByRole("button", { name: "GitHub" })).toBeNull();
    });
  });

  it("RF-47: borrar fuera de la papelera la manda a la papelera sin diálogo y avisa", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    chooseOrder("Enviar a la papelera");

    expect(await screen.findByText(/«GitHub» se movió a la papelera/)).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(backend.entry("entry-github")?.groupId).toBe("group-bin");
    // The window went with its entry: the focus goes to the title of the list.
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Entradas de Trabajo" }));
    });
    expect(await screen.findByRole("button", { name: "Papelera de reciclaje (papelera) · 2 entradas" })).toBeDefined();
  });

  it("RF-47: si la bóveda no tiene papelera, el primer borrado la crea", async () => {
    const { backend } = await setup({
      groups: { id: "r", name: "Cuentas", kind: "root", children: [{ id: "g", name: "Personal" }] },
      entries: [{ id: "e", groupId: "g", title: "Única" }],
    });
    await openEntry("Personal", "Única");

    chooseOrder("Enviar a la papelera");

    expect(await screen.findByRole("button", { name: "Papelera de reciclaje (papelera) · 1 entrada" })).toBeDefined();
    expect(backend.groupTrail(backend.entry("e")?.groupId ?? "")).toEqual(["Cuentas", "Papelera de reciclaje"]);
  });

  it("RF-48: dentro de la papelera pide confirmación, dice que es definitivo y luego la elimina", async () => {
    const { backend } = await setup();
    await openEntry("Papelera de reciclaje", "Canción borrada");

    chooseOrder("Borrar definitivamente…");
    const dialog = await screen.findByRole("dialog", { name: "Borrar «Canción borrada» de forma definitiva" });
    expect(within(dialog).getByText(/no se puede deshacer/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.entry("entry-old")).toBeDefined();

    chooseOrder("Borrar definitivamente…");
    fireEvent.click(
      within(
        await screen.findByRole("dialog", { name: "Borrar «Canción borrada» de forma definitiva" }),
      ).getByRole("button", { name: "Borrar definitivamente" }),
    );

    expect(await screen.findByText("Se borró «Canción borrada» de forma definitiva.")).toBeDefined();
    expect(backend.entry("entry-old")).toBeUndefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Entradas de Papelera de reciclaje" }));
    });
    expect(await screen.findByText("La papelera está vacía.")).toBeDefined();
  });
});

describe("P5 Entrada — la tarjeta de campos", () => {
  it("RF-43: una fila por campo, en orden: usuario, contraseña, sitio, campos propios, etiquetas, adjuntos y las notas al final", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const labels = [...detailZone().querySelectorAll(".entry-card > .card-row > dt")].map((dt) => dt.textContent);
    expect(labels).toEqual(["Usuario", "Contraseña", "Sitio", "PIN", "Recuperación", "Etiquetas", "Adjuntos", "Notas"]);
    // The tags and the attachments are rows without buttons; the notes too.
    for (const label of ["Etiquetas", "Adjuntos", "Notas"]) expect(within(rowOf(label)).queryAllByRole("button")).toEqual([]);
    expect(within(rowOf("Etiquetas")).getAllByRole("listitem").map((tag) => tag.textContent)).toEqual(["código", "trabajo"]);
    expect(rowOf("Adjuntos").textContent).toContain("codigos.txt (120 bytes)");
    expect(within(rowOf("Notas")).getByText("cuenta del trabajo").className).toBe("card-notes");
  });

  it("RF-43: los botones de cada fila son solo de icono, con nombre y ayuda al pasar el cursor, y van en una columna al final", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const names = (label: string) =>
      within(rowOf(label)).getAllByRole("button").map((button) => button.getAttribute("aria-label"));
    expect(names("Usuario")).toEqual(["Copiar usuario"]);
    expect(names("Contraseña")).toEqual(["Mostrar contraseña", "Copiar contraseña"]);
    expect(names("Sitio")).toEqual(["Abrir el sitio en el navegador", "Copiar dirección del sitio"]);
    expect(names("PIN")).toEqual(["Mostrar «PIN»", "Copiar «PIN»"]);
    expect(names("Recuperación")).toEqual(["Copiar «Recuperación»"]);
    const buttons = [...detailZone().querySelectorAll<HTMLElement>(".entry-card button")];
    for (const button of buttons) {
      // No text of its own: the name is the `aria-label`, and the same words are the help under the pointer.
      expect(button.textContent).toBe("");
      expect(button.getAttribute("title")).toBe(button.getAttribute("aria-label"));
      // Every button sits in the actions column that closes its row.
      expect(button.parentElement?.className).toBe("card-actions");
      expect(button.parentElement?.parentElement?.lastElementChild).toBe(button.parentElement);
    }
  });

  it("RF-71: «Copiar contraseña» es el único botón de icono con fondo", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const plated = [...detailZone().querySelectorAll(".entry-card button.btn-primary")];
    expect(plated.map((button) => button.getAttribute("aria-label"))).toEqual(["Copiar contraseña"]);
    // Showing a value does not give any other button a plate.
    fireEvent.click(screen.getByRole("button", { name: "Mostrar «PIN»" }));
    await screen.findByText("4321");
    expect(detailZone().querySelectorAll(".entry-card button.btn-primary")).toHaveLength(1);
  });

  it("RF-44: RF-76: un valor mostrado va en un elemento `.secret`, oculto es la barra de ancho fijo, y nunca entra a un atributo", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const hidden = rowOf("Contraseña").querySelector(".secret");
    expect(hidden?.hasAttribute("data-masked")).toBe(true);
    // The bar says nothing of the value: the same element, with the same text, whatever its length.
    expect(hidden?.textContent).toBe("(oculto)");
    expect(rowOf("PIN").querySelector(".secret")?.outerHTML).toBe(hidden?.outerHTML);

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    const shownValue = await screen.findByText(PASSWORD);
    expect(shownValue.className).toBe("secret");
    expect(shownValue.hasAttribute("data-masked")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Mostrar «PIN»" }));
    expect((await screen.findByText("4321")).className).toBe("secret");

    // Neither value is in any attribute of the page: no `title`, no `aria-label`, no `data-*`.
    for (const element of document.querySelectorAll("*")) {
      for (const attribute of element.attributes) {
        expect(attribute.value).not.toContain(PASSWORD);
        expect(attribute.value).not.toContain("4321");
      }
    }
  });

  it("RF-44: la nota de los 20 segundos se ve solo mientras hay algo mostrado", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");
    const hint = () => screen.queryByText("Lo que muestres se oculta solo a los 20 segundos.");

    expect(hint()).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    await screen.findByText(PASSWORD);
    expect(hint()).not.toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Ocultar contraseña" }));
    expect(hint()).toBeNull();
  });

  it("RF-43: sin usuario, sin contraseña, sin sitio o sin notas lo dice y no ofrece botones que no tengan sentido", async () => {
    await setup({ entries: [{ id: "x", groupId: "group-personal", title: "Vacía", username: "", url: "", notes: "" }] });
    await openEntry("Personal", "Vacía");

    expect(rowOf("Usuario").textContent).toBe("UsuarioSin usuario");
    expect(rowOf("Contraseña").textContent).toBe("ContraseñaSin contraseña");
    expect(rowOf("Sitio").textContent).toBe("SitioSin sitio");
    expect(rowOf("Notas").textContent).toBe("NotasSin notas");
    expect(detailZone().querySelectorAll(".entry-card button")).toHaveLength(0);
    // Without tags or attachments there is no row for them.
    expect(within(detailZone()).queryByText("Etiquetas")).toBeNull();
    expect(within(detailZone()).queryByText("Adjuntos")).toBeNull();
  });

  it("RF-43: bajo la tarjeta dice cuándo se creó y cuándo se modificó, en una línea, con la hora en el `title`", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const dates = detailZone().querySelector(".entry-dates");
    expect(dates?.textContent).toBe("Creada el 1 oct 2026 · Modificada el 1 nov 2026");
    expect(dates?.querySelector('time[datetime="2026-11-01T08:00:00Z"]')?.getAttribute("title")).toMatch(/2026/);
  });
});

describe("P5 Entrada — el aviso de copiado", () => {
  it("RF-74: copiar la dirección del sitio va por el núcleo y avisa «Dirección copiada», sin cuenta", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar dirección del sitio" }));

    expect(await screen.findByText("Dirección copiada")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "URL" });
    expect(backend.clipboard()).toBe("https://github.com/login");
  });

  it("RF-74: un campo propio no protegido se copia sin cuenta; uno protegido, con la cuenta del ajuste real", async () => {
    const { backend } = await setup({ clipboardSeconds: 45 });
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar «Recuperación»" }));
    expect(await screen.findByText("«Recuperación» copiado")).toBeDefined();
    expect(backend.clipboard()).toBe("pregunta: mascota");

    fireEvent.click(screen.getByRole("button", { name: "Copiar «PIN»" }));
    expect(await screen.findByText("«PIN» copiado. Se borra del portapapeles en 45 s")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "PIN" });
    // The last copy is the one the notice speaks of.
    expect(screen.queryByText("«Recuperación» copiado")).toBeNull();
    expect(bodyContains("4321")).toBe(false);
  });

  it("RF-74: el aviso es un `role=status` al pie del detalle, con el visto bueno, y existe antes de tener algo que decir", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const zone = detailZone().querySelector(".toast-zone");
    expect(zone?.getAttribute("role")).toBe("status");
    expect(zone?.childElementCount).toBe(0);
    // The last thing in the detail zone: the stylesheet pins it to the foot.
    expect(detailZone().lastElementChild).toBe(zone);

    fireEvent.click(screen.getByRole("button", { name: "Copiar usuario" }));
    const notice = await screen.findByText("Usuario copiado");
    expect(notice.className).toBe("toast");
    expect(notice.parentElement).toBe(zone);
    expect(notice.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("RF-74: la cuenta sigue al elegir otra entrada mientras el portapapeles tenga el valor; un aviso sin cuenta se va con su entrada", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar contraseña" }));
    await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s");
    // With the window closed the notice goes on at the foot of the list.
    closeWindow();
    expect(copyNotice()).toBe("Contraseña copiada. Se borra del portapapeles en 20 s");
    await openRow("VPN");
    await act(() => vi.advanceTimersByTimeAsync(3_000));
    expect(copyNotice()).toBe("Contraseña copiada. Se borra del portapapeles en 17 s");

    // Once the time is over it says so, and choosing another entry takes that away.
    await act(() => vi.advanceTimersByTimeAsync(17_000));
    expect(copyNotice()).toMatch(/limpió el portapapeles/);
    closeWindow();
    await openRow("GitHub");
    expect(copyNotice()).toBe("");

    fireEvent.click(await screen.findByRole("button", { name: "Copiar usuario" }));
    await screen.findByText("Usuario copiado");
    closeWindow();
    await openRow("VPN");
    expect(copyNotice()).toBe("");
  });
});

describe("P5 Entrada — cabecera y menú de la entrada", () => {
  const items = () => screen.getAllByRole("menuitem").map((item) => item.textContent);

  it("RF-43: la cabecera lleva el círculo con la inicial del título sobre el color de su grupo, y el grupo debajo", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const head = detailZone().querySelector(".entry-head");
    const circle = head?.querySelector(".bullet");
    expect(circle?.getAttribute("data-initial")).toBe("G");
    // Trabajo is the second group of the rail: line 2. The dot of the group line has the same colour.
    expect(circle?.getAttribute("data-line")).toBe("2");
    expect(circle?.getAttribute("aria-hidden")).toBe("true");
    const group = head?.querySelector(".entry-group");
    expect(group?.textContent).toBe("Grupo: Trabajo");
    expect(group?.querySelector(".group-dot")?.getAttribute("data-line")).toBe("2");
    // Only the group of the entry is read; the whole way from the root is in the `title`.
    expect(group?.getAttribute("title")).toBe("Cuentas › Trabajo");
  });

  it("RF-43: un subgrupo se nombra solo y lleva el color de su grupo de primer nivel", async () => {
    await setup();
    await openEntry("Clientes", "Banana");

    const group = detailZone().querySelector(".entry-group");
    expect(group?.textContent).toBe("Grupo: Clientes");
    expect(group?.getAttribute("title")).toBe("Cuentas › Trabajo › Clientes");
    expect(detailZone().querySelector(".entry-head .bullet")?.getAttribute("data-line")).toBe("2");
  });

  it("RF-46: RF-47: «Editar» queda a la vista y mover y enviar a la papelera van en el menú, lo destructivo al final", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    const detail = detailZone();
    expect(within(detail).getByRole("button", { name: "Editar" })).toBeDefined();
    // Nothing destructive is in sight until the menu opens, and the old row of buttons at the foot is gone.
    expect(within(detail).queryByRole("button", { name: /papelera/i })).toBeNull();
    expect(within(detail).queryByRole("button", { name: /^Mover/ })).toBeNull();
    expect(screen.queryByRole("menu")).toBeNull();
    const more = moreButton();
    expect(more.getAttribute("aria-haspopup")).toBe("menu");
    expect(more.getAttribute("aria-expanded")).toBe("false");
    expect(more.getAttribute("title")).toBe("Más acciones de la entrada");

    fireEvent.click(more);

    expect(more.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("menu", { name: "Acciones de la entrada" })).toBeDefined();
    // "Duplicar" is not offered: the core has no such command.
    // It acts at once, without asking: no ellipsis (RF-47).
    expect(items()).toEqual(["Mover a otro grupo…", "Enviar a la papelera"]);
    expect(screen.getByRole("menuitem", { name: "Enviar a la papelera" }).className).toBe("menu-danger");
    expect(screen.getByRole("menuitem", { name: "Mover a otro grupo…" }).className).toBe("");
  });

  it("RF-48: en la papelera el menú ofrece mover (así se restaura) y borrar definitivamente", async () => {
    await setup();
    await openEntry("Papelera de reciclaje", "Canción borrada");

    fireEvent.click(moreButton());

    expect(items()).toEqual(["Mover a otro grupo…", "Borrar definitivamente…"]);
    expect(screen.getByRole("menuitem", { name: "Borrar definitivamente…" }).className).toBe("menu-danger");
  });

  it("RF-46: el menú de la entrada se recorre con las flechas, Inicio y Fin, y abre con flecha abajo", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");
    const more = moreButton();

    more.focus();
    fireEvent.keyDown(more, { key: "ArrowDown" });
    const [move, bin] = screen.getAllByRole("menuitem");
    if (move === undefined || bin === undefined) throw new Error("expected two orders");
    // Opening puts the focus on the first order.
    expect(document.activeElement).toBe(move);

    fireEvent.keyDown(move, { key: "ArrowDown" });
    expect(document.activeElement).toBe(bin);
    fireEvent.keyDown(bin, { key: "ArrowDown" });
    expect(document.activeElement).toBe(move);
    fireEvent.keyDown(move, { key: "ArrowUp" });
    expect(document.activeElement).toBe(bin);
    fireEvent.keyDown(bin, { key: "Home" });
    expect(document.activeElement).toBe(move);
    fireEvent.keyDown(move, { key: "End" });
    expect(document.activeElement).toBe(bin);
  });

  it("RF-46: Escape cierra el menú de la entrada sin ordenar nada y devuelve el foco a su botón; un clic fuera también lo cierra", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    const more = moreButton();

    fireEvent.click(more);
    fireEvent.keyDown(screen.getAllByRole("menuitem")[0] ?? more, { key: "Escape" });

    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(more);
    expect(more.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(more);
    // The menu is inside the modal window, or it would be inert behind it.
    expect(detailZone().contains(screen.getByRole("menu"))).toBe(true);
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();

    // The button also closes the menu it opened.
    fireEvent.click(more);
    fireEvent.click(more);
    expect(screen.queryByRole("menu")).toBeNull();
    expect(backend.count("entry_move")).toBe(0);
    expect(backend.count("entry_delete")).toBe(0);
    // No dialog of an order opened: only the window of the entry is there.
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);
  });

  it("RF-47: elegir «Enviar a la papelera» en el menú lo cierra y actúa de una vez, sin diálogo", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    chooseOrder("Enviar a la papelera");

    expect(screen.queryByRole("menu")).toBeNull();
    // No dialog asks first: only the window of the entry is there, and it closes once the entry is in the bin.
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);
    await waitFor(() => {
      expect(backend.entry("entry-github")?.groupId).toBe("group-bin");
    });
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("RF-14: en solo lectura no hay «Editar» ni menú de la entrada, y lo dice", async () => {
    await setup({ status: { ...OPEN, read_only: true } });
    await openGroup("Trabajo");
    await openRow("GitHub");

    const detail = detailZone();
    expect(await within(detail).findByText(/Solo lectura: esta entrada no se puede editar, mover ni borrar/)).toBeDefined();
    expect(within(detail).queryByRole("button", { name: "Editar" })).toBeNull();
    expect(within(detail).queryByRole("button", { name: "Más acciones de la entrada" })).toBeNull();
    // Looking and copying stay.
    expect(within(detail).getByRole("button", { name: "Copiar contraseña" })).toBeDefined();
  });
});

describe("P5 Entrada — bloqueada", () => {
  it("RF-32: tras vault-locked no queda en el DOM ningún valor mostrado, ni uno que llega tarde", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(await screen.findByText(PASSWORD)).toBeDefined();
    // A reveal still on its way when the lock arrives.
    const release = backend.hold("entry_reveal");
    fireEvent.click(screen.getByRole("button", { name: "Mostrar «PIN»" }));
    fireEvent.click(screen.getByRole("button", { name: "Copiar contraseña" }));
    expect(await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s")).toBeDefined();

    act(() => {
      backend.lockFromCore();
    });
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    for (const value of [PASSWORD, "4321", "ana@example.com", "copiada", "portapapeles"]) {
      expect(bodyContains(value)).toBe(false);
    }
    expect(backend.clipboard()).toBeNull();
  });

  it("RF-32: tras vault-locked no queda en el DOM ningún valor generado ni escrito a medias", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");
    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    await screen.findByRole("dialog", { name: "Nueva entrada" });
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "borrador-a-medias" } });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "escrita-sin-guardar" } });
    fireEvent.click(screen.getByRole("button", { name: "Generar" }));
    await waitFor(() => {
      expect(passwordField().value).not.toBe("");
    });
    const generated = passwordField().value;
    // The draft is there to lose: the three values are on screen before the lock.
    for (const value of ["borrador-a-medias", "escrita-sin-guardar", generated]) {
      expect(bodyContains(value)).toBe(true);
    }

    act(() => {
      backend.lockFromCore();
    });

    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    for (const value of ["borrador-a-medias", "escrita-sin-guardar", generated]) {
      expect(bodyContains(value)).toBe(false);
    }
  });
});

describe("P5 Entrada — el aviso espera al disco", () => {
  const emitSave = (backend: { emit: (event: string, payload?: unknown) => void }, state: string) => {
    act(() => {
      backend.emit("vault-save-state", state);
    });
  };

  async function editGitHubTitle() {
    await openEntry("Trabajo", "GitHub");
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "GitHub personal" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
  }

  it("RF-21: al editar no dice «Entrada guardada.» hasta que el núcleo confirma `saved`", async () => {
    const { backend } = await setup();
    await editGitHubTitle();

    expect(await screen.findByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeDefined();
    expect(screen.queryByText("Entrada guardada.")).toBeNull();
    emitSave(backend, "saving");
    expect(screen.queryByText("Entrada guardada.")).toBeNull();
    emitSave(backend, "saved");
    expect(screen.getByText("Entrada guardada.")).toBeDefined();
    expect(screen.queryByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeNull();
  });

  it("RF-21: RF-23: un `saved` anterior no cuenta, y si el guardado falla manda el aviso de guardado", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    emitSave(backend, "saved");
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "GitHub personal" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeDefined();
    expect(screen.queryByText("Entrada guardada.")).toBeNull();
    emitSave(backend, "failed");
    expect(screen.getByRole("heading", { name: "No se pudo guardar: tus cambios siguen en memoria" })).toBeDefined();
    expect(screen.queryByText("Cambios aplicados. Guardando la entrada en el archivo…")).toBeNull();
    expect(screen.queryByText("Entrada guardada.")).toBeNull();
  });

  it("RF-21: al crear dice que se está guardando y pasa a «Entrada guardada.» con `saved`", async () => {
    const { backend } = await setup();
    await startNewEntry();
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Correo" } });
    typePassword("clave larga 123");
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();
    expect(screen.queryByText("Entrada guardada.")).toBeNull();
    emitSave(backend, "saved");
    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
  });
});

describe("P4 Bóveda — buscar y copiar con teclado", () => {
  const search = () => screen.getByRole("searchbox", { name: "Buscar en la bóveda" });
  const rows = () => within(screen.getByRole("table")).getAllByRole("button");

  it("RF-59: Ctrl+K, escribir y Enter elige el primer resultado y deja el foco en su «Copiar contraseña»; otro Enter copia", async () => {
    const { backend } = await setup();
    // The shortcut listener is attached in an effect: press again until it is there (focusing twice is harmless).
    await waitFor(() => {
      fireEvent.keyDown(window, { key: "k", ctrlKey: true });
      expect(document.activeElement).toBe(search());
    });
    fireEvent.change(search(), { target: { value: "GitHub" } });
    // Enter right after typing, while the search is still on its way: it opens the first result when it lands.
    fireEvent.keyDown(search(), { key: "Enter" });

    const copy = await screen.findByRole("button", { name: "Copiar contraseña de GitHub" });
    await waitFor(() => {
      expect(document.activeElement).toBe(copy);
    });
    // The first result is the chosen row, and its window does not open by itself.
    const row = within(screen.getByRole("table")).getByRole("button", { name: "GitHub" });
    expect(row.getAttribute("aria-current")).toBe("true");
    expect(row.closest("tr")?.contains(copy)).toBe(true);
    expect(screen.queryByRole("dialog")).toBeNull();

    // Enter on a button is its click: the core copies, and the list says so with the countdown.
    fireEvent.click(copy);
    expect(await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "Password" });
    expect(bodyContains(PASSWORD)).toBe(false);
  });

  it("RF-59: flecha abajo va a la lista, las flechas recorren las filas, Enter abre y Escape vuelve", async () => {
    await setup();
    await openGroup("Trabajo");
    search().focus();
    fireEvent.keyDown(search(), { key: "ArrowDown" });
    const [first, second] = rows();
    if (first === undefined || second === undefined) throw new Error("expected two rows");
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(document.activeElement).toBe(second);
    fireEvent.keyDown(second, { key: "ArrowUp" });
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: "Escape" });
    expect(document.activeElement).toBe(search());

    fireEvent.keyDown(search(), { key: "ArrowDown" });
    fireEvent.keyDown(rows()[0] ?? search(), { key: "Enter" });
    const copy = await within(detailZone()).findByRole("button", { name: "Copiar contraseña" });
    await waitFor(() => {
      expect(document.activeElement).toBe(copy);
    });
  });

  it("RF-59: abrir con el ratón no mueve el foco al detalle", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");
    expect(document.activeElement).not.toBe(within(detailZone()).getByRole("button", { name: "Copiar contraseña" }));
  });
});

describe("P5 Entrada — ventana mínima", () => {
  it("RF-108: «Cerrar» vuelve a la lista con la fila elegida y el foco en ella; al editar, el formulario queda sobre la ventana y cancelar vuelve a ella", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(within(detailZone()).getByRole("button", { name: "Editar" }));
    const editing = await screen.findByRole<HTMLDialogElement>("dialog", { name: "Editar entrada" });
    // The edit is a modal dialog over the window: cancelling it goes back to the window of the entry.
    expect(editing.open).toBe(true);
    fireEvent.click(within(editing).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog", { name: "Editar entrada" })).toBeNull();
    expect(within(detailZone()).getByRole("heading", { name: "GitHub" })).toBeDefined();

    closeWindow();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.queryByText("Elige una entrada para ver su detalle.")).toBeNull();
    const row = within(screen.getByRole("table")).getByRole("button", { name: "GitHub" });
    expect(row.getAttribute("aria-current")).toBe("true");
    expect(document.activeElement).toBe(row);
  });
});

describe("P5 Entrada — diálogos sobre la ventana de la entrada", () => {
  it("RF-108: Esc en el diálogo de mover o en el formulario de editar cierra solo ese diálogo: la ventana de la entrada sigue abierta", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    // Escape reaches a modal dialog as its `cancel` event, which React carries up to the window it is mounted in.
    chooseOrder("Mover a otro grupo…");
    const move = await screen.findByRole("dialog", { name: "Mover «GitHub»" });
    expect(detailZone().contains(move)).toBe(true);
    fireEvent(move, new Event("cancel", { cancelable: true }));
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);

    fireEvent.click(within(detailZone()).getByRole("button", { name: "Editar" }));
    const editing = await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent(editing, new Event("cancel", { cancelable: true }));
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);
    expect(within(detailZone()).getByRole("heading", { name: "GitHub" })).toBeDefined();
  });

  it("RF-108: la ventana se abre con el foco en «Cerrar», y ese botón sigue siendo el mismo cuando llega la entrada", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");
    const release = backend.hold("entry_get");
    await openRow("GitHub");
    const close = within(detailZone()).getByRole("button", { name: "Cerrar" });

    await act(async () => {
      release();
      await Promise.resolve();
    });
    await within(detailZone()).findByRole("button", { name: "Editar" });

    // Replaced by another element, the focus the browser gave it would be left on nothing behind a modal window.
    expect(within(detailZone()).getByRole("button", { name: "Cerrar" })).toBe(close);
  });
});

describe("P5 Entrada — el diálogo sobre la bóveda", () => {
  it("RF-100: «Nueva entrada» y «Editar» abren un diálogo sobre la bóveda: la lista y la ventana de la entrada quedan detrás y el panel no muestra formularios", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");

    for (const [button, name] of [
      ["Nueva entrada", "Nueva entrada"],
      ["Editar", "Editar entrada"],
    ] as const) {
      // "Nueva entrada" is in the list, which a modal window leaves out of reach: the window closes first.
      if (button === "Nueva entrada") closeWindow();
      else await openRow("GitHub");
      fireEvent.click(screen.getByRole("button", { name: button }));
      const dialog = await screen.findByRole<HTMLDialogElement>("dialog", { name });
      expect(dialog.open).toBe(true);
      expect(within(dialog).getByText("En el grupo Trabajo")).toBeDefined();
      expect(document.activeElement).toBe(within(dialog).getByLabelText("Título"));
      // Behind it, as they were: the chosen row and, when editing, the window of its entry.
      const row = within(screen.getByRole("table")).getByRole("button", { name: "GitHub" });
      expect(row.getAttribute("aria-current")).toBe("true");
      if (button === "Editar") {
        expect(within(detailZone()).getByRole("heading", { name: "GitHub" })).toBeDefined();
        expect(within(detailZone()).getByRole("button", { name: "Copiar contraseña" })).toBeDefined();
        const forms = [...detailZone().querySelectorAll("form")];
        expect(forms.every((form) => form.closest("dialog") === dialog)).toBe(true);
      }

      fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
      expect(screen.queryByRole("dialog", { name })).toBeNull();
    }
  });

  it("RF-100: al guardar una entrada nueva el diálogo se cierra y la entrada queda elegida en la lista, con el foco en su «Copiar contraseña»; la ventana no se abre sola", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    closeWindow();
    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    const dialog = await screen.findByRole("dialog", { name: "Nueva entrada" });
    fireEvent.change(within(dialog).getByLabelText("Título"), { target: { value: "Gitea" } });
    typePassword("clave-de-gitea");

    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      const row = within(screen.getByRole("table")).getByRole("button", { name: "Gitea" });
      expect(row.getAttribute("aria-current")).toBe("true");
    });
    // RF-106: the new entry is chosen in the list; its window does not open by itself.
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(within(screen.getByRole("table")).getByRole("button", { name: "GitHub" }).getAttribute("aria-current")).toBeNull();
    expect(lastArgs(backend, "entry_create")).toMatchObject({ group_id: "group-work", title: "Gitea" });
    expect(screen.getByText("Entrada creada. Guardándola en el archivo…")).toBeDefined();
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Copiar contraseña de Gitea" }));
    });
    expect(bodyContains("clave-de-gitea")).toBe(false);
  });

  it("RF-100: al guardar una edición el diálogo se cierra, la entrada sigue elegida con sus datos nuevos y el foco vuelve a «Editar»", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    const dialog = await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.change(within(dialog).getByLabelText("Título"), { target: { value: "GitHub del equipo" } });

    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar entrada" }));

    // RF-108: saving goes back to the window of the entry, with the new data.
    expect(await within(detailZone()).findByRole("heading", { name: "GitHub del equipo" })).toBeDefined();
    expect(screen.getAllByRole("dialog")).toEqual([detailZone()]);
    expect(document.activeElement).toBe(within(detailZone()).getByRole("button", { name: "Editar" }));
    await waitFor(() => {
      const row = within(screen.getByRole("table")).getByRole("button", { name: "GitHub del equipo" });
      expect(row.getAttribute("aria-current")).toBe("true");
    });
  });

  it("RF-100: con el diálogo abierto, Ctrl+K y Ctrl+F no llevan el foco a la búsqueda de detrás", async () => {
    await setup();
    await openGroup("Personal");
    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    const dialog = await screen.findByRole("dialog", { name: "Nueva entrada" });
    const title = within(dialog).getByLabelText("Título");

    for (const key of ["k", "f"]) {
      // The key is still taken from the browser (no "find in page"), but the focus stays in the dialog.
      expect(fireEvent.keyDown(window, { key, ctrlKey: true })).toBe(false);
      expect(document.activeElement).toBe(title);
    }

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(document.activeElement).toBe(document.getElementById("vault-search"));
  });

  it("RF-32: al bloquear a mano con el diálogo de editar abierto tampoco queda nada del borrador en el DOM", async () => {
    await setup();
    await openEntry("Trabajo", "GitHub");
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    const dialog = await screen.findByRole("dialog", { name: "Editar entrada" });
    fireEvent.change(within(dialog).getByLabelText("Título"), { target: { value: "borrador-a-medias" } });
    typePassword("escrita-sin-guardar");
    expect(bodyContains("escrita-sin-guardar")).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: /^Bloquear/ }));

    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    for (const value of ["borrador-a-medias", "escrita-sin-guardar", PASSWORD]) expect(bodyContains(value)).toBe(false);
  });
});
