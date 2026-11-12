import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, FAKE_DEFAULT_FOLDER, type FakeBackend, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";
import {
  EXISTS_CHOSEN,
  EXISTS_KEY,
  EXISTS_VAULT,
  KEY_BESIDE_NEEDS_PASSWORD,
  KEY_HELP_BESIDE,
  KEY_HELP_EXISTING,
  KEY_HELP_NONE,
  NO_NAME,
  NO_PATH,
  PATH_HELP,
} from "./CreateScreen";

afterEach(cleanup);

/** Where the fake core proposes "Familia" (RF-97), and the key file beside it (RF-99). */
const FAMILY_PATH = `${FAKE_DEFAULT_FOLDER}/Familia.kdbx`;
const FAMILY_KEY = `${FAKE_DEFAULT_FOLDER}/Familia.keyx`;
/** What the save dialog answers when another location is chosen. */
const NEW_PATH = "D:/bovedas/nueva.kdbx";
const KEY = "D:/llaves/mia.keyx";
const SECRET = "maestra-de-prueba";

/** `prepare` runs before the screen opens: the first question about the location is asked on mount. */
async function openCreate(options: FakeOptions, prepare?: (backend: FakeBackend) => void) {
  const backend = createFakeBackend(options);
  prepare?.(backend);
  render(<VaultRoot client={createClient(backend.transport)} />);
  fireEvent.click(await screen.findByRole("button", { name: "Crear bóveda nueva" }));
  await screen.findByRole("heading", { name: "Crear bóveda" });
  return backend;
}

const nameField = () => screen.getByLabelText<HTMLInputElement>("Nombre de la bóveda");
const passwordField = () => screen.getByLabelText<HTMLInputElement>("Contraseña maestra");
const confirmField = () => screen.getByLabelText<HTMLInputElement>("Repite la contraseña maestra");
const createButton = () => screen.getByRole<HTMLButtonElement>("button", { name: "Crear bóveda" });
const keyOption = (name: string) => screen.getByRole<HTMLInputElement>("radio", { name });
const createCalls = (backend: FakeBackend) =>
  backend.calls.filter((call) => call.command === "vault_create");

function typeBoth(password: string, confirmation = password) {
  fireEvent.input(passwordField(), { target: { value: password } });
  fireEvent.input(confirmField(), { target: { value: confirmation } });
}

function typeName(name: string) {
  fireEvent.change(nameField(), { target: { value: name } });
}

/** Writes the name and waits for the core to say which file it would be. */
async function nameIt(name = "Familia", file = `${name}.kdbx`) {
  typeName(name);
  await screen.findByText(file);
}

function withoutKeyFile() {
  fireEvent.click(keyOption("Sin archivo llave"));
}

function withNewKeyFile() {
  fireEvent.click(keyOption("Crear uno nuevo"));
}

/** The whole line under the key file options, as it is read (its file name is a separate element). */
const keyHelp = () => document.getElementById("create-key-help")?.textContent;

