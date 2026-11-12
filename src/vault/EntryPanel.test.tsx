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

const detailZone = () => screen.getByRole("region", { name: "Detalle" });
const lastArgs = (backend: { calls: readonly { command: string; args: unknown }[] }, command: string) =>
  backend.calls.filter((call) => call.command === command).at(-1)?.args;

async function openGroup(name: string) {
  fireEvent.click(await screen.findByRole("button", { name: new RegExp(`^${name} `) }));
  await screen.findByRole("heading", { name: `Entradas de ${name}` });
  await waitFor(() => {
    expect(screen.queryByText("Cargando las entradas…")).toBeNull();
  });
}

async function openEntry(group: string, title: string) {
  await openGroup(group);
  fireEvent.click(within(screen.getByRole("table")).getByRole("button", { name: title }));
  await within(detailZone()).findByRole("button", { name: "Editar" });
}

async function startNewEntry(group = "Personal") {
  await openGroup(group);
  fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
  return screen.findByRole("form", { name: `Nueva entrada en ${group}` });
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

  it("RF-43: un campo estándar protegido (null) se ve «(protegido)» y solo se muestra a pedido", async () => {
    await setup();

    await openEntry("Trabajo", "VPN");

    const detail = detailZone();
    expect(within(detail).getByText("(protegido)")).toBeDefined();
    expect(within(detail).getByRole("button", { name: "Mostrar usuario" })).toBeDefined();
    expect(within(detail).queryByText("usuario-vpn")).toBeNull();
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
    fireEvent.click(within(screen.getByRole("table")).getByRole("button", { name: "VPN" }));
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

    expect(await screen.findByText("Copiado: se borra en 20 s")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "Password" });
    expect(backend.clipboard()).toBe(PASSWORD);
    expect(bodyContains(PASSWORD)).toBe(false);
    expect(backend.count("entry_reveal")).toBe(0);

    await act(() => vi.advanceTimersByTimeAsync(5_000));
    expect(screen.getByText("Copiado: se borra en 15 s")).toBeDefined();
    await act(() => vi.advanceTimersByTimeAsync(15_000));
    expect(screen.getByText(/limpió el portapapeles/)).toBeDefined();
  });

  it("RF-74: copiar el usuario también va por entry_copy y usa los segundos que da el núcleo", async () => {
    const { backend } = await setup({ clipboardSeconds: 45 });
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Copiar usuario" }));

    expect(await screen.findByText("Copiado: se borra en 45 s")).toBeDefined();
    expect(lastArgs(backend, "entry_copy")).toEqual({ id: "entry-github", field: "UserName" });
    expect(backend.clipboard()).toBe("ana@example.com");
  });

  it("RF-83: error — si copiar falla lo dice sin el texto del núcleo", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    backend.rejectOnce("entry_copy", { code: "Bloqueada", message: "core text" });

    fireEvent.click(screen.getByRole("button", { name: "Copiar contraseña" }));

    expect(await screen.findByText(ERROR_MESSAGES.Bloqueada)).toBeDefined();
    expect(screen.queryByText(/Copiado/)).toBeNull();
  });

  it("RF-49: abre una URL http(s) por el núcleo y explica UrlNoPermitida sin anteponer https://", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Abrir URL en el navegador" }));
    expect(await screen.findByText("La dirección se abrió en el navegador.")).toBeDefined();
    expect(backend.openedUrls).toEqual(["https://github.com/login"]);

    fireEvent.click(within(screen.getByRole("table")).getByRole("button", { name: "VPN" }));
    await within(detailZone()).findByRole("button", { name: "Editar" });
    fireEvent.click(screen.getByRole("button", { name: "Abrir URL en el navegador" }));

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

    fireEvent.click(within(screen.getByRole("table")).getByRole("button", { name: "GitHub" }));
    expect(await screen.findByText("Cargando la entrada…")).toBeDefined();
    await act(async () => {
      release();
      await Promise.resolve();
    });
    await within(detailZone()).findByRole("button", { name: "Editar" });

    backend.rejectOnce("entry_get", { code: "NoEncontrado", message: "core text" });
    fireEvent.click(within(screen.getByRole("table")).getByRole("button", { name: "VPN" }));
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
    fireEvent.change(screen.getByLabelText("Usuario"), { target: { value: "ana" } });
    typePassword("clave larga 123");
    fireEvent.change(screen.getByLabelText("URL"), { target: { value: "https://correo.example" } });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "personal" } });
    fireEvent.change(screen.getByLabelText("Etiquetas (una por línea)"), { target: { value: "correo\n\n hogar \n" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
    expect(lastArgs(backend, "entry_create")).toEqual({
      group_id: "group-personal",
      title: "Correo",
      username: "ana",
      password: "clave larga 123",
      url: "https://correo.example",
      notes: "personal",
      tags: ["correo", "hogar"],
    });
    expect(await within(detailZone()).findByRole("heading", { name: "Correo" })).toBeDefined();
    await waitFor(() => {
      expect(within(screen.getByRole("table")).getByRole("button", { name: "Correo" })).toBeDefined();
    });
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
    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
  });

  it("RF-41: la contraseña viaja tal cual: sin recortar, sin cambiar mayúsculas ni normalizar", async () => {
    const { backend } = await setup();
    await startNewEntry();

    const written = "  Clave\u0301 Ñ  ";
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Exacta" } });
    typePassword(written);
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
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
      expect(await screen.findByText(ERROR_MESSAGES.TituloVacio)).toBeDefined();
      expect(title().getAttribute("aria-invalid")).toBe("true");
      expect(title().getAttribute("aria-describedby")).toContain("entry-title-error");
      expect(document.activeElement).toBe(title());
    }

    fireEvent.change(title(), { target: { value: "a".repeat(256) } });
    save();
    expect(await screen.findByText(/El título admite hasta 255 caracteres/)).toBeDefined();
    expect(backend.count("entry_create")).toBe(0);

    fireEvent.change(title(), { target: { value: "\u{1F511}".repeat(255) } });
    save();
    expect(await screen.findByText("Entrada guardada.")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    fireEvent.change(await screen.findByLabelText("Título"), { target: { value: "  Banco  " } });
    save();
    await waitFor(() => {
      expect(backend.count("entry_create")).toBe(2);
    });
    const created = backend.calls.filter((call) => call.command === "entry_create").at(-1);
    expect(created?.args?.title).toBe("  Banco  ");
    expect(await within(detailZone()).findByRole("heading", { name: "Banco" })).toBeDefined();
  });

  it("RF-91: una etiqueta con ; o , se rechaza junto al campo sin llamar al núcleo", async () => {
    const { backend } = await setup();
    await startNewEntry();

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Etiquetas" } });
    fireEvent.change(screen.getByLabelText("Etiquetas (una por línea)"), { target: { value: "uno, dos" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText(ERROR_MESSAGES.EtiquetaInvalida)).toBeDefined();
    expect(screen.getByLabelText("Etiquetas (una por línea)").getAttribute("aria-invalid")).toBe("true");
    expect(backend.count("entry_create")).toBe(0);
  });

  it("RF-40: un carácter de control se rechaza junto al campo; si el núcleo rechaza, el error va a su campo", async () => {
    const { backend } = await setup();
    await startNewEntry();

    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "Bien" } });
    fireEvent.change(screen.getByLabelText("Usuario"), { target: { value: "ana\u0007" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    expect(await screen.findByText(ERROR_MESSAGES.CaracterNoPermitido)).toBeDefined();
    expect(screen.getByLabelText("Usuario").getAttribute("aria-invalid")).toBe("true");
    expect(backend.count("entry_create")).toBe(0);

    fireEvent.change(screen.getByLabelText("Usuario"), { target: { value: "ana" } });
    backend.rejectOnce("entry_create", { code: "TituloVacio", message: "core text" });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    expect(await screen.findByText(ERROR_MESSAGES.TituloVacio)).toBeDefined();
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

    expect(await screen.findByText(/Fortaleza: muy buena \(4 de 4\)/)).toBeDefined();
    expect(lastArgs(backend, "password_strength")).toEqual({ password: "Abcdef12!" });
    expect(passwordField().type).toBe("password");
    expect(passwordField().getAttribute("autocomplete")).toBe("new-password");
    expect(passwordField().getAttribute("spellcheck")).toBe("false");
  });

  it("RF-45: ejemplo 8 — cambiar solo el título envía solo el título y la contraseña sigue igual", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("form", { name: "Editar GitHub" });
    expect(passwordField().value).toBe("");
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "GitHub personal" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", title: "GitHub personal" });
    expect(backend.entry("entry-github")?.password).toBe(PASSWORD);
    expect(backend.entry("entry-github")?.history).toBe(1);
    const modified = backend.entry("entry-github")?.modified;
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Editar" }));

    // Confirm again without changing anything: neither the date nor the history move.
    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("form", { name: "Editar GitHub personal" });
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
    await screen.findByRole("form", { name: "Editar VPN" });
    expect(screen.getByLabelText<HTMLInputElement>("Usuario").value).toBe("");
    expect(screen.getByText(/Este campo está protegido en la bóveda/)).toBeDefined();
    typePassword("nueva-clave");
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-vpn", password: "nueva-clave" });
    expect(backend.entry("entry-vpn")?.username).toBeNull();
    expect(backend.entry("entry-vpn")?.password).toBe("nueva-clave");
  });

  it("RF-45: un campo vaciado a propósito viaja vacío y reemplaza el valor", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");

    fireEvent.click(screen.getByRole("button", { name: "Editar" }));
    await screen.findByRole("form", { name: "Editar GitHub" });
    fireEvent.change(screen.getByLabelText("Notas"), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("Etiquetas (una por línea)"), { target: { value: "código" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("Entrada guardada.")).toBeDefined();
    expect(lastArgs(backend, "entry_update")).toEqual({ id: "entry-github", notes: "", tags: ["código"] });
  });
});

describe("P5 Entrada — mover y borrar", () => {
  it("RF-46: mover abre un diálogo con el destino y devuelve el foco al cancelar", async () => {
    const { backend } = await setup();
    await openEntry("Trabajo", "GitHub");
    const opener = screen.getByRole("button", { name: "Mover a otro grupo…" });

    opener.focus();
    fireEvent.click(opener);
    const dialog = await screen.findByRole("dialog", { name: "Mover «GitHub»" });
    const options = within(dialog).getAllByRole("option").map((option) => option.textContent);
    expect(options).not.toContain("Cuentas › Trabajo");
    expect(options.some((label) => label.includes("Papelera"))).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);

    fireEvent.click(opener);
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

    fireEvent.click(screen.getByRole("button", { name: "Borrar entrada" }));

    expect(await screen.findByText(/«GitHub» se movió a la papelera/)).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(backend.entry("entry-github")?.groupId).toBe("group-bin");
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Detalle" }));
    expect(await screen.findByRole("button", { name: "Papelera de reciclaje (papelera) · 2 entradas" })).toBeDefined();
  });

  it("RF-47: si la bóveda no tiene papelera, el primer borrado la crea", async () => {
    const { backend } = await setup({
      groups: { id: "r", name: "Cuentas", kind: "root", children: [{ id: "g", name: "Personal" }] },
      entries: [{ id: "e", groupId: "g", title: "Única" }],
    });
    await openEntry("Personal", "Única");

    fireEvent.click(screen.getByRole("button", { name: "Borrar entrada" }));

    expect(await screen.findByRole("button", { name: "Papelera de reciclaje (papelera) · 1 entrada" })).toBeDefined();
    expect(backend.groupTrail(backend.entry("e")?.groupId ?? "")).toEqual(["Cuentas", "Papelera de reciclaje"]);
  });

  it("RF-48: dentro de la papelera pide confirmación, dice que es definitivo y luego la elimina", async () => {
    const { backend } = await setup();
    await openEntry("Papelera de reciclaje", "Canción borrada");

    fireEvent.click(screen.getByRole("button", { name: "Borrar definitivamente…" }));
    const dialog = await screen.findByRole("dialog", { name: "Borrar «Canción borrada» de forma definitiva" });
    expect(within(dialog).getByText(/no se puede deshacer/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.entry("entry-old")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Borrar definitivamente…" }));
    fireEvent.click(
      within(await screen.findByRole("dialog")).getByRole("button", { name: "Borrar definitivamente" }),
    );

    expect(await screen.findByText("Se borró «Canción borrada» de forma definitiva.")).toBeDefined();
    expect(backend.entry("entry-old")).toBeUndefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Detalle" }));
    expect(await screen.findByText("La papelera está vacía.")).toBeDefined();
  });
});

describe("P6 Generador", () => {
  it("RF-65: RF-70: vacío y éxito — valores por defecto, genera, muestra la fortaleza y la usa en el formulario", async () => {
    const { backend } = await setup();
    await startNewEntry();

    fireEvent.click(screen.getByRole("button", { name: "Generar contraseña" }));
    const generator = screen.getByRole("group", { name: "Generador de contraseñas" });
    expect(within(generator).getByLabelText<HTMLInputElement>("Longitud (entre 8 y 128)").value).toBe("20");
    for (const kind of ["Minúsculas (a-z)", "Mayúsculas (A-Z)", "Dígitos (0-9)", "Símbolos (!@#$…)"]) {
      expect(within(generator).getByLabelText<HTMLInputElement>(kind).checked).toBe(true);
    }
    expect(within(generator).getByLabelText<HTMLInputElement>(/Evitar caracteres parecidos/).checked).toBe(false);

    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
    const generated = (await waitFor(() => {
      const node = generator.querySelector("output");
      if (node === null) throw new Error("not yet");
      return node;
    })).textContent;
    expect(Array.from(generated)).toHaveLength(20);
    expect(lastArgs(backend, "generate_password")).toEqual({
      length: 20,
      lower: true,
      upper: true,
      digits: true,
      symbols: true,
      avoid_lookalikes: false,
    });
    expect(within(generator).getByText(/Fortaleza:/)).toBeDefined();

    fireEvent.click(within(generator).getByRole("button", { name: "Usar esta contraseña" }));

    expect(passwordField().value).toBe(generated);
    expect(passwordField().type).toBe("password");
    expect(document.activeElement).toBe(passwordField());
    expect(screen.queryByRole("group", { name: "Generador de contraseñas" })).toBeNull();
    expect(document.body.innerHTML.includes(generated)).toBe(false);
    expect(screen.getByText(/Fortaleza:/)).toBeDefined();
  });

  it("RF-66: RF-67: ejemplo 5 —longitud 8, cuatro tipos y evitar parecidos: así viaja la petición y así sale", async () => {
    const { backend } = await setup();
    await startNewEntry();
    fireEvent.click(screen.getByRole("button", { name: "Generar contraseña" }));
    const generator = screen.getByRole("group", { name: "Generador de contraseñas" });

    fireEvent.change(within(generator).getByLabelText("Longitud (entre 8 y 128)"), { target: { value: "8" } });
    fireEvent.click(within(generator).getByLabelText(/Evitar caracteres parecidos/));
    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));

    const node = await waitFor(() => {
      const found = generator.querySelector("output");
      if (found === null) throw new Error("not yet");
      return found;
    });
    expect(lastArgs(backend, "generate_password")).toEqual({
      length: 8,
      lower: true,
      upper: true,
      digits: true,
      symbols: true,
      avoid_lookalikes: true,
    });
    const password = node.textContent;
    expect(Array.from(password)).toHaveLength(8);
    expect(password).toMatch(/[a-z]/);
    expect(password).toMatch(/[A-Z]/);
    expect(password).toMatch(/[0-9]/);
    expect(password).toMatch(/[^a-zA-Z0-9]/);
    expect(password).not.toMatch(/[0Oo1lI|]/);
  });

  it("RF-68: error — sin tipos o con longitud fuera de rango dice por qué y no llama al núcleo", async () => {
    const { backend } = await setup();
    await startNewEntry();
    fireEvent.click(screen.getByRole("button", { name: "Generar contraseña" }));
    const generator = screen.getByRole("group", { name: "Generador de contraseñas" });

    for (const kind of ["Minúsculas (a-z)", "Mayúsculas (A-Z)", "Dígitos (0-9)", "Símbolos (!@#$…)"]) {
      fireEvent.click(within(generator).getByLabelText(kind));
    }
    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
    expect(await within(generator).findByText(ERROR_MESSAGES.ReglasImposibles)).toBeDefined();

    fireEvent.click(within(generator).getByLabelText("Dígitos (0-9)"));
    for (const length of ["7", "129", "8.5", ""]) {
      fireEvent.change(within(generator).getByLabelText("Longitud (entre 8 y 128)"), { target: { value: length } });
      fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
      expect(await within(generator).findByText(ERROR_MESSAGES.ReglasImposibles)).toBeDefined();
    }
    expect(backend.count("generate_password")).toBe(0);

    fireEvent.change(within(generator).getByLabelText("Longitud (entre 8 y 128)"), { target: { value: "12" } });
    backend.rejectOnce("generate_password", { code: "ReglasImposibles", message: "core text" });
    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
    await waitFor(() => {
      expect(backend.count("generate_password")).toBe(1);
    });
    expect(within(generator).getByText(ERROR_MESSAGES.ReglasImposibles)).toBeDefined();
  });

  it("RF-65: Enter en un campo del generador genera y no guarda la entrada", async () => {
    const { backend } = await setup();
    await startNewEntry();
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "No guardar" } });
    fireEvent.click(screen.getByRole("button", { name: "Generar contraseña" }));
    const length = screen.getByLabelText("Longitud (entre 8 y 128)");

    const notCancelled = fireEvent.keyDown(length, { key: "Enter" });

    expect(notCancelled).toBe(false);
    await waitFor(() => {
      expect(backend.count("generate_password")).toBe(1);
    });
    expect(backend.count("entry_create")).toBe(0);
  });

  it("RF-65: cerrar el generador descarta la contraseña generada y devuelve el foco", async () => {
    await setup();
    await startNewEntry();
    const toggle = screen.getByRole("button", { name: "Generar contraseña" });
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    const generator = screen.getByRole("group", { name: "Generador de contraseñas" });
    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
    const generated = (
      await waitFor(() => {
        const found = generator.querySelector("output");
        if (found === null) throw new Error("not yet");
        return found;
      })
    ).textContent;

    fireEvent.click(within(generator).getByRole("button", { name: "Cerrar generador" }));

    expect(bodyContains(generated)).toBe(false);
    expect(passwordField().value).toBe("");
    expect(document.activeElement).toBe(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
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
    expect(await screen.findByText("Copiado: se borra en 20 s")).toBeDefined();

    act(() => {
      backend.lockFromCore();
    });
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    for (const value of [PASSWORD, "4321", "ana@example.com", "Copiado"]) {
      expect(bodyContains(value)).toBe(false);
    }
    expect(backend.clipboard()).toBeNull();
  });

  it("RF-32: tras vault-locked no queda en el DOM ningún valor generado ni escrito a medias", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");
    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    await screen.findByRole("form", { name: "Nueva entrada en Trabajo" });
    fireEvent.change(screen.getByLabelText("Título"), { target: { value: "borrador-a-medias" } });
    typePassword("escrita-sin-guardar");
    fireEvent.click(screen.getByRole("button", { name: "Generar contraseña" }));
    const generator = screen.getByRole("group", { name: "Generador de contraseñas" });
    fireEvent.click(within(generator).getByRole("button", { name: "Generar" }));
    const generated = (
      await waitFor(() => {
        const found = generator.querySelector("output");
        if (found === null) throw new Error("not yet");
        return found;
      })
    ).textContent;

    act(() => {
      backend.lockFromCore();
    });

    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    for (const value of ["borrador-a-medias", "escrita-sin-guardar", generated]) {
      expect(bodyContains(value)).toBe(false);
    }
  });
});
