import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createClient, type Transport } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, keyFileBeside, type FakeOptions } from "../ipc/fake";

import { KEY_SUGGESTED } from "./UnlockScreen";
import { STATUS_POLL_MS, VaultRoot } from "./VaultRoot";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const PATH = "D:/bovedas/cuentas.kdbx";
const KEY = "D:/llaves/cuentas.keyx";
const WITH_PASSWORD = { path: PATH, name: "Cuentas", password: "correcta" };
const WITH_BOTH = { ...WITH_PASSWORD, keyFilePath: KEY };
/** RF-110: the key file beside the vault, the one the core proposes. */
const BESIDE = keyFileBeside(PATH);

function setup(options: FakeOptions) {
  const backend = createFakeBackend(options);
  render(<VaultRoot client={createClient(backend.transport)} />);
  return backend;
}

const passwordField = () => screen.getByLabelText<HTMLInputElement>("Contraseña maestra");
const unlockButton = () => screen.getByRole<HTMLButtonElement>("button", { name: "Desbloquear" });
const keyFileGroup = () => screen.getByRole("group", { name: "Archivo llave" });

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

  it("RF-110: la llave que está junto a la bóveda viene ya elegida y un clic en Desbloquear la manda", async () => {
    const backend = setup({
      vaults: [{ ...WITH_PASSWORD, keyFilePath: BESIDE }],
      recent: [{ path: PATH, exists: true }],
      files: [BESIDE],
    });
    fireEvent.click(await screen.findByRole("button", { name: "Abrir cuentas.kdbx" }));
    await findUnlockScreen();

    // Its name, with the whole path in `title`, and the line that says the app chose it.
    const proposed = await screen.findByText("cuentas.keyx");
    expect(proposed.title).toBe(BESIDE);
    expect(screen.getByText(KEY_SUGGESTED).textContent).toBe(
      "Se eligió sola la llave que está junto a la bóveda. Si esta bóveda no la usa, quítala.",
    );
    expect(screen.queryByText("Ninguno")).toBeNull();
    expect(keyFileGroup().getAttribute("aria-describedby")).toBe("unlock-key-hint");
    expect(passwordField().getAttribute("aria-describedby")).toBe("unlock-help unlock-key-hint");
    // The path asked is the one the core gave, and the password still has the focus: type it and Enter.
    expect(backend.calls.filter((call) => call.command === "unlock_key_suggestion")).toEqual([
      { command: "unlock_key_suggestion", args: { path: PATH } },
    ]);
    expect(document.activeElement).toBe(passwordField());

    typePassword("correcta");
    fireEvent.click(unlockButton());

    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "vault_open")).toEqual([
      { command: "vault_open", args: { path: PATH, password: "correcta", key_file_path: BESIDE } },
    ]);
  });

  it("RF-110: la llave propuesta se puede cambiar por otra con «Elegir archivo llave…»", async () => {
    const backend = setup({
      vaults: [WITH_BOTH],
      status: { state: "locked", path: PATH, recovery: null },
      files: [BESIDE],
      picks: { pick_key_file: [KEY] },
    });
    await findUnlockScreen();
    await screen.findByText("cuentas.keyx");

    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    expect(await screen.findByText(KEY)).toBeDefined();
    expect(screen.queryByText("cuentas.keyx")).toBeNull();
    expect(screen.queryByText(KEY_SUGGESTED)).toBeNull();
    expect(keyFileGroup().getAttribute("aria-describedby")).toBeNull();

    typePassword("correcta");
    fireEvent.click(unlockButton());
    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(backend.calls.find((call) => call.command === "vault_open")?.args).toEqual({
      path: PATH,
      password: "correcta",
      key_file_path: KEY,
    });
  });

  it("RF-110: la llave propuesta se puede quitar, y un desbloqueo fallido no la vuelve a poner", async () => {
    const backend = setup({
      vaults: [WITH_PASSWORD],
      recent: [{ path: PATH, exists: true }],
      files: [BESIDE],
    });
    fireEvent.click(await screen.findByRole("button", { name: "Abrir cuentas.kdbx" }));
    await findUnlockScreen();
    await screen.findByText("cuentas.keyx");

    fireEvent.click(screen.getByRole("button", { name: "Quitar archivo llave" }));
    expect(screen.getByText("Ninguno")).toBeDefined();
    expect(screen.queryByText(KEY_SUGGESTED)).toBeNull();
    expect(screen.queryByRole("button", { name: "Quitar archivo llave" })).toBeNull();

    typePassword("mala");
    fireEvent.click(unlockButton());
    expect(await screen.findByText(ERROR_MESSAGES.LlaveIncorrecta)).toBeDefined();
    await waitForTicks();
    expect(screen.getByText("Ninguno")).toBeDefined();
    expect(screen.queryByText("cuentas.keyx")).toBeNull();
    expect(backend.count("unlock_key_suggestion")).toBe(1);

    typePassword("correcta");
    fireEvent.click(unlockButton());
    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "vault_open").at(-1)?.args).toEqual({
      path: PATH,
      password: "correcta",
    });
  });

  it("RF-110: una respuesta tardía no pisa el archivo llave que la persona ya eligió", async () => {
    const backend = createFakeBackend({
      vaults: [WITH_BOTH],
      status: { state: "locked", path: PATH, recovery: null },
      files: [BESIDE],
      picks: { pick_key_file: [KEY] },
    });
    const answer = backend.hold("unlock_key_suggestion");
    render(<VaultRoot client={createClient(backend.transport)} />);
    await findUnlockScreen();
    // The question was asked and its answer is still on the way: nothing is proposed yet.
    await waitForTicks();
    expect(backend.count("unlock_key_suggestion")).toBe(1);
    expect(screen.getByText("Ninguno")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    await screen.findByText(KEY);
    answer();
    await waitForTicks();

    expect(backend.count("unlock_key_suggestion")).toBe(1);
    expect(screen.getByText(KEY)).toBeDefined();
    expect(screen.queryByText("cuentas.keyx")).toBeNull();
    expect(screen.queryByText(KEY_SUGGESTED)).toBeNull();
  });

  it("RF-110: sin llave junto a la bóveda, o si la consulta falla, queda «Ninguno» y no hay error", async () => {
    // `null`: nothing beside the vault (which does not mean the vault has no key file).
    const none = setup({ vaults: [WITH_PASSWORD], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();
    await waitForTicks();
    expect(none.count("unlock_key_suggestion")).toBe(1);
    expect(screen.getByText("Ninguno")).toBeDefined();
    expect(screen.queryByText(KEY_SUGGESTED)).toBeNull();
    expect(screen.getByRole("alert").textContent).toBe("");
    expect(passwordField().getAttribute("aria-describedby")).toBe("unlock-help");
    cleanup();

    // The question fails: the screen is the one of always.
    const failing = createFakeBackend({
      vaults: [WITH_PASSWORD],
      status: { state: "locked", path: PATH, recovery: null },
      files: [BESIDE],
    });
    failing.rejectOnce("unlock_key_suggestion", { code: "ErrorInterno", message: "core text" });
    render(<VaultRoot client={createClient(failing.transport)} />);
    await findUnlockScreen();
    await waitForTicks();
    expect(failing.count("unlock_key_suggestion")).toBe(1);
    expect(screen.getByText("Ninguno")).toBeDefined();
    expect(screen.queryByText("cuentas.keyx")).toBeNull();
    expect(screen.getByRole("alert").textContent).toBe("");

    typePassword("correcta");
    fireEvent.click(unlockButton());
    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
  });

  it("RF-112: el fallo no afirma que la bóveda use archivo llave, y la pista de la llave propuesta se queda", async () => {
    // An old key file beside a vault that does not use one: the proposal is what makes the unlock fail.
    const backend = setup({
      vaults: [WITH_PASSWORD],
      recent: [{ path: PATH, exists: true }],
      files: [BESIDE],
    });
    fireEvent.click(await screen.findByRole("button", { name: "Abrir cuentas.kdbx" }));
    await findUnlockScreen();
    await screen.findByText("cuentas.keyx");

    typePassword("correcta");
    fireEvent.click(unlockButton());

    const alert = await screen.findByRole("alert");
    await waitFor(() => {
      expect(alert.textContent).toBe(
        "La contraseña o el archivo llave no coinciden. Si esta bóveda usa archivo llave, elígelo.",
      );
    });
    expect(ERROR_MESSAGES.LlaveIncorrecta).toBe(alert.textContent);
    // Next to the key file the app chose, the way out is still in sight; and it is read with the password.
    expect(screen.getByText("cuentas.keyx").title).toBe(BESIDE);
    expect(screen.getByText(KEY_SUGGESTED)).toBeDefined();
    expect(passwordField().getAttribute("aria-describedby")).toBe("unlock-help unlock-key-hint unlock-error");
    expect(document.activeElement).toBe(passwordField());
    expect(backend.count("unlock_key_suggestion")).toBe(1);

    fireEvent.click(screen.getByRole("button", { name: "Quitar archivo llave" }));
    typePassword("correcta");
    fireEvent.click(unlockButton());
    expect(await screen.findByRole("heading", { name: "Cuentas" })).toBeDefined();
  });

  it("RF-112: el fallo sin archivo llave elegido dice que lo elija si la bóveda lo usa", async () => {
    setup({ vaults: [WITH_BOTH], status: { state: "locked", path: PATH, recovery: null } });
    await findUnlockScreen();

    typePassword("correcta");
    fireEvent.click(unlockButton());

    expect(
      await screen.findByText("La contraseña o el archivo llave no coinciden. Si esta bóveda usa archivo llave, elígelo."),
    ).toBeDefined();
    expect(screen.getByText("Ninguno")).toBeDefined();
    expect(screen.queryByText(KEY_SUGGESTED)).toBeNull();
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

describe("P4 Bóveda · cuenta de bloqueo", () => {
  const OPEN = {
    state: "open",
    path: PATH,
    name: "Cuentas",
    read_only: false,
    weak_kdf: false,
    save_pending: false,
    recovery: null,
  } as const;

  const countdown = () => document.querySelector(".lock-countdown .num")?.textContent ?? null;

  async function advance(ms: number) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ms);
    });
  }

  it("RF-29: la cuenta «Se bloquea en» baja cada segundo y sigue al núcleo cada 5 s sin contar como actividad", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = setup({ status: { ...OPEN, locks_in_seconds: 300 } });
    await screen.findByRole("heading", { name: "Cuentas" });
    expect(countdown()).toBe("5:00");

    await advance(3000);
    expect(countdown()).toBe("4:57");

    // The core says 90 s are left (e.g. the lock time changed in another way): the next poll follows it.
    const before = backend.count("vault_status");
    backend.respondOnce("vault_status", { ...OPEN, locks_in_seconds: 90 });
    await advance(STATUS_POLL_MS - 3000);
    expect(backend.count("vault_status")).toBe(before + 1);
    expect(countdown()).toBe("1:30");

    await advance(2000);
    expect(countdown()).toBe("1:28");
    // Asking is not activity: the lock is never pushed back by the window itself.
    expect(backend.count("activity")).toBe(0);
  });

  it("RF-29: nunca muestra menos de cero, y sin «locks_in_seconds» no muestra nada", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = setup({ status: { ...OPEN, locks_in_seconds: 2 } });
    await screen.findByRole("heading", { name: "Cuentas" });
    expect(countdown()).toBe("0:02");
    // The next poll still brings the field: answer without it before the clock runs out, then let it run out.
    await advance(4000);
    expect(countdown()).toBe("0:00");

    backend.respondOnce("vault_status", OPEN);
    await advance(STATUS_POLL_MS);
    expect(countdown()).toBeNull();
    expect(screen.queryByText("Se bloquea en")).toBeNull();
  });

  it("RF-29: tras guardar los ajustes la cuenta usa el tiempo nuevo sin esperar a la siguiente consulta", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = setup({ status: { ...OPEN, locks_in_seconds: 300 } });
    await screen.findByRole("heading", { name: "Cuentas" });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes y Acerca de" }));
    const select = await screen.findByLabelText<HTMLSelectElement>(
      "Bloquear la bóveda tras este tiempo sin usar la app",
    );
    fireEvent.change(select, { target: { value: "1" } });
    backend.respondOnce("vault_status", { ...OPEN, locks_in_seconds: 60 });
    fireEvent.click(screen.getByRole("button", { name: "Guardar ajustes" }));

    await waitFor(() => {
      expect(countdown()).toBe("1:00");
    });
  });

  it("RF-29: RF-32: al bloquear deja de consultar y una respuesta tardía no vuelve a montar la bóveda", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = createFakeBackend({ status: { ...OPEN, locks_in_seconds: 300 } });
    let gate: Promise<void> | null = null;
    let release: () => void = () => undefined;
    // A poll that left before the lock and answers after it: an "open" status taken before, delivered after.
    const transport: Transport = {
      listen: (event, handler) => backend.transport.listen(event, handler),
      async invoke(command, payload) {
        const value = await backend.transport.invoke(command, payload);
        if (command === "vault_status" && gate !== null) await gate;
        return value;
      },
    };
    render(<VaultRoot client={createClient(transport)} />);
    await screen.findByRole("heading", { name: "Cuentas" });

    gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    await advance(STATUS_POLL_MS);
    gate = null;
    act(() => {
      backend.lockFromCore();
    });
    expect(await findUnlockScreen()).toBeDefined();
    await act(async () => {
      release();
      await Promise.resolve();
    });
    await waitForTicks();
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
    expect(countdown()).toBeNull();

    const afterLock = backend.count("vault_status");
    await advance(STATUS_POLL_MS * 3);
    expect(backend.count("vault_status")).toBe(afterLock);
  });
});
