import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeOptions } from "../ipc/fake";
import { savedAgo } from "./OpenVault";
import { VaultRoot } from "./VaultRoot";

/** What the foot of the rail says about the disk. */
const saveState = () => document.querySelector(".save-state")?.textContent ?? "";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const PATH = "D:/bovedas/cuentas.kdbx";
const NEW_PATH = "D:/bovedas/cuentas-4.kdbx";
const OPEN = {
  state: "open",
  path: PATH,
  name: "Cuentas",
  read_only: false,
  weak_kdf: false,
  save_pending: false,
  recovery: null,
} as const;
const FILE = { path: PATH, name: "Cuentas", password: "correcta" };

async function setup(options: FakeOptions = {}) {
  const backend = createFakeBackend({
    status: OPEN,
    vaults: [FILE],
    groups: SAMPLE_GROUPS,
    entries: SAMPLE_ENTRIES,
    ...options,
  });
  render(<VaultRoot client={createClient(backend.transport)} />);
  await screen.findByRole("heading", { name: "Cuentas" });
  // The tree arrives after the effects of the open vault ran, so the `vault-save-state` listener is in place.
  await waitFor(() => {
    expect(screen.queryByText("Cargando los grupos…")).toBeNull();
  });
  return backend;
}

function emitSave(backend: { emit: (event: string, payload?: unknown) => void }, state: string) {
  act(() => {
    backend.emit("vault-save-state", state);
  });
}

const failureTitle = "No se pudo guardar: tus cambios siguen en memoria";

async function unlockAgain() {
  await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" });
  fireEvent.change(screen.getByLabelText("Contraseña maestra"), { target: { value: "correcta" } });
  fireEvent.click(screen.getByRole("button", { name: "Desbloquear" }));
  await screen.findByRole("heading", { name: "Cuentas" });
}

