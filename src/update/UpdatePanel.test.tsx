import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { DownloadEvent, Update } from "@tauri-apps/plugin-updater";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UpdatePanel } from "./UpdatePanel";

const { check } = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check }));

type Download = (onEvent?: (event: DownloadEvent) => void) => Promise<void>;

/** Double of the plugin's `Update`: `download` resolves only for a valid signature, `install` runs after. */
function fakeUpdate(downloadImpl: Download) {
  const download = vi.fn(downloadImpl);
  const install = vi.fn(() => Promise.resolve());
  const update = { version: "0.0.2", body: "Arreglos de prueba.", download, install };
  return { update: update as unknown as Update, download, install };
}

/** Like the plugin 2.14.1: it reports the whole download (Finished included) and only THEN verifies. */
function downloadThenReject(message: string): Download {
  return async (onEvent) => {
    onEvent?.({ event: "Started", data: { contentLength: 100 } });
    onEvent?.({ event: "Progress", data: { chunkLength: 100 } });
    onEvent?.({ event: "Finished" });
    // Let React paint the intermediate state before the verification fails.
    await new Promise((resolve) => setTimeout(resolve, 0));
    throw new Error(message);
  };
}

/** Every text the status region announced, in order (not only the final one). */
function recordAnnouncements(): string[] {
  const announced: string[] = [];
  const observer = new MutationObserver(() => {
    const text = document.querySelector('[role="status"]')?.textContent ?? "";
    if (text !== "" && announced.at(-1) !== text) announced.push(text);
  });
  observer.observe(document.body, { subtree: true, childList: true, characterData: true });
  return announced;
}

/** Any IPC that escapes the mocked plugin (for example a process restart) is recorded here. */
let ipcCalls: string[] = [];

/** Default `beforeInstall`: closing the vault succeeds. */
const prepared = vi.fn(() => Promise.resolve());

