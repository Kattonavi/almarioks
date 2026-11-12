import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";

afterEach(cleanup);

const NEW_PATH = "D:/bovedas/nueva.kdbx";
const KEY = "D:/llaves/nueva.keyx";
const SECRET = "maestra-de-prueba";

async function openCreate(options: FakeOptions) {
  const backend = createFakeBackend(options);
  render(<VaultRoot client={createClient(backend.transport)} />);
  fireEvent.click(await screen.findByRole("button", { name: "Crear bóveda nueva" }));
  await screen.findByRole("heading", { name: "Crear bóveda" });
  return backend;
}

const passwordField = () => screen.getByLabelText<HTMLInputElement>("Contraseña maestra");
const confirmField = () => screen.getByLabelText<HTMLInputElement>("Repite la contraseña maestra");
const createButton = () => screen.getByRole<HTMLButtonElement>("button", { name: "Crear bóveda" });

function typeBoth(password: string, confirmation = password) {
  fireEvent.input(passwordField(), { target: { value: password } });
  fireEvent.input(confirmField(), { target: { value: confirmation } });
}

async function choosePath() {
  fireEvent.click(screen.getByRole("button", { name: "Elegir dónde guardarla…" }));
  await screen.findByText(NEW_PATH);
}

async function waitForTicks(ms = 20) {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

describe("P2 Crear bóveda", () => {
  it("RF-1: vacío — campos vacíos con ayuda, el aviso de recuperación y los campos de secreto seguros", async () => {
    await openCreate({});

    expect(screen.getByText(/protégelo con una contraseña maestra, un archivo llave o los dos/)).toBeDefined();
    expect(screen.getByText("Sin elegir")).toBeDefined();
    expect(screen.getByText(/Nadie puede recuperar tu bóveda si olvidas la contraseña maestra/)).toBeDefined();
    expect(screen.getByRole<HTMLInputElement>("radio", { name: "Ninguno" }).checked).toBe(true);
    for (const field of [passwordField(), confirmField()]) {
      expect(field.type).toBe("password");
      expect(field.value).toBe("");
      expect(field.getAttribute("autocomplete")).toBe("new-password");
      expect(field.getAttribute("spellcheck")).toBe("false");
    }
    expect(screen.queryByRole("meter")).toBeNull();
  });

  it("RF-1: éxito — crea con contraseña en un solo invoke, vacía los campos y abre la bóveda", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();
    typeBoth(SECRET);

    fireEvent.click(createButton());
    expect(passwordField().value).toBe("");
    expect(confirmField().value).toBe("");

    expect(await screen.findByRole("heading", { name: "nueva" })).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "vault_create")).toEqual([
      {
        command: "vault_create",
        args: { path: NEW_PATH, name: "nueva", password: SECRET, key_file: { kind: "none" } },
      },
    ]);
    const others = backend.calls.filter(
      (call) => call.command !== "vault_create" && call.command !== "password_strength",
    );
    expect(JSON.stringify(others)).not.toContain(SECRET);
  });

  it("RF-7: mientras escribe muestra la fortaleza y avisa si es débil, sin impedir continuar", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });

    fireEvent.input(passwordField(), { target: { value: "123456" } });
    expect(await screen.findByText(/Fortaleza: muy débil \(0 de 4\)/)).toBeDefined();
    expect(screen.getByText(/Es una contraseña débil/)).toBeDefined();
    expect(screen.getByRole<HTMLMeterElement>("meter", { name: "Fortaleza" }).value).toBe(0);

    fireEvent.input(passwordField(), { target: { value: "k7#Qz!m2Lp$9vR@x4Tw&" } });
    expect(await screen.findByText(/Fortaleza: muy buena \(4 de 4\)/)).toBeDefined();
    expect(screen.queryByText(/Es una contraseña débil/)).toBeNull();

    // Weak does not block: it is created anyway.
    await choosePath();
    typeBoth("123456");
    fireEvent.click(createButton());
    expect(await screen.findByRole("heading", { name: "nueva" })).toBeDefined();
    expect(backend.count("vault_create")).toBe(1);
  });

  it("RF-7: limita las llamadas mientras escribe y descarta una respuesta vieja", async () => {
    const backend = await openCreate({});

    fireEvent.input(passwordField(), { target: { value: "1" } });
    fireEvent.input(passwordField(), { target: { value: "12" } });
    fireEvent.input(passwordField(), { target: { value: "123456" } });
    await screen.findByText(/0 de 4/);
    expect(backend.count("password_strength")).toBe(1);

    const release = backend.hold("password_strength");
    fireEvent.input(passwordField(), { target: { value: "abcdefgh" } });
    await waitFor(() => {
      expect(backend.count("password_strength")).toBe(2);
    });
    fireEvent.input(passwordField(), { target: { value: "k7#Qz!m2Lp$9vR@x4Tw&" } });
    expect(await screen.findByText(/4 de 4/)).toBeDefined();
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(screen.getByText(/4 de 4/)).toBeDefined();
    expect(screen.queryByText(/1 de 4/)).toBeNull();
  });

  it("RF-8: error — si no coinciden no llama al núcleo, lo dice y vacía los dos campos", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();
    typeBoth(SECRET, `${SECRET}x`);

    fireEvent.click(createButton());

    expect(await screen.findByText(/no coinciden/)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");
    expect(confirmField().value).toBe("");
    expect(confirmField().getAttribute("aria-invalid")).toBe("true");
  });

  it("RF-3: error — sin contraseña ni archivo llave dice que falta la llave y no crea", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();

    fireEvent.click(createButton());

    expect(await screen.findByText(ERROR_MESSAGES.LlaveVacia)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
  });

  it("RF-1: error — sin ruta elegida (diálogo cancelado) pide elegirla y no crea", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [null] } });
    fireEvent.click(screen.getByRole("button", { name: "Elegir dónde guardarla…" }));
    await waitForTicks();
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect(await screen.findByText("Elige dónde guardar la bóveda.")).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");
  });

  it("RF-9: éxito — con archivo llave nuevo avisa antes de crear y otra vez al terminar", async () => {
    const backend = await openCreate({
      picks: { pick_vault_to_create: [NEW_PATH], pick_new_key_file: [KEY] },
    });
    await choosePath();
    expect(screen.queryByText(/Sin una copia del archivo llave/)).toBeNull();

    fireEvent.click(screen.getByRole("radio", { name: "Generar un archivo llave nuevo" }));
    expect(screen.getByText(/Sin una copia del archivo llave y sin la contraseña no hay forma/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Elegir dónde guardar el archivo llave…" }));
    await screen.findByText(KEY);
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Bóveda creada" })).toBeDefined();
    expect(screen.getByText(/Sin una copia de ese archivo y sin la contraseña no hay forma de recuperarla/)).toBeDefined();
    expect(screen.getByText(KEY)).toBeDefined();
    expect(backend.calls.find((call) => call.command === "vault_create")?.args).toEqual({
      path: NEW_PATH,
      name: "nueva",
      password: SECRET,
      key_file: { kind: "generate", path: KEY },
    });

    fireEvent.click(screen.getByRole("button", { name: "Ir a la bóveda" }));
    expect(await screen.findByRole("heading", { name: "nueva" })).toBeDefined();
  });

  it("RF-1: éxito — solo con un archivo llave existente, sin contraseña", async () => {
    const backend = await openCreate({
      files: [KEY],
      picks: { pick_vault_to_create: [NEW_PATH], pick_key_file: [KEY] },
    });
    await choosePath();
    fireEvent.click(screen.getByRole("radio", { name: "Usar un archivo llave que ya tengo" }));
    fireEvent.click(createButton());
    expect(await screen.findByText("Elige el archivo llave o marca «Ninguno».")).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    await screen.findByText(KEY);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Bóveda creada" })).toBeDefined();
    expect(backend.calls.find((call) => call.command === "vault_create")?.args).toEqual({
      path: NEW_PATH,
      name: "nueva",
      key_file: { kind: "existing", path: KEY },
    });
  });

  it("RF-1: carga — mientras crea muestra progreso, no se congela y no lanza una segunda creación", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();
    const release = backend.hold("vault_create");
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect(await screen.findByRole("progressbar", { name: "Creando la bóveda" })).toBeDefined();
    expect(screen.getByText(/derivando la llave/)).toBeDefined();
    expect(createButton().matches(":disabled")).toBe(true);
    const form = createButton().closest("form");
    if (form === null) throw new Error("missing form");
    fireEvent.submit(form);
    expect(backend.count("vault_create")).toBe(1);

    await act(async () => {
      release();
      await Promise.resolve();
    });
    expect(await screen.findByRole("heading", { name: "nueva" })).toBeDefined();
  });

  it("RF-4: error — si el archivo ya existe lo dice, no lo toca y vacía los campos", async () => {
    const backend = await openCreate({
      vaults: [{ path: NEW_PATH, name: "Otra", password: "otra" }],
      picks: { pick_vault_to_create: [NEW_PATH] },
    });
    await choosePath();
    typeBoth(SECRET);

    fireEvent.click(createButton());

    const alert = await screen.findByText(ERROR_MESSAGES.ArchivoExiste);
    expect(alert.textContent).not.toContain("ArchivoExiste");
    expect(backend.count("vault_create")).toBe(1);
    expect(passwordField().value).toBe("");
    expect(screen.getByRole("heading", { name: "Crear bóveda" })).toBeDefined();
  });

  it("RF-23: error — no se puede escribir en esa carpeta: lo dice y deja volver a intentarlo", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();
    backend.rejectOnce("vault_create", { code: "GuardadoFallido", message: "core text" });
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect(await screen.findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(passwordField().value).toBe("");
    expect(createButton().matches(":disabled")).toBe(false);

    typeBoth(SECRET);
    fireEvent.click(createButton());
    expect(await screen.findByRole("heading", { name: "nueva" })).toBeDefined();
  });

  it("RF-32: un bloqueo del núcleo retira el formulario a medio llenar", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await choosePath();
    typeBoth(SECRET);

    act(() => {
      backend.lockFromCore();
    });

    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
    expect(screen.queryByText(NEW_PATH)).toBeNull();
  });
});