describe("P4 Bóveda — guardado", () => {
  it("RF-89: en reposo al abrir dice «Guardado», y tras un guardado, hace cuánto: «hace un momento», «hace N min», «hace N h»", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = await setup();
    expect(saveState()).toBe("Guardado");
    expect(document.querySelector(".save-state")?.getAttribute("data-state")).toBe("idle");

    emitSave(backend, "saved");
    expect(saveState()).toBe("Guardado hace un momento");
    // The time is for the eye: what assistive technology hears is "Guardado", once.
    const ago = document.querySelector(".save-state [aria-hidden]");
    expect(ago?.textContent).toBe(" hace un momento");
    expect(ago?.getAttribute("aria-hidden")).toBe("true");

    await act(() => vi.advanceTimersByTimeAsync(130_000));
    expect(saveState()).toBe("Guardado hace 2 min");

    // Another save starts the count again.
    emitSave(backend, "saving");
    expect(saveState()).toBe("Guardando…");
    emitSave(backend, "saved");
    expect(saveState()).toBe("Guardado hace un momento");
    vi.useRealTimers();

    expect(savedAgo(0, 59_999)).toBe("hace un momento");
    expect(savedAgo(0, 60_000)).toBe("hace 1 min");
    expect(savedAgo(0, 59 * 60_000)).toBe("hace 59 min");
    expect(savedAgo(0, 3 * 3_600_000 + 5)).toBe("hace 3 h");
    // A clock that went back never shows a negative time.
    expect(savedAgo(10_000, 0)).toBe("hace un momento");
  });

  it("RF-89: RF-14: una bóveda de solo lectura no dice «Guardado»: nunca guarda", async () => {
    await setup({ status: { ...OPEN, read_only: true } });
    expect(saveState()).toBe("");
  });

  it("RF-89: carga — `saving` muestra «Guardando…» y `saved` lo cambia por «Guardado.» sin modales", async () => {
    const backend = await setup();

    emitSave(backend, "saving");
    expect(screen.getByText("Guardando…")).toBeDefined();
    emitSave(backend, "saved");
    expect(saveState()).toBe("Guardado hace un momento");
    expect(screen.queryByText("Guardando…")).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.queryByRole("alert", { name: failureTitle })).toBeNull();
  });

  it("RF-23: error — `failed` deja un aviso persistente y «Reintentar el guardado» lo resuelve", async () => {
    const backend = await setup();

    emitSave(backend, "failed");
    expect(screen.getByRole("heading", { name: failureTitle })).toBeDefined();
    const alert = screen.getByRole("heading", { name: failureTitle }).closest('[role="alert"]');
    expect(alert?.textContent).toMatch(/ciérrala y\s+reintenta/);

    fireEvent.click(screen.getByRole("button", { name: "Reintentar el guardado" }));
    await waitFor(() => {
      expect(saveState()).toBe("Guardado hace un momento");
    });
    expect(backend.count("vault_retry_save")).toBe(1);
    expect(screen.queryByRole("heading", { name: failureTitle })).toBeNull();
    // The button that had focus went away with the notice: focus is on the vault title, not lost.
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Cuentas", level: 2 }));
    });
  });

  it("RF-23: error — si el reintento vuelve a fallar lo dice y el aviso sigue", async () => {
    const backend = await setup();
    emitSave(backend, "failed");
    backend.rejectOnce("vault_retry_save", { code: "GuardadoFallido", message: "PermissionDenied" });

    fireEvent.click(screen.getByRole("button", { name: "Reintentar el guardado" }));

    expect(await screen.findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(ERROR_MESSAGES.GuardadoFallido).toMatch(/otra app tiene abierto el archivo/);
    expect(screen.getByRole("heading", { name: failureTitle })).toBeDefined();
    expect(document.body.textContent).not.toMatch(/PermissionDenied/);
  });

  it("RF-23: éxito — «Guardar en otro lugar…» usa el diálogo del núcleo y vault_save_as", async () => {
    const backend = await setup({ picks: { pick_vault_to_create: [null, NEW_PATH] } });
    emitSave(backend, "failed");

    fireEvent.click(screen.getByRole("button", { name: "Guardar en otro lugar…" }));
    await waitFor(() => {
      expect(backend.count("pick_vault_to_create")).toBe(1);
    });
    expect(backend.count("vault_save_as")).toBe(0);
    expect(screen.getByRole("heading", { name: failureTitle })).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Guardar en otro lugar…" }));
    await waitFor(() => {
      expect(saveState()).toBe("Guardado hace un momento");
    });
    expect(backend.calls.filter((call) => call.command === "vault_save_as")).toEqual([
      { command: "vault_save_as", args: { path: NEW_PATH } },
    ]);
    // The rail shows no paths: the file of the vault is in the `title` of its name.
    expect(screen.getByRole("heading", { level: 2 }).title).toBe(NEW_PATH);
    expect(screen.queryByText(NEW_PATH)).toBeNull();
  });

  it("RF-24: error — `external-change` ofrece guardar en otro lugar o recargar, confirmando qué se pierde", async () => {
    const backend = await setup();
    emitSave(backend, "external-change");

    expect(screen.getByRole("heading", { name: "El archivo cambió en disco" })).toBeDefined();
    expect(screen.queryByRole("button", { name: "Reintentar el guardado" })).toBeNull();
    expect(screen.getByRole("button", { name: "Guardar en otro lugar…" })).toBeDefined();
    const trees = backend.count("group_tree");

    fireEvent.click(screen.getByRole("button", { name: "Recargar descartando lo pendiente…" }));
    const dialog = await screen.findByRole("dialog", { name: "Recargar el archivo del disco" });
    expect(within(dialog).getByText(/no se puede recuperar/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.count("vault_reload")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: "Recargar descartando lo pendiente…" }));
    fireEvent.click(
      within(await screen.findByRole("dialog")).getByRole("button", { name: "Recargar y descartar" }),
    );

    expect(await screen.findByText(/Se recargó el archivo del disco/)).toBeDefined();
    expect(backend.count("vault_reload")).toBe(1);
    expect(screen.queryByRole("heading", { name: "El archivo cambió en disco" })).toBeNull();
    // The vault screens asked the core again: what was on screen may be gone from the file.
    await waitFor(() => {
      expect(backend.count("group_tree")).toBeGreaterThan(trees);
    });
  });

  it("RF-24: error — un CambioExterno al reintentar cambia el aviso al del archivo cambiado", async () => {
    const backend = await setup();
    emitSave(backend, "failed");
    backend.rejectOnce("vault_retry_save", { code: "CambioExterno", message: "x" });

    fireEvent.click(screen.getByRole("button", { name: "Reintentar el guardado" }));

    expect(await screen.findByRole("heading", { name: "El archivo cambió en disco" })).toBeDefined();
    expect(screen.getByText(ERROR_MESSAGES.CambioExterno)).toBeDefined();
  });

  it("RF-23: `save_pending` del estado pinta el mismo aviso al entrar", async () => {
    await setup({ status: { ...OPEN, save_pending: true } });

    expect(screen.getByRole("heading", { name: failureTitle })).toBeDefined();
    expect(screen.getByRole("button", { name: "Reintentar el guardado" })).toBeDefined();
  });
});