beforeEach(() => {
  ipcCalls = [];
  prepared.mockImplementation(() => Promise.resolve());
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
    render(<UpdatePanel beforeInstall={prepared} />);
    expect(check).not.toHaveBeenCalled();
    expect(ipcCalls).toEqual([]);
  });

  it("ADR-0003: check alone never downloads; it shows version and notes and waits", async () => {
    const { update, download } = fakeUpdate(() => Promise.resolve());
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);

    clickCheck();

    expect(await screen.findByText("Hay una versión nueva: 0.0.2.")).toBeDefined();
    expect(screen.getByText("Arreglos de prueba.")).toBeDefined();
    expect(screen.getByRole("button", { name: "Actualizar" })).toBeDefined();
    expect(check).toHaveBeenCalledTimes(1);
    expect(download).not.toHaveBeenCalled();
  });

  it('ADR-0003: "Ahora no" goes back without downloading', async () => {
    const { update, download } = fakeUpdate(() => Promise.resolve());
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Ahora no" }));

    expect(screen.getByRole("button", { name: "Buscar actualizaciones" })).toBeDefined();
    expect(download).not.toHaveBeenCalled();
  });

  it("says when the app is up to date", async () => {
    check.mockResolvedValue(null);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();

    expect(await screen.findByText("Tienes la versión más reciente.")).toBeDefined();
  });

  it('ADR-0003: only "Actualizar" downloads, shows progress, verifies and then installs', async () => {
    let emit: ((event: DownloadEvent) => void) | undefined;
    let finish: (() => void) | undefined;
    const { update, download, install } = fakeUpdate(
      (onEvent) =>
        new Promise<void>((resolve) => {
          emit = onEvent;
          finish = resolve;
        }),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();

    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));
    expect(download).toHaveBeenCalledTimes(1);

    emit?.({ event: "Started", data: { contentLength: 200 } });
    emit?.({ event: "Progress", data: { chunkLength: 50 } });
    expect(await screen.findByText("Descargando la actualización: 25 %.")).toBeDefined();
    expect(screen.getByRole("progressbar", { name: "Progreso de la descarga" })).toBeDefined();

    emit?.({ event: "Finished" });
    expect(await screen.findByText("Descarga completa. Verificando la firma…")).toBeDefined();
    expect(screen.queryByText(/Firma verificada/)).toBeNull();
    expect(install).not.toHaveBeenCalled();

    finish?.();
    expect(await screen.findByText(/Firma verificada\. Instalando/)).toBeDefined();
    expect(install).toHaveBeenCalledTimes(1);
  });

  it("without Content-Length the progress is indeterminate and shows no percentage", async () => {
    let emit: ((event: DownloadEvent) => void) | undefined;
    const { update } = fakeUpdate(
      (onEvent) =>
        new Promise<void>(() => {
          emit = onEvent;
        }),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();
    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    // The plugin serializes a missing Content-Length as null.
    emit?.({ event: "Started", data: { contentLength: null } } as unknown as DownloadEvent);
    emit?.({ event: "Progress", data: { chunkLength: 50 } });

    expect(await screen.findByText("Descargando la actualización…")).toBeDefined();
    expect(screen.queryByText(/%/)).toBeNull();
    const bar = screen.getByRole("progressbar", { name: "Progreso de la descarga" });
    expect(bar.hasAttribute("value")).toBe(false);
  });

  it("AMENAZAS U1: an invalid signature shows the message, never claims it was verified, installs nothing and does not restart", async () => {
    const { update, install } = fakeUpdate(downloadThenReject("The signature verification failed"));
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();
    const actualizar = await screen.findByRole("button", { name: "Actualizar" });
    const announced = recordAnnouncements();
    fireEvent.click(actualizar);

    expect(
      await screen.findByText(/La firma de la actualización no es válida: no se instaló\./),
    ).toBeDefined();
    expect(announced).toContain("Descarga completa. Verificando la firma…");
    expect(announced.filter((text) => text.includes("Firma verificada"))).toEqual([]);
    expect(screen.queryByText(/Instalando/)).toBeNull();
    expect(screen.queryByText(/signature verification failed/i)).toBeNull();
    expect(install).not.toHaveBeenCalled();
    expect(ipcCalls.filter((cmd) => cmd.startsWith("plugin:process"))).toEqual([]);
  });

  it("AMENAZAS U1: an update signed with another key is treated as an invalid signature", async () => {
    const { update, install } = fakeUpdate(
      downloadThenReject("The signature was created with a different key than the one provided"),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();
    const actualizar = await screen.findByRole("button", { name: "Actualizar" });
    const announced = recordAnnouncements();
    fireEvent.click(actualizar);

    expect(
      await screen.findByText(/La firma de la actualización no es válida: no se instaló\./),
    ).toBeDefined();
    expect(announced.filter((text) => text.includes("Firma verificada"))).toEqual([]);
    expect(install).not.toHaveBeenCalled();
  });

  it("RF-75 RF-36 (C-1): the vault is closed after the verified download and before installing", async () => {
    const order: string[] = [];
    const { update, install } = fakeUpdate(() => {
      order.push("download");
      return Promise.resolve();
    });
    install.mockImplementation(() => {
      order.push("install");
      return Promise.resolve();
    });
    let release: (() => void) | undefined;
    const beforeInstall = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          order.push("beforeInstall");
          release = resolve;
        }),
    );
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={beforeInstall} />);
    clickCheck();
    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    // It waits for the vault to close: nothing is installed meanwhile.
    expect(await screen.findByText("Firma verificada. Cerrando la bóveda antes de instalar…")).toBeDefined();
    expect(install).not.toHaveBeenCalled();

    release?.();
    expect(await screen.findByText(/Firma verificada\. Instalando/)).toBeDefined();
    expect(order).toEqual(["download", "beforeInstall", "install"]);
  });

  it("RF-75 RF-36 (C-1): if the vault cannot be closed, nothing is installed and the message says what to do", async () => {
    const { update, install } = fakeUpdate(() => Promise.resolve());
    const beforeInstall = vi.fn(() => Promise.reject(new Error("CoreBusy")));
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={beforeInstall} />);
    clickCheck();
    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    expect(await screen.findByText(/No se pudo cerrar la bóveda antes de actualizar; ciérrala con «Bloquear»/)).toBeDefined();
    expect(beforeInstall).toHaveBeenCalledTimes(1);
    expect(install).not.toHaveBeenCalled();
    expect(screen.queryByText(/CoreBusy/)).toBeNull();
    expect(screen.getByRole("button", { name: "Buscar actualizaciones" }).hasAttribute("disabled")).toBe(false);
  });

  it("RF-75 (C-1): a signature that fails verification never closes the vault", async () => {
    const { update } = fakeUpdate(downloadThenReject("The signature verification failed"));
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();
    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    expect(await screen.findByText(/La firma de la actualización no es válida/)).toBeDefined();
    expect(prepared).not.toHaveBeenCalled();
  });

  it("RF-75 (C-1): if installing fails after the vault was closed, it asks to close and reopen AlmarioKS", async () => {
    const { update, install } = fakeUpdate(() => Promise.resolve());
    install.mockImplementation(() => Promise.reject(new Error("installer failed")));
    check.mockResolvedValue(update);
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();
    fireEvent.click(await screen.findByRole("button", { name: "Actualizar" }));

    expect(await screen.findByText(/Cierra AlmarioKS y vuelve a abrirlo/)).toBeDefined();
    expect(prepared).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/installer failed/)).toBeNull();
  });

  it("without a connection it explains what happened and lets the user retry", async () => {
    check.mockRejectedValue(
      "error sending request for url (https://github.com/Kattonavi/almarioks/releases/latest/download/latest.json)",
    );
    render(<UpdatePanel beforeInstall={prepared} />);
    clickCheck();

    expect(await screen.findByText(/No hay conexión con el servidor de actualizaciones/)).toBeDefined();
    expect(screen.queryByText(/error sending request/)).toBeNull();
    const retry = screen.getByRole("button", { name: "Buscar actualizaciones" });
    expect(retry.hasAttribute("disabled")).toBe(false);
  });
});
