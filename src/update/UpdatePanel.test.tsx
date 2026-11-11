import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { DownloadEvent, Update } from "@tauri-apps/plugin-updater";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UpdatePanel } from "./UpdatePanel";

const { check } = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check }));

type Install = (onEvent?: (event: DownloadEvent) => void) => Promise<void>;

function fakeUpdate(install: Install) {
  const downloadAndInstall = vi.fn(install);
  const update = { version: "0.0.2", body: "Arreglos de prueba.", downloadAndInstall };
  return { update: update as unknown as Update, downloadAndInstall };
}

/** Any IPC that escapes the mocked plugin (for example a process restart) is recorded here. */
let ipcCalls: string[] = [];

beforeEach(() => {
  ipcCalls = [];
  mockIPC((cmd) => {
    ipcCalls.push(cmd);
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.clearAllMocks();
});

function clickCheck() {
  fireEvent.click(screen.getByRole("button", { name: "Buscar actualizaciones" }));
}

describe("UpdatePanel (ADR-0003: update only with consent)", () => {
  it("ADR-0003: does not check for updates until the user clicks", () => {
    render(<UpdatePanel />);
    expect(check).not.toHaveBeenCalled();
    expect(ipcCalls).toEqual([]);
  });

  it("ADR-0003: check alone never downloads; it shows version and notes and waits", async () => {
    const { update, downloadAndInstall } = fakeUpdate(() => Promise.resolve());
    check.mockResolvedValue(update);
    render(<UpdatePanel />);

    clickCheck();

    expect(await screen.findByText("Hay una versión nueva: 0.0.2.")).toBeDefined();
    expect(screen.getByText("Arreglos de prueba.")).toBeDefined();
    expect(screen.getByRole("button", { name: "Actualizar" })).toBeDefined();
    expect(check).toHaveBeenCalledTimes(1);
    expect(downloadAndInstall).not.toHaveBeenCalled();
  });

  it('ADR-0003: "Ahora no" goes back without downloading', async () => {
    const { update, downloadAndInstall } = fakeUpdate(() => Promise.resolve());
    check.mockResolvedValue(update);
    render(<UpdatePanel />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Ahora no" }));

    expect(screen.getByRole("button", { name: "Buscar actualizaciones" })).toBeDefined();
    expect(downloadAndInstall).not.toHaveBeenCalled();
  });

  it("says when the app is up to date", async () => {
    check.mockResolvedValue(null);
    render(<UpdatePanel />);
    clickCheck();

    expect(await screen.findByText("Tienes la versión más reciente.")).toBeDefined();
  });

  it('ADR-0003: only "Actualizar" downloads, shows progress and then installing', async () => {
    let emit: ((event: DownloadEvent) => void) | undefined;
    let finish: (() => void) | undefined;
    const { update, downloadAndInstall } = fakeUpdate(
      (onEvent) =>
        new Promise<void>((resolve) => {
          emit = onEvent;
          finish = resolve;
        }),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);

    emit?.({ event: "Started", data: { contentLength: 200 } });
    emit?.({ event: "Progress", data: { chunkLength: 50 } });
    expect(await screen.findByText("Descargando la actualización: 25 %.")).toBeDefined();
    expect(screen.getByRole("progressbar", { name: "Progreso de la descarga" })).toBeDefined();

    emit?.({ event: "Finished" });
    finish?.();
    expect(await screen.findByText(/Firma verificada\. Instalando/)).toBeDefined();
  });

  it("AMENAZAS U1: an invalid signature shows the message, installs nothing and does not restart", async () => {
    const { update } = fakeUpdate(() =>
      Promise.reject(new Error("The signature verification failed")),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    expect(
      await screen.findByText(/La firma de la actualización no es válida: no se instaló\./),
    ).toBeDefined();
    expect(screen.queryByText(/Instalando/)).toBeNull();
    expect(screen.queryByText(/signature verification failed/i)).toBeNull();
    expect(ipcCalls.filter((cmd) => cmd.startsWith("plugin:process"))).toEqual([]);
  });

  it("AMENAZAS U1: an update signed with another key is treated as an invalid signature", async () => {
    const { update } = fakeUpdate(() =>
      Promise.reject(
        new Error("The signature was created with a different key than the one provided"),
      ),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    expect(
      await screen.findByText(/La firma de la actualización no es válida: no se instaló\./),
    ).toBeDefined();
  });

  it("without a connection it explains what happened and lets the user retry", async () => {
    check.mockRejectedValue(
      "error sending request for url (https://github.com/Kattonavi/almarioks/releases/latest/download/latest.json)",
    );
    render(<UpdatePanel />);
    clickCheck();

    expect(await screen.findByText(/No hay conexión con el servidor de actualizaciones/)).toBeDefined();
    expect(screen.queryByText(/error sending request/)).toBeNull();
    const retry = screen.getByRole("button", { name: "Buscar actualizaciones" });
    expect(retry.hasAttribute("disabled")).toBe(false);
  });
});
