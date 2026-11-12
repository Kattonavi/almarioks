import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";

afterEach(cleanup);

const PATH = "D:/bovedas/cuentas.kdbx";
const KEY = "D:/llaves/cuentas.keyx";
const WITH_PASSWORD = { path: PATH, name: "Cuentas", password: "correcta" };
const WITH_BOTH = { ...WITH_PASSWORD, keyFilePath: KEY };

function setup(options: FakeOptions) {
  const backend = createFakeBackend(options);
  render(<VaultRoot client={createClient(backend.transport)} />);
  return backend;
}

const passwordField = () => screen.getByLabelText<HTMLInputElement>("Contraseña maestra");
const unlockButton = () => screen.getByRole<HTMLButtonElement>("button", { name: "Desbloquear" });

function typePassword(value: string) {
  fireEvent.change(passwordField(), { target: { value } });
}

async function findUnlockScreen(fileName = "cuentas.kdbx") {
  return screen.findByRole("heading", { name: `Desbloquear ${fileName}` });
}

async function waitForTicks() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 20));
  });
}

describe("P1 Bienvenida", () => {
  it("RF-19: vacío — sin bóvedas recientes ofrece las dos acciones", async () => {
    setup({});

    expect(await screen.findByText(/Todavía no has abierto ninguna bóveda/)).toBeDefined();
    expect(screen.getByRole("button", { name: "Crear bóveda nueva" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Abrir una bóveda…" })).toBeDefined();
  });

  it("RF-19: éxito — lista las recientes y abrir una lleva a P3 (y se puede volver)", async () => {
    setup({ recent: [{ path: PATH, exists: true }] });

    fireEvent.click(await screen.findByRole("button", { name: "Abrir cuentas.kdbx" }));
    expect(await findUnlockScreen()).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Volver a la bienvenida" }));
    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
  });

  it("RF-20: error — una reciente que ya no existe lo dice y se puede quitar de la lista", async () => {
    const lost = "D:/viejo/perdida.kdbx";
    const backend = setup({ recent: [{ path: lost, exists: false }] });

    expect(await screen.findByText(/ya no está en/)).toBeDefined();
    expect(screen.queryByRole("button", { name: "Abrir perdida.kdbx" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Quitar perdida.kdbx de la lista" }));

    expect(await screen.findByText(/Todavía no has abierto ninguna bóveda/)).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "recent_remove")).toEqual([
      { command: "recent_remove", args: { path: lost } },
    ]);
  });

  it("RF-20: error — buscar una reciente perdida abre el diálogo del núcleo y lleva a P3 con la ruta elegida", async () => {
    setup({
      recent: [{ path: "D:/viejo/perdida.kdbx", exists: false }],
      picks: { pick_vault_to_open: ["E:/movida/perdida.kdbx"] },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Buscar perdida.kdbx…" }));

    expect(await findUnlockScreen("perdida.kdbx")).toBeDefined();
    expect(screen.getByText("E:/movida/perdida.kdbx")).toBeDefined();
  });

  it("RF-10: éxito — abrir otra bóveda con el diálogo; cancelar el diálogo deja la bienvenida", async () => {
    const backend = setup({ picks: { pick_vault_to_open: [null, PATH] } });
    const open = await screen.findByRole("button", { name: "Abrir una bóveda…" });

    fireEvent.click(open);
    await waitFor(() => {
      expect(backend.count("pick_vault_to_open")).toBe(1);
    });
    await waitForTicks();
    expect(screen.getByRole("heading", { name: "Bienvenida" })).toBeDefined();

    fireEvent.click(open);
    expect(await findUnlockScreen()).toBeDefined();
  });

  it("RF-19: éxito — crear va a la pantalla de crear bóveda (P2)", async () => {
    setup({});

    fireEvent.click(await screen.findByRole("button", { name: "Crear bóveda nueva" }));

    expect(await screen.findByRole("heading", { name: "Crear bóveda" })).toBeDefined();
  });

  it("RF-83: error — si no se pueden leer las recientes lo dice sin código y deja las acciones", async () => {
    const backend = createFakeBackend({});
    backend.rejectOnce("recent_list", new Error("disk"));
    render(<VaultRoot client={createClient(backend.transport)} />);

    const message = await screen.findByText(ERROR_MESSAGES.Inesperado);
    expect(message.textContent).not.toContain("Inesperado");
    expect(screen.getByRole("button", { name: "Crear bóveda nueva" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Abrir una bóveda…" })).toBeDefined();
  });

  it("RF-78: una ruta con HTML se pinta como texto", async () => {
    setup({ recent: [{ path: "D:/<img src=x onerror=alert(1)>.kdbx", exists: true }] });

    expect(
      await screen.findByRole("button", { name: "Abrir <img src=x onerror=alert(1)>.kdbx" }),
    ).toBeDefined();
    expect(document.querySelector("img")).toBeNull();
  });
});

describe("P3 Desbloquear", () => {
  it("RF-10: éxito — la llave correcta abre la bóveda; la maestra viaja en un solo invoke y el campo se vacía", async () => {
    const backend = setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    const field = passwordField();
    expect(field.type).toBe("password");
    expect(field.getAttribute("autocomplete")).toBe("off");
    expect(field.getAttribute("spellcheck")).toBe("false");

    typePassword("correcta");
    fireEvent.click(unlockButton());
    expect(field.value).toBe("");

    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "vault_open")).toEqual([
      { command: "vault_open", args: { path: PATH, password: "correcta" } },
    ]);
    const others = backend.calls.filter((call) => call.command !== "vault_open");
    expect(JSON.stringify(others)).not.toContain("correcta");
  });

  it("RF-11: error — llave incorrecta da un solo mensaje sea cual sea la parte equivocada y sigue bloqueada", async () => {
    const backend = setup({
      vaults: [WITH_BOTH],
      status: { state: "locked", path: PATH, recovery: null },
      picks: { pick_key_file: [KEY, "D:/llaves/otra.keyx"] },
    });
    await findUnlockScreen();
    const messages: string[] = [];

    // Wrong password, right key file.
    typePassword("mala");
    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    await screen.findByText(KEY);
    fireEvent.click(unlockButton());
    messages.push((await screen.findByText(ERROR_MESSAGES.LlaveIncorrecta)).textContent);
    expect(passwordField().value).toBe("");

    // Right password, key file of another vault.
    typePassword("correcta");
    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    await screen.findByText("D:/llaves/otra.keyx");
    fireEvent.click(unlockButton());
    messages.push((await screen.findByText(ERROR_MESSAGES.LlaveIncorrecta)).textContent);

    // Right password, no key file.
    typePassword("correcta");
    fireEvent.click(screen.getByRole("button", { name: "Quitar archivo llave" }));
    fireEvent.click(unlockButton());
    messages.push((await screen.findByText(ERROR_MESSAGES.LlaveIncorrecta)).textContent);

    expect(backend.count("vault_open")).toBe(3);
    expect(new Set(messages)).toEqual(new Set([ERROR_MESSAGES.LlaveIncorrecta]));
    expect(screen.getByRole("alert").textContent).not.toMatch(/LlaveIncorrecta|core text/);
    expect(passwordField().getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
  });

  it("RF-18: carga — mientras deriva muestra progreso, no se congela y no lanza un segundo desbloqueo", async () => {
    const backend = setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    const release = backend.holdUnlock();

    typePassword("correcta");
    fireEvent.click(unlockButton());

    expect(await screen.findByText(/derivando la llave/)).toBeDefined();
    expect(screen.getByRole("progressbar", { name: "Abriendo la bóveda" })).toBeDefined();
    // Disabled through its <fieldset disabled>, so `.disabled` of the button itself stays false.
    expect(unlockButton().matches(":disabled")).toBe(true);
    expect(passwordField().matches(":disabled")).toBe(true);
    const form = unlockButton().closest("form");
    if (form === null) throw new Error("missing form");
    fireEvent.submit(form);
    expect(backend.count("vault_open")).toBe(1);

    await act(async () => {
      release();
      await Promise.resolve();
    });
    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("RF-18: carga — si el núcleo informa una derivación en curso muestra progreso y pregunta hasta que termina", async () => {
    const backend = createFakeBackend({ status: { state: "locked", path: PATH, recovery: null } });
    backend.respondOnce("vault_status", { state: "deriving", path: PATH });
    render(<VaultRoot client={createClient(backend.transport)} />);

    expect(await screen.findByRole("progressbar", { name: "Abriendo la bóveda" })).toBeDefined();
    await waitFor(() => {
      expect(screen.queryByRole("progressbar")).toBeNull();
    });
    expect(screen.getByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(backend.count("vault_status")).toBeGreaterThanOrEqual(2);
  });

  it("RF-83: error — archivo dañado, formato no soportado y archivo llave dañado o no encontrado", async () => {
    const backend = setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    const codes = ["ArchivoDanado", "FormatoNoSoportado", "ArchivoLlaveDanado", "ArchivoLlaveNoEncontrado"] as const;

    for (const code of codes) {
      backend.rejectOnce("vault_open", { code, message: "core text" });
      typePassword("correcta");
      fireEvent.click(unlockButton());
      const alert = await screen.findByText(ERROR_MESSAGES[code]);
      expect(alert.textContent).not.toContain(code);
      expect(passwordField().value).toBe("");
    }
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
  });

  it("RF-34: tras un bloqueo con pendientes, al desbloquear dice en qué archivo de recuperación quedaron", async () => {
    const recoveryPath = "D:/bovedas/cuentas.kdbx.recuperacion-20261111-101500.kdbx";
    setup({
      vaults: [WITH_PASSWORD],
      status: { state: "locked", path: PATH, recovery: { kind: "saved", path: recoveryPath } },
    });
    await findUnlockScreen();

    typePassword("correcta");
    fireEvent.click(unlockButton());

    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(screen.getByText(/Quedaron en un archivo de recuperación/)).toBeDefined();
    expect(screen.getByText(recoveryPath)).toBeDefined();
  });

  it("RF-34: tras un bloqueo con pendientes, al desbloquear dice si se perdieron", async () => {
    setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: { kind: "lost" } } });
    await findUnlockScreen();

    typePassword("correcta");
    fireEvent.click(unlockButton());

    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(screen.getByText(/no se pudieron conservar/)).toBeDefined();
  });

  it("RF-28: bloqueada — Bloquear con la bóveda abierta lleva a la pantalla de desbloqueo", async () => {
    const backend = setup({
      vaults: [WITH_PASSWORD],
      status: {
        state: "open",
        path: PATH,
        name: "Cuentas",
        read_only: false,
        weak_kdf: false,
        save_pending: false,
        recovery: null,
      },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Bloquear" }));

    expect(await findUnlockScreen()).toBeDefined();
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
    expect(backend.count("vault_lock")).toBe(1);
  });

  it("RF-32: el evento vault-locked retira la bóveda abierta y el formulario a medio llenar", async () => {
    const backend = setup({
      vaults: [WITH_BOTH],
      status: {
        state: "open",
        path: PATH,
        name: "Cuentas",
        read_only: false,
        weak_kdf: false,
        save_pending: false,
        recovery: null,
      },
      picks: { pick_key_file: [KEY] },
    });
    await screen.findByRole("heading", { name: "Cuentas" });

    act(() => {
      backend.lockFromCore();
    });
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
    expect(screen.queryByText(PATH)).toBeNull();
    await findUnlockScreen();

    typePassword("a medio escribir");
    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    await screen.findByText(KEY);
    act(() => {
      backend.lockFromCore();
    });
    expect(screen.queryByLabelText("Contraseña maestra")).toBeNull();

    await findUnlockScreen();
    expect(passwordField().value).toBe("");
    expect(screen.queryByText(KEY)).toBeNull();
    expect(screen.getByText("Ninguno")).toBeDefined();
  });

  it("RF-32: un desbloqueo que responde después del bloqueo no vuelve a montar la bóveda", async () => {
    const backend = setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    const release = backend.holdUnlock();
    typePassword("correcta");
    fireEvent.click(unlockButton());
    await screen.findByRole("progressbar");

    act(() => {
      backend.lockFromCore();
    });
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(await findUnlockScreen()).toBeDefined();
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("RF-29: con la bóveda abierta avisa actividad al núcleo; bloqueada no", async () => {
    const open = setup({
      status: {
        state: "open",
        path: PATH,
        name: "Cuentas",
        read_only: false,
        weak_kdf: false,
        save_pending: false,
        recovery: null,
      },
    });
    await screen.findByRole("heading", { name: "Cuentas" });
    fireEvent.keyDown(window, { key: "a" });
    expect(open.count("activity")).toBe(1);
    cleanup();

    const locked = setup({ status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    fireEvent.keyDown(window, { key: "a" });
    expect(locked.count("activity")).toBe(0);
  });

  it("RF-83: si no se puede leer el estado de la bóveda lo dice y permite reintentar", async () => {
    const backend = createFakeBackend({});
    backend.rejectOnce("vault_status", new Error("ipc"));
    render(<VaultRoot client={createClient(backend.transport)} />);

    expect(await screen.findByText(ERROR_MESSAGES.Inesperado)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Reintentar" }));

    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
  });

  it("RF-36: cerrar desde la pantalla de desbloqueo vuelve a la bienvenida", async () => {
    const backend = setup({ status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();

    fireEvent.click(screen.getByRole("button", { name: "Cerrar bóveda" }));

    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
    expect(backend.count("vault_close")).toBe(1);
  });
});