async function chooseLocation() {
  fireEvent.click(screen.getByRole("button", { name: "Cambiar ubicación…" }));
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

    expect(screen.getByText("Ponle nombre y protégela. Lo demás se puede cambiar después.")).toBeDefined();
    expect(nameField().value).toBe("");
    expect(screen.getByText(/Si pierdes la contraseña maestra o el archivo llave, nadie puede abrir la bóveda/)).toBeDefined();
    for (const field of [passwordField(), confirmField()]) {
      expect(field.type).toBe("password");
      expect(field.value).toBe("");
      expect(field.getAttribute("autocomplete")).toBe("new-password");
      expect(field.getAttribute("spellcheck")).toBe("false");
    }
    expect(screen.queryByRole("meter")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("RF-1: éxito — crea con contraseña en un solo invoke, vacía los campos y abre la bóveda", async () => {
    const backend = await openCreate({});
    await nameIt();
    withoutKeyFile();
    typeBoth(SECRET);

    fireEvent.click(createButton());
    expect(passwordField().value).toBe("");
    expect(confirmField().value).toBe("");

    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
    expect(createCalls(backend)).toEqual([
      {
        command: "vault_create",
        args: { path: FAMILY_PATH, name: "Familia", password: SECRET, key_file: { kind: "none" } },
      },
    ]);
    const others = backend.calls.filter(
      (call) => call.command !== "vault_create" && call.command !== "password_strength",
    );
    expect(JSON.stringify(others)).not.toContain(SECRET);
  });

  it("RF-1: el ojo muestra y oculta las dos contraseñas, cambia de nombre y vuelve a ocultar al enviar", async () => {
    await openCreate({});
    typeBoth(SECRET);

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    expect(passwordField().type).toBe("text");
    expect(confirmField().type).toBe("text");
    expect(passwordField().value).toBe(SECRET);

    fireEvent.click(screen.getByRole("button", { name: "Ocultar contraseña" }));
    expect(passwordField().type).toBe("password");
    expect(confirmField().type).toBe("password");

    fireEvent.click(screen.getByRole("button", { name: "Mostrar contraseña" }));
    fireEvent.click(createButton());
    expect(passwordField().type).toBe("password");
    expect(screen.getByRole("button", { name: "Mostrar contraseña" })).toBeDefined();
  });

  it("RF-7: con puntaje 2 avisa sin llamarla débil (no contradice «regular»)", async () => {
    await openCreate({ strength: () => 2 });

    fireEvent.input(passwordField(), { target: { value: "abcdefgh1" } });
    expect(await screen.findByText(/Fortaleza: regular \(2 de 4\)/)).toBeDefined();
    expect(screen.getByText(/Todavía es fácil de adivinar\. Puedes continuar/)).toBeDefined();
    expect(screen.queryByText(/débil/)).toBeNull();
  });

  it("RF-7: mientras escribe muestra la fortaleza y avisa si es débil, sin impedir continuar", async () => {
    const backend = await openCreate({});

    fireEvent.input(passwordField(), { target: { value: "123456" } });
    expect(await screen.findByText(/Fortaleza: muy débil \(0 de 4\)/)).toBeDefined();
    expect(screen.getByText(/Es una contraseña débil/)).toBeDefined();
    expect(screen.getByRole<HTMLMeterElement>("meter", { name: "Fortaleza" }).value).toBe(0);

    fireEvent.input(passwordField(), { target: { value: "k7#Qz!m2Lp$9vR@x4Tw&" } });
    expect(await screen.findByText(/Fortaleza: muy buena \(4 de 4\)/)).toBeDefined();
    expect(screen.queryByText(/Es una contraseña débil/)).toBeNull();

    // Weak does not block: it is created anyway.
    await nameIt();
    withoutKeyFile();
    typeBoth("123456");
    fireEvent.click(createButton());
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
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
    const backend = await openCreate({});
    await nameIt();
    typeBoth(SECRET, `${SECRET}x`);

    fireEvent.click(createButton());

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe("Las dos contraseñas no son iguales. Escríbelas otra vez.");
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");
    expect(confirmField().value).toBe("");
    expect(confirmField().getAttribute("aria-invalid")).toBe("true");
    expect(confirmField().getAttribute("aria-describedby")).toBe(alert.id);
  });

  it("RF-8: mientras escribe la confirmación dice que no son iguales y lo retira cuando coinciden", async () => {
    await openCreate({});

    fireEvent.input(passwordField(), { target: { value: SECRET } });
    expect(screen.queryByText("Las dos contraseñas no son iguales.")).toBeNull();
    expect(confirmField().hasAttribute("aria-describedby")).toBe(false);

    fireEvent.input(confirmField(), { target: { value: "maestra" } });
    const message = screen.getByText("Las dos contraseñas no son iguales.");
    expect(confirmField().getAttribute("aria-invalid")).toBe("true");
    expect(confirmField().getAttribute("aria-describedby")).toBe(message.id);

    fireEvent.input(confirmField(), { target: { value: SECRET } });
    expect(screen.queryByText("Las dos contraseñas no son iguales.")).toBeNull();
    expect(confirmField().getAttribute("aria-invalid")).toBe("false");
  });

  it("RF-3: error — sin contraseña ni archivo llave dice que falta la llave y no crea", async () => {
    const backend = await openCreate({});
    await nameIt();
    withoutKeyFile();

    fireEvent.click(createButton());

    expect(await screen.findByText(ERROR_MESSAGES.LlaveVacia)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
  });

  it("RF-99: RF-3: con el archivo llave nuevo junto a la bóveda y sin contraseña maestra no crea y pide una", async () => {
    const backend = await openCreate({});
    await nameIt();
    // The key file alone, beside the vault, would protect nothing.
    withNewKeyFile();

    fireEvent.click(createButton());

    expect(await screen.findByText(KEY_BESIDE_NEEDS_PASSWORD)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
  });

  it("RF-72: los dos campos de la contraseña maestra van marcados como valor protegido", async () => {
    await openCreate({});
    for (const id of ["create-password", "create-confirm"]) {
      expect(document.getElementById(id)?.hasAttribute("data-secret")).toBe(true);
    }
  });

  it("RF-1: error — sin ruta elegida (diálogo cancelado) pide elegirla y no crea", async () => {
    // No Documents folder, so there is no proposal; the person opens the dialog and cancels it.
    const backend = await openCreate({ picks: { pick_vault_to_create: [null] } }, (fake) => {
      fake.rejectOnce("vault_create_location", { code: "ErrorInterno", message: "core text" });
    });
    typeName("Familia");
    fireEvent.click(await screen.findByRole("button", { name: "Elegir ubicación…" }));
    await waitForTicks();
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect((await screen.findByRole("alert")).textContent).toBe(NO_PATH);
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");
  });

  it("RF-9: éxito — con archivo llave nuevo avisa antes de crear y otra vez al terminar", async () => {
    const backend = await openCreate({});
    await nameIt();
    withNewKeyFile();

    // Before: the notice names the key file, and the help of the marked option asks for a copy.
    expect(screen.getByText(/Si pierdes la contraseña maestra o el archivo llave, nadie puede abrir la bóveda/)).toBeDefined();
    expect(keyHelp()).toMatch(/^Se guarda junto a la bóveda: .*\. Guarda una copia en otro lugar\.$/);
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Bóveda creada" })).toBeDefined();
    expect(screen.getByText(/Sin una copia de ese archivo y sin la contraseña no hay forma de recuperarla/)).toBeDefined();
    expect(screen.getByText(FAMILY_KEY)).toBeDefined();
    expect(createCalls(backend)[0]?.args).toEqual({
      path: FAMILY_PATH,
      name: "Familia",
      password: SECRET,
      key_file: { kind: "generate", path: FAMILY_KEY },
    });

    fireEvent.click(screen.getByRole("button", { name: "Ir a la bóveda" }));
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
  });

  it("RF-1: éxito — solo con un archivo llave existente, sin contraseña", async () => {
    const backend = await openCreate({ files: [KEY], picks: { pick_key_file: [KEY] } });
    await nameIt();
    fireEvent.click(keyOption("Usar uno mío"));
    expect(screen.getByText("Eliges un archivo llave que ya tienes.")).toBeDefined();
    fireEvent.click(createButton());
    expect(await screen.findByText("Elige tu archivo llave o marca «Sin archivo llave».")).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: "Elegir archivo llave…" }));
    // The name of the chosen file is shown; its whole path is one hover away.
    expect((await screen.findByText("mia.keyx")).title).toBe(KEY);
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Bóveda creada" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toEqual({
      path: FAMILY_PATH,
      name: "Familia",
      key_file: { kind: "existing", path: KEY },
    });
  });

  it("RF-1: carga — mientras crea muestra progreso, no se congela y no lanza una segunda creación", async () => {
    const backend = await openCreate({});
    await nameIt();
    withoutKeyFile();
    const release = backend.hold("vault_create");
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect(await screen.findByRole("progressbar", { name: "Creando la bóveda" })).toBeDefined();
    expect(screen.getByText(/derivando la llave/)).toBeDefined();
    expect(createButton().matches(":disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Cancelar" }).matches(":disabled")).toBe(true);
    const form = createButton().closest("form");
    if (form === null) throw new Error("missing form");
    fireEvent.submit(form);
    await waitFor(() => {
      expect(backend.count("vault_create")).toBe(1);
    });
    fireEvent.submit(form);
    expect(backend.count("vault_create")).toBe(1);

    await act(async () => {
      release();
      await Promise.resolve();
    });
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
  });

  it("RF-1: Cancelar vuelve a la bienvenida sin crear nada", async () => {
    const backend = await openCreate({});
    await nameIt();
    typeBoth(SECRET);

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
  });

  it("RF-4: error — si el archivo ya existe lo dice, no lo toca y vacía los campos", async () => {
    // The location came from the Windows dialog, which may have asked "replace it?": the message says so.
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await nameIt();
    await chooseLocation();
    withoutKeyFile();
    expect(screen.getByText(PATH_HELP)).toBeDefined();
    // The file appears between the question about the location and the creation: the core is the one that refuses.
    backend.rejectOnce("vault_create", { code: "ArchivoExiste", message: "core text" });
    typeBoth(SECRET);

    fireEvent.click(createButton());

    const alert = await screen.findByText(EXISTS_CHOSEN);
    expect(alert.textContent).not.toContain("ArchivoExiste");
    expect(alert.textContent).toContain("aunque Windows haya preguntado");
    expect(alert.id).toBe("create-location-error");
    expect(screen.getByRole("button", { name: "Cambiar ubicación…" }).getAttribute("aria-describedby")).toBe(alert.id);
    expect(backend.count("vault_create")).toBe(1);
    expect(passwordField().value).toBe("");
    expect(screen.getByRole("heading", { name: "Crear bóveda" })).toBeDefined();
  });

  it("RF-4: con otra ubicación elegida, un archivo que ya existe se avisa antes y no se crea", async () => {
    const backend = await openCreate({
      vaults: [{ path: NEW_PATH, name: "Otra", password: "otra" }],
      picks: { pick_vault_to_create: [NEW_PATH] },
    });
    await nameIt();
    await chooseLocation();
    withoutKeyFile();

    expect(screen.getByText(EXISTS_CHOSEN)).toBeDefined();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    await waitFor(() => {
      expect(createButton().matches(":disabled")).toBe(false);
    });
    expect(screen.getByText(EXISTS_CHOSEN)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);
  });

  it("RF-23: error — no se puede escribir en esa carpeta: lo dice y deja volver a intentarlo", async () => {
    const backend = await openCreate({});
    await nameIt();
    withoutKeyFile();
    backend.rejectOnce("vault_create", { code: "GuardadoFallido", message: "core text" });
    typeBoth(SECRET);

    fireEvent.click(createButton());

    expect(await screen.findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(passwordField().value).toBe("");
    expect(createButton().matches(":disabled")).toBe(false);

    typeBoth(SECRET);
    fireEvent.click(createButton());
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
  });

  it("RF-32: un bloqueo del núcleo retira el formulario a medio llenar", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await nameIt();
    await chooseLocation();
    typeBoth(SECRET);

    act(() => {
      backend.lockFromCore();
    });

    expect(await screen.findByRole("heading", { name: "Bienvenida" })).toBeDefined();
    expect(screen.queryByText(NEW_PATH)).toBeNull();
    expect(screen.queryByLabelText("Nombre de la bóveda")).toBeNull();
  });

  it("RF-96: el nombre es el primer campo, tiene el foco al abrir y vacío no deja crear", async () => {
    const backend = await openCreate({});

    const fields = [...document.querySelectorAll("form input")];
    expect(fields[0]).toBe(nameField());
    expect(document.activeElement).toBe(nameField());

    typeName("   ");
    withoutKeyFile();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe(NO_NAME);
    expect(nameField().getAttribute("aria-invalid")).toBe("true");
    expect(nameField().getAttribute("aria-describedby")).toContain(alert.id);
    expect(document.activeElement).toBe(nameField());
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");

    // The message goes away as soon as the name is being fixed.
    typeName("Familia");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("RF-96: el nombre escrito es el que se envía, aunque el archivo se llame distinto", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    // Windows refuses `:` and `?` in a file name: the core drops them from the file, never from the name.
    await nameIt("Casa: ¿quién?", "Casa ¿quién.kdbx");
    withoutKeyFile();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Casa: ¿quién?" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toEqual({
      path: `${FAKE_DEFAULT_FOLDER}/Casa ¿quién.kdbx`,
      name: "Casa: ¿quién?",
      password: SECRET,
      key_file: { kind: "none" },
    });
  });

  it("RF-96: con otra ubicación elegida el nombre sigue siendo el escrito, no el del archivo", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await nameIt();
    await chooseLocation();
    withoutKeyFile();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toEqual({
      path: NEW_PATH,
      name: "Familia",
      password: SECRET,
      key_file: { kind: "none" },
    });
  });

  it("RF-97: la línea de ubicación sigue al nombre con el archivo que responde el núcleo", async () => {
    const backend = await openCreate({});

    // Empty name: the core proposes its default file.
    const first = await screen.findByText("bóveda.kdbx");
    expect(first.title).toBe(`${FAKE_DEFAULT_FOLDER}/bóveda.kdbx`);
    expect(screen.getByText("Se guarda en Documentos › AlmarioKS ›")).toBeDefined();
    expect(nameField().getAttribute("aria-describedby")).toBe("create-location");
    expect(backend.calls.filter((call) => call.command === "vault_create_location")).toEqual([
      { command: "vault_create_location", args: { name: "", chosen_path: null } },
    ]);

    // A burst of keystrokes is one question, asked with the name as it ended up.
    typeName("F");
    typeName("Fam");
    typeName("Familia");
    expect((await screen.findByText("Familia.kdbx")).title).toBe(FAMILY_PATH);
    expect(backend.count("vault_create_location")).toBe(2);

    // The file name is the core's, not one worked out in the window.
    backend.respondOnce("vault_create_location", {
      vault_path: `${FAKE_DEFAULT_FOLDER}/lo-que-diga-el-nucleo.kdbx`,
      vault_file: "lo-que-diga-el-nucleo.kdbx",
      key_file_path: `${FAKE_DEFAULT_FOLDER}/lo-que-diga-el-nucleo.keyx`,
      key_file: "lo-que-diga-el-nucleo.keyx",
      in_default_folder: true,
      vault_exists: false,
      key_file_exists: false,
    });
    typeName("Trabajo");
    expect(await screen.findByText("lo-que-diga-el-nucleo.kdbx")).toBeDefined();
    expect(screen.queryByText("Trabajo.kdbx")).toBeNull();
  });

  it("RF-97: descarta la respuesta vieja de una ubicación", async () => {
    const backend = await openCreate({});
    await screen.findByText("bóveda.kdbx");

    const release = backend.hold("vault_create_location");
    typeName("Fam");
    await waitFor(() => {
      expect(backend.count("vault_create_location")).toBe(2);
    });
    typeName("Familia");
    expect(await screen.findByText("Familia.kdbx")).toBeDefined();

    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(screen.getByText("Familia.kdbx")).toBeDefined();
    expect(screen.queryByText("Fam.kdbx")).toBeNull();
  });

  it("RF-97: al crear dentro del retardo usa el archivo del nombre como quedó, no el de una tecla anterior", async () => {
    const backend = await openCreate({});
    await nameIt("Fam");
    withoutKeyFile();

    // The line still says "Fam.kdbx" when the form is sent.
    typeName("Familia");
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toMatchObject({ path: FAMILY_PATH, name: "Familia" });
  });

  it("RF-97: una ubicación elegida se muestra entera y deja de seguir al nombre", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await nameIt();

    await chooseLocation();
    expect(screen.getByText(NEW_PATH).title).toBe(NEW_PATH);
    expect(screen.getByText("Se guarda en")).toBeDefined();
    expect(screen.queryByText("Se guarda en Documentos › AlmarioKS ›")).toBeNull();
    expect(backend.calls.filter((call) => call.command === "vault_create_location").at(-1)?.args).toEqual({
      name: "Familia",
      chosen_path: NEW_PATH,
    });

    const asked = backend.count("vault_create_location");
    typeName("Otra cosa");
    await waitForTicks(250);
    expect(screen.getByText(NEW_PATH)).toBeDefined();
    expect(screen.queryByText("Otra cosa.kdbx")).toBeNull();
    expect(backend.count("vault_create_location")).toBe(asked);
  });

  it("RF-97: sin Documentos pide elegir la ubicación y no crea hasta elegirla", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } }, (fake) => {
      fake.rejectOnce("vault_create_location", { code: "ErrorInterno", message: "core text" });
    });

    expect(await screen.findByText(/Windows no dio la carpeta Documentos\. Elige dónde guardar la bóveda\./)).toBeDefined();
    expect(screen.queryByText(/Se guarda en/)).toBeNull();
    // Without a Documents folder the name has nowhere to be proposed: the core is not asked again.
    typeName("Familia");
    await waitForTicks(250);
    expect(backend.count("vault_create_location")).toBe(1);

    withoutKeyFile();
    typeBoth(SECRET);
    fireEvent.click(createButton());
    expect((await screen.findByRole("alert")).textContent).toBe(NO_PATH);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Elegir ubicación…" }));
    expect(backend.count("vault_create")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: "Elegir ubicación…" }));
    await screen.findByText(NEW_PATH);
    expect(screen.queryByRole("alert")).toBeNull();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toMatchObject({ path: NEW_PATH, name: "Familia" });
  });

  it("RF-98: avisa bajo el nombre, antes de pulsar Crear, si ya hay un archivo con ese nombre, y no crea", async () => {
    const backend = await openCreate({ vaults: [{ path: FAMILY_PATH, name: "Familia", password: "otra" }] });
    await nameIt();
    withoutKeyFile();

    const warning = await screen.findByText(EXISTS_VAULT);
    expect(warning.id).toBe("create-name-error");
    expect(warning.textContent).not.toContain("Windows");
    expect(nameField().getAttribute("aria-invalid")).toBe("true");
    expect(nameField().getAttribute("aria-describedby")).toContain(warning.id);

    typeBoth(SECRET);
    fireEvent.click(createButton());
    await waitFor(() => {
      expect(createButton().matches(":disabled")).toBe(false);
    });
    expect(screen.getByText(EXISTS_VAULT)).toBeDefined();
    // The walkthrough on the installed app reported a "passwords differ" message here: with both equal there is
    // none, and the focus goes back to the name, which is what has to change.
    expect(screen.queryByText(/no son iguales/)).toBeNull();
    await waitFor(() => {
      expect(document.activeElement).toBe(nameField());
    });
    expect(document.activeElement).toBe(nameField());
    expect(backend.count("vault_create")).toBe(0);
    expect(passwordField().value).toBe("");

    // Another name is free again.
    await nameIt("Familia 2");
    expect(screen.queryByText(EXISTS_VAULT)).toBeNull();
    expect(nameField().getAttribute("aria-invalid")).toBe("false");
  });

  it("RF-98: si aun así el núcleo responde que el archivo existe, muestra el mismo aviso", async () => {
    const backend = await openCreate({});
    await nameIt();
    expect(screen.queryByText(EXISTS_VAULT)).toBeNull();
    backend.rejectOnce("vault_create", { code: "ArchivoExiste", message: "core text" });
    typeBoth(SECRET);

    fireEvent.click(createButton());

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe(EXISTS_VAULT);
    expect(alert.id).toBe("create-name-error");
    expect(backend.count("vault_create")).toBe(1);
    expect(passwordField().value).toBe("");
    expect(screen.getByRole("heading", { name: "Crear bóveda" })).toBeDefined();
  });

  it("RF-99: al abrir sale marcada «Sin archivo llave», la primera de las tres, y el grupo no dice «recomendado»", async () => {
    await openCreate({});

    const options = screen.getAllByRole<HTMLInputElement>("radio");
    expect(options.map((option) => option.closest("label")?.textContent)).toEqual([
      "Sin archivo llave",
      "Crear uno nuevo",
      "Usar uno mío",
    ]);
    expect(options.map((option) => option.checked)).toEqual([true, false, false]);
    expect(screen.getByRole("group", { name: "Archivo llave (opcional)" })).toBeDefined();
    expect(screen.queryByText(/recomendado/)).toBeNull();
    expect(keyHelp()).toBe(KEY_HELP_NONE);
  });

  it("RF-99: sin tocar las opciones del archivo llave no se crea ningún archivo llave", async () => {
    const backend = await openCreate({});
    await nameIt();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    // No "Bóveda creada" step about a key file: straight to the vault.
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
    expect(createCalls(backend).map((call) => call.args)).toEqual([
      { path: FAMILY_PATH, name: "Familia", password: SECRET, key_file: { kind: "none" } },
    ]);
    expect(JSON.stringify(backend.calls)).not.toContain('"generate"');
    expect(backend.count("pick_new_key_file")).toBe(0);
    expect(backend.count("pick_key_file")).toBe(0);
  });

  it("RF-113: con «Crear uno nuevo» la ayuda dice dónde queda el archivo llave y sigue al nombre y a la ubicación", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });
    await nameIt();
    expect(keyHelp()).toBe(KEY_HELP_NONE);

    withNewKeyFile();
    expect(keyHelp()).toBe(
      "Se guarda junto a la bóveda: Documentos › AlmarioKS › Familia.keyx. Guarda una copia en otro lugar.",
    );
    const beside = screen.getByText("Familia.keyx");
    expect(beside.title).toBe(FAMILY_KEY);
    expect(beside.classList.contains("mono")).toBe(true);
    // The fieldset is described by that line.
    expect(screen.getByRole("group", { name: "Archivo llave (opcional)" }).getAttribute("aria-describedby")).toBe(
      "create-key-help",
    );

    // It follows the name, with the file the core answers (not one worked out in the window).
    await nameIt("Trabajo");
    expect(keyHelp()).toBe(
      "Se guarda junto a la bóveda: Documentos › AlmarioKS › Trabajo.keyx. Guarda una copia en otro lugar.",
    );
    backend.respondOnce("vault_create_location", {
      vault_path: `${FAKE_DEFAULT_FOLDER}/lo-que-diga-el-nucleo.kdbx`,
      vault_file: "lo-que-diga-el-nucleo.kdbx",
      key_file_path: `${FAKE_DEFAULT_FOLDER}/la-llave-del-nucleo.keyx`,
      key_file: "la-llave-del-nucleo.keyx",
      in_default_folder: true,
      vault_exists: false,
      key_file_exists: false,
    });
    await nameIt("Casa", "lo-que-diga-el-nucleo.kdbx");
    expect(screen.getByText("la-llave-del-nucleo.keyx").title).toBe(`${FAKE_DEFAULT_FOLDER}/la-llave-del-nucleo.keyx`);
    expect(screen.queryByText("Casa.keyx")).toBeNull();

    // And the location chosen with the dialog: the whole path, like the line of the vault.
    await chooseLocation();
    expect(keyHelp()).toBe("Se guarda junto a la bóveda: D:/bovedas/nueva.keyx. Guarda una copia en otro lugar.");
    expect(screen.getByText("D:/bovedas/nueva.keyx").title).toBe("D:/bovedas/nueva.keyx");

    // The other options keep their own line.
    fireEvent.click(keyOption("Usar uno mío"));
    expect(keyHelp()).toBe(KEY_HELP_EXISTING);
  });

  it("RF-113: sin ubicación resuelta la ayuda solo dice que queda junto a la bóveda", async () => {
    await openCreate({}, (fake) => {
      fake.rejectOnce("vault_create_location", { code: "ErrorInterno", message: "core text" });
    });
    await screen.findByText(/Windows no dio la carpeta Documentos/);

    withNewKeyFile();
    expect(keyHelp()).toBe(KEY_HELP_BESIDE);
  });

  it("RF-99: «Crear uno nuevo» crea el archivo llave junto a la bóveda, sin diálogo", async () => {
    const backend = await openCreate({ picks: { pick_vault_to_create: [NEW_PATH] } });

    expect(screen.getAllByRole("radio")).toHaveLength(3);
    expect(keyOption("Usar uno mío").checked).toBe(false);

    // The help line changes with the option.
    withNewKeyFile();
    expect(keyOption("Crear uno nuevo").checked).toBe(true);
    expect(keyOption("Sin archivo llave").checked).toBe(false);
    expect(keyHelp()).toMatch(/^Se guarda junto a la bóveda/);
    fireEvent.click(keyOption("Sin archivo llave"));
    expect(screen.getByText("La bóveda se abre solo con la contraseña maestra.")).toBeDefined();
    withNewKeyFile();
    expect(keyHelp()).toMatch(/^Se guarda junto a la bóveda/);

    // Beside the vault wherever it goes: here, in the location chosen with the dialog.
    await nameIt();
    await chooseLocation();
    typeBoth(SECRET);
    fireEvent.click(createButton());

    expect(await screen.findByRole("heading", { name: "Bóveda creada" })).toBeDefined();
    expect(createCalls(backend)[0]?.args).toEqual({
      path: NEW_PATH,
      name: "Familia",
      password: SECRET,
      key_file: { kind: "generate", path: "D:/bovedas/nueva.keyx" },
    });
    expect(backend.count("pick_new_key_file")).toBe(0);
    expect(backend.count("pick_key_file")).toBe(0);
  });

  it("RF-99: si el archivo llave ya existe lo dice y no crea; con otra opción sí", async () => {
    const backend = await openCreate({ files: [FAMILY_KEY] });
    await nameIt();
    // Nothing is said about a key file nobody asked to create.
    expect(screen.queryByText(EXISTS_KEY)).toBeNull();
    withNewKeyFile();

    const warning = await screen.findByText(EXISTS_KEY);
    expect(warning.id).toBe("create-key-error");
    typeBoth(SECRET);
    fireEvent.click(createButton());
    await waitFor(() => {
      expect(createButton().matches(":disabled")).toBe(false);
    });
    expect(screen.getByText(EXISTS_KEY)).toBeDefined();
    expect(backend.count("vault_create")).toBe(0);

    withoutKeyFile();
    expect(screen.queryByText(EXISTS_KEY)).toBeNull();
    typeBoth(SECRET);
    fireEvent.click(createButton());
    expect(await screen.findByRole("heading", { name: "Familia" })).toBeDefined();
  });
});
