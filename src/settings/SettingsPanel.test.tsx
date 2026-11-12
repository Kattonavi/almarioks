import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES, toAppError } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "../vault/VaultRoot";
import { SettingsPanel } from "./SettingsPanel";

afterEach(cleanup);

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

const version = () => Promise.resolve("0.1.0");

function setup(options: FakeOptions = {}) {
  const backend = createFakeBackend(options);
  render(<SettingsPanel client={createClient(backend.transport)} readVersion={version} />);
  return backend;
}

const autoLock = () => screen.getByLabelText<HTMLSelectElement>("Bloquear la bóveda tras este tiempo sin usar la app");
const clipboard = () => screen.getByLabelText<HTMLInputElement>("Limpiar el portapapeles tras (segundos)");

describe("P8 Ajustes y Acerca de", () => {
  it("RF-82: carga — dice que está cargando y luego muestra los valores guardados", async () => {
    const backend = createFakeBackend({ autoLockMinutes: 15, clipboardSeconds: 30 });
    const release = backend.hold("settings_get");
    render(<SettingsPanel client={createClient(backend.transport)} readVersion={version} />);

    expect(screen.getByText("Cargando los ajustes…")).toBeDefined();
    await act(async () => {
      release();
      await Promise.resolve();
    });

    expect((await screen.findByLabelText<HTMLSelectElement>(/Bloquear la bóveda/)).value).toBe("15");
    expect(clipboard().value).toBe("30");
    expect(screen.queryByText("Cargando los ajustes…")).toBeNull();
  });

  it("RF-29: el tiempo de bloqueo solo ofrece 1, 5, 15 y 30 minutos; no existe «nunca»", async () => {
    setup();
    await screen.findByLabelText(/Bloquear la bóveda/);

    expect(within(autoLock()).getAllByRole("option").map((option) => option.textContent)).toEqual([
      "1 minuto",
      "5 minutos",
      "15 minutos",
      "30 minutos",
    ]);
    expect(screen.getByText(/No existe «nunca»/)).toBeDefined();
  });

  it("RF-29: RF-73: éxito — guarda los dos ajustes con settings_set y lo dice", async () => {
    const backend = setup();
    await screen.findByLabelText(/Bloquear la bóveda/);

    fireEvent.change(autoLock(), { target: { value: "30" } });
    fireEvent.change(clipboard(), { target: { value: "45" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar ajustes" }));

    expect(await screen.findByText("Ajustes guardados.")).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "settings_set")).toEqual([
      { command: "settings_set", args: { auto_lock_minutes: 30, clipboard_seconds: 45 } },
    ]);
  });

  it("RF-73: error — un tiempo de limpieza fuera de 10 a 120 s se explica junto al campo y no se envía", async () => {
    const backend = setup();
    await screen.findByLabelText(/Bloquear la bóveda/);

    for (const value of ["5", "121", "12.5", ""]) {
      fireEvent.change(clipboard(), { target: { value } });
      fireEvent.click(screen.getByRole("button", { name: "Guardar ajustes" }));
      expect(await screen.findByText(/entre 10 y 120/)).toBeDefined();
      expect(clipboard().getAttribute("aria-invalid")).toBe("true");
    }
    expect(backend.count("settings_set")).toBe(0);
    expect(document.body.textContent).not.toMatch(/inválid/i);
  });

  it("RF-82: error — si el núcleo rechaza el valor (ArgumentoInvalido), el mensaje va junto al control cambiado", async () => {
    const backend = setup();
    await screen.findByLabelText(/Bloquear la bóveda/);
    backend.rejectOnce("settings_set", { code: "ArgumentoInvalido", message: "x" });

    fireEvent.change(autoLock(), { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar ajustes" }));

    expect(await screen.findByText(/Elige 1, 5, 15 o 30 minutos/)).toBeDefined();
    expect(autoLock().getAttribute("aria-invalid")).toBe("true");
    expect(clipboard().getAttribute("aria-invalid")).toBe("false");
    expect(screen.queryByText(ERROR_MESSAGES.ArgumentoInvalido)).toBeNull();
  });

  it("RF-82: error — si no se pueden leer los ajustes lo dice y deja volver a intentarlo", async () => {
    const backend = createFakeBackend();
    backend.rejectOnce("settings_get", { code: "Inesperado", message: "x" });
    render(<SettingsPanel client={createClient(backend.transport)} readVersion={version} />);

    expect(await screen.findByText(/No se pudieron leer los ajustes/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Volver a cargar los ajustes" }));
    expect(await screen.findByLabelText(/Bloquear la bóveda/)).toBeDefined();
  });

  it("P8: muestra la versión, la licencia GPL-3.0 y el panel de actualización, sin aviso de versión de desarrollo", async () => {
    setup();

    expect(await screen.findByText("Versión 0.1.0")).toBeDefined();
    expect(screen.getByText(/GPL-3\.0/)).toBeDefined();
    // Owner's decision of 2026-11-12: the 0.x versions no longer carry the warning.
    expect(screen.queryByText(/Versión de desarrollo|contraseñas reales/)).toBeNull();
    expect(screen.getByRole("heading", { name: "Actualizaciones" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Buscar actualizaciones" })).toBeDefined();
  });

  it("RF-82: bloqueada — se abre desde la bienvenida y desde el bloqueo, y un bloqueo cierra el diálogo", async () => {
    const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES });
    render(<VaultRoot client={createClient(backend.transport)} />);
    await screen.findByRole("heading", { name: "Cuentas" });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes y Acerca de" }));
    await screen.findByRole("dialog", { name: "Ajustes y Acerca de" });
    act(() => {
      backend.lockFromCore();
    });
    expect(screen.queryByRole("dialog")).toBeNull();

    await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" });
    fireEvent.click(screen.getByRole("button", { name: "Ajustes y Acerca de" }));
    const dialog = await screen.findByRole("dialog", { name: "Ajustes y Acerca de" });
    expect(await within(dialog).findByLabelText(/Bloquear la bóveda/)).toBeDefined();
    // Nothing of the vault inside: no group, no entry.
    expect(within(dialog).queryByText(/Personal|Banco|Trabajo/)).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cerrar" }));
    expect(screen.queryByRole("dialog")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Cerrar bóveda" }));
    await screen.findByRole("heading", { name: "Bienvenida" });
    fireEvent.click(screen.getByRole("button", { name: "Ajustes y Acerca de" }));
    expect(await screen.findByRole("dialog", { name: "Ajustes y Acerca de" })).toBeDefined();
  });
});

describe("Precisiones del contrato (plan.md 15 y 16)", () => {
  it("RF-40: `AppError` conserva `field` y `limit` solo en los códigos que los llevan", () => {
    expect(toAppError({ code: "CampoMuyLargo", message: "x", field: "url", limit: 2048 })).toEqual({
      code: "CampoMuyLargo",
      field: "url",
      limit: 2048,
    });
    expect(toAppError({ code: "CaracterNoPermitido", message: "x", field: "notes", limit: null })).toEqual({
      code: "CaracterNoPermitido",
      field: "notes",
    });
    expect(toAppError({ code: "TituloVacio", message: "x", field: "title" })).toEqual({ code: "TituloVacio" });
    expect(toAppError({ code: "CampoMuyLargo", message: "x" })).toEqual({ code: "CampoMuyLargo" });
  });

  it("RF-40: el error del núcleo con `field` y `limit` se muestra junto a ese campo, con ese límite", async () => {
    const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES });
    render(<VaultRoot client={createClient(backend.transport)} />);
    fireEvent.click(await screen.findByRole("button", { name: /^Personal / }));
    await screen.findByRole("heading", { name: "Entradas de Personal" });
    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    fireEvent.change(await screen.findByLabelText("Título"), { target: { value: "Correo" } });
    fireEvent.change(screen.getByLabelText("Sitio"), { target: { value: "https://correo.example" } });
    backend.rejectOnce("entry_create", { code: "CampoMuyLargo", message: "x", field: "url", limit: 20 });

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    expect(await screen.findByText("El sitio admite hasta 20 caracteres. Acórtalo.")).toBeDefined();
    await waitFor(() => {
      expect(screen.getByLabelText("Sitio").getAttribute("aria-invalid")).toBe("true");
    });
  });
});

describe("P8 Ajustes — el archivo de la bóveda", () => {
  it("RF-102: la ruta del archivo de la bóveda abierta se ve en Ajustes, en una fila de solo lectura, y no sin bóveda", async () => {
    const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES });
    render(<VaultRoot client={createClient(backend.transport)} />);
    const name = await screen.findByRole("heading", { name: "Cuentas", level: 2 });
    // The rail shows no paths: the file is in the `title` of the name of the vault…
    expect(name.title).toBe(PATH);
    expect(screen.queryByText(PATH)).toBeNull();

    // …and in Settings.
    fireEvent.click(screen.getByRole("button", { name: "Ajustes y Acerca de" }));
    const dialog = await screen.findByRole("dialog", { name: "Ajustes y Acerca de" });
    expect(within(dialog).getByText("Archivo de la bóveda")).toBeDefined();
    expect(within(dialog).getByText(PATH)).toBeDefined();
    expect(within(dialog).queryByDisplayValue(PATH)).toBeNull();

    cleanup();
    setup();
    await screen.findByLabelText("Limpiar el portapapeles tras (segundos)");
    expect(screen.queryByText("Archivo de la bóveda")).toBeNull();
  });
});