describe("P4 Bóveda — solo lectura y protección débil", () => {
  it("RF-14: KDBX 3.x — aviso fijo y sin acciones que cambian la bóveda; ver y copiar sí", async () => {
    await setup({ status: { ...OPEN, read_only: true } });

    expect(screen.getByRole("heading", { name: "Bóveda en solo lectura" })).toBeDefined();
    expect(screen.getByText(/KDBX 3\.x/)).toBeDefined();
    fireEvent.click(await screen.findByRole("button", { name: /^Personal / }));
    await screen.findByRole("heading", { name: "Entradas de Personal" });
    expect(screen.queryByRole("button", { name: "Nueva entrada" })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Más acciones de / })).toBeNull();
    expect(screen.queryByRole("button", { name: "Nuevo grupo" })).toBeNull();

    // RF-106: a double click on the title opens the window of the entry.
    fireEvent.click(await screen.findByRole("button", { name: "Banco" }));
    fireEvent.doubleClick(screen.getByRole("button", { name: "Banco" }));
    await screen.findByRole("heading", { name: "Banco", level: 4 });
    expect(screen.queryByRole("button", { name: "Editar" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Enviar a la papelera" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Mover a otro grupo…" })).toBeNull();
    expect(screen.getByRole("button", { name: "Copiar usuario" })).toBeDefined();
  });

  it("RF-15: convertir a KDBX 4 escribe en una ruta nueva, dice que el original no se toca y deja editar", async () => {
    const backend = await setup({
      status: { ...OPEN, read_only: true },
      picks: { pick_vault_to_create: [NEW_PATH] },
    });
    expect(screen.getByText(/el archivo original no\s+se toca/)).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Convertir a KDBX 4…" }));

    expect(await screen.findByText(/El archivo original quedó igual/)).toBeDefined();
    expect(backend.calls.filter((call) => call.command === "vault_convert")).toEqual([
      { command: "vault_convert", args: { new_path: NEW_PATH } },
    ]);
    expect(screen.queryByRole("heading", { name: "Bóveda en solo lectura" })).toBeNull();
    expect(await screen.findByRole("button", { name: "Nueva entrada" })).toBeDefined();
    // The rail shows no paths: the file of the vault is in the `title` of its name.
    expect(screen.getByRole("heading", { level: 2 }).title).toBe(NEW_PATH);
    expect(screen.queryByText(NEW_PATH)).toBeNull();
  });

  it("RF-15: error — si la conversión falla lo dice junto al botón y la bóveda sigue en solo lectura", async () => {
    const backend = await setup({
      status: { ...OPEN, read_only: true },
      picks: { pick_vault_to_create: [NEW_PATH] },
    });
    backend.rejectOnce("vault_convert", { code: "ArchivoExiste", message: "x" });

    fireEvent.click(screen.getByRole("button", { name: "Convertir a KDBX 4…" }));

    expect(await screen.findByText(ERROR_MESSAGES.ArchivoExiste)).toBeDefined();
    expect(screen.getByRole("heading", { name: "Bóveda en solo lectura" })).toBeDefined();
  });

  it("RF-14: si un comando responde SoloLectura, se muestra su mensaje", async () => {
    const backend = await setup();
    backend.rejectOnce("group_create", { code: "SoloLectura", message: "x" });
    fireEvent.click(await screen.findByRole("button", { name: /^Personal / }));
    await screen.findByRole("heading", { name: "Entradas de Personal" });

    fireEvent.click(screen.getByRole("button", { name: "Nuevo grupo" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Nuevo" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Crear grupo" }));

    expect(await within(dialog).findByText(ERROR_MESSAGES.SoloLectura)).toBeDefined();
  });

  it("RF-17: protección débil — explica qué significa y qué hacer, y se cierra por esta sesión", async () => {
    const backend = await setup({
      status: { ...OPEN, weak_kdf: true },
      vaults: [{ ...FILE, weakKdf: true }],
    });

    expect(screen.getByRole("heading", { name: "Protección débil contra fuerza bruta" })).toBeDefined();
    expect(screen.getByText(/KeePass 2 o KeePassXC/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Cerrar este aviso por esta sesión" }));
    expect(screen.queryByRole("heading", { name: "Protección débil contra fuerza bruta" })).toBeNull();

    act(() => {
      backend.lockFromCore();
    });
    await unlockAgain();
    expect(screen.queryByRole("heading", { name: "Protección débil contra fuerza bruta" })).toBeNull();
  });
});

describe("P4 Bóveda — bloqueo", () => {
  it("RF-32: tras vault-locked desaparecen el aviso de guardado y el diálogo de recarga", async () => {
    const backend = await setup();
    emitSave(backend, "external-change");
    fireEvent.click(screen.getByRole("button", { name: "Recargar descartando lo pendiente…" }));
    await screen.findByRole("dialog");

    act(() => {
      backend.lockFromCore();
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.queryByRole("heading", { name: "El archivo cambió en disco" })).toBeNull();
    await unlockAgain();
    expect(screen.queryByRole("heading", { name: "El archivo cambió en disco" })).toBeNull();
    expect(screen.queryByRole("heading", { name: failureTitle })).toBeNull();
  });

  it("RF-32: un «Guardar en otro lugar» que responde después del bloqueo no vuelve a montar la bóveda", async () => {
    const backend = await setup({ picks: { pick_vault_to_create: [NEW_PATH] } });
    emitSave(backend, "failed");
    const release = backend.hold("vault_save_as");

    fireEvent.click(screen.getByRole("button", { name: "Guardar en otro lugar…" }));
    await waitFor(() => {
      expect(backend.count("vault_save_as")).toBe(1);
    });
    act(() => {
      backend.lockFromCore();
    });
    await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" });
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(screen.getByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(screen.queryByRole("heading", { name: "Cuentas" })).toBeNull();
  });
});
