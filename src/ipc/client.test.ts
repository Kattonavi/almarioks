import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createTauriClient } from "./tauri";
import { z } from "./zod";

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

const OPEN_STATUS = {
  state: "open",
  path: "C:/vaults/cuentas.kdbx",
  name: "Cuentas",
  read_only: false,
  weak_kdf: false,
  save_pending: false,
  recovery: null,
};

describe("IPC client over the Tauri channel", () => {
  it("RF-10: sends vault_open as one snake_case `args` object and parses the status into domain types", async () => {
    const seen: { cmd: string; payload: unknown }[] = [];
    mockIPC((cmd, payload) => {
      seen.push({ cmd, payload });
      return OPEN_STATUS;
    });

    const result = await createTauriClient().vaultOpen({
      path: "C:/vaults/cuentas.kdbx",
      password: "fixture-password",
      keyFilePath: "C:/keys/cuentas.keyx",
    });

    expect(seen).toEqual([
      {
        cmd: "vault_open",
        payload: {
          args: {
            path: "C:/vaults/cuentas.kdbx",
            password: "fixture-password",
            key_file_path: "C:/keys/cuentas.keyx",
          },
        },
      },
    ]);
    expect(result).toEqual({
      ok: true,
      value: {
        state: "open",
        path: "C:/vaults/cuentas.kdbx",
        name: "Cuentas",
        readOnly: false,
        weakKdf: false,
        savePending: false,
        recovery: null,
      },
    });
  });

  it("RF-10: leaves out the parts of the key that were not given", async () => {
    const payloads: unknown[] = [];
    mockIPC((_cmd, payload) => {
      payloads.push(payload);
      return OPEN_STATUS;
    });

    await createTauriClient().vaultOpen({ path: "C:/v.kdbx", keyFilePath: "C:/k.keyx" });
    await createTauriClient().vaultOpen({ path: "C:/v.kdbx", password: "x" });

    expect(payloads).toEqual([
      { args: { path: "C:/v.kdbx", key_file_path: "C:/k.keyx" } },
      { args: { path: "C:/v.kdbx", password: "x" } },
    ]);
  });

  it("RF-83: a response that does not match the contract is not used", async () => {
    mockIPC(() => ({ state: "open", path: "C:/v.kdbx" }));

    expect(await createTauriClient().vaultStatus()).toEqual({
      ok: false,
      error: { code: "RespuestaInvalida" },
    });
  });

  it("RF-83: a status for a locked vault without a path is not used", async () => {
    mockIPC(() => ({ state: "locked", recovery: null }));

    expect(await createTauriClient().vaultStatus()).toEqual({
      ok: false,
      error: { code: "RespuestaInvalida" },
    });
  });

  it("RF-11: a `{ code, message }` rejection keeps the code and drops the core's text", async () => {
    mockIPC(() => {
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- Tauri rejects with the serialized object.
      throw { code: "LlaveIncorrecta", message: "texto del núcleo" };
    });

    expect(await createTauriClient().vaultOpen({ path: "C:/v.kdbx", password: "x" })).toEqual({
      ok: false,
      error: { code: "LlaveIncorrecta" },
    });
  });

  it("RF-83: a string rejection, an unknown code or a window-only code become `Inesperado`", async () => {
    const rejections: unknown[] = [
      "command vault_status not found",
      { code: "AlgoNuevo", message: "?" },
      { code: "RespuestaInvalida", message: "?" },
      new Error("boom"),
    ];
    const client = createTauriClient();
    for (const rejection of rejections) {
      mockIPC(() => {
        throw rejection;
      });
      expect(await client.vaultStatus()).toEqual({ ok: false, error: { code: "Inesperado" } });
    }
  });

  it("RF-19: commands without input send no `args`; `null` results are accepted as nothing", async () => {
    const seen: unknown[] = [];
    mockIPC((cmd, payload) => {
      seen.push([cmd, payload]);
      return cmd === "pick_vault_to_open" ? null : [{ path: "C:/a.kdbx", exists: false }];
    });
    const client = createTauriClient();

    expect(await client.pickVaultToOpen()).toEqual({ ok: true, value: null });
    expect(await client.recentList()).toEqual({ ok: true, value: [{ path: "C:/a.kdbx", exists: false }] });
    expect(seen).toEqual([
      ["pick_vault_to_open", {}],
      ["recent_list", {}],
    ]);
  });

  it("RF-43: a protected custom field never brings a value to the window, even if one is sent", async () => {
    mockIPC(() => ({
      id: "e1",
      title: "GitHub",
      username: null,
      url: "https://github.com",
      tags: [],
      modified: "2026-11-11T10:00:00Z",
      group_id: "g1",
      notes: null,
      protected_fields: ["Password"],
      custom_fields: [{ name: "PIN", protected: true, value: "CANARIO" }],
      attachments: [],
      created: "2026-11-11T09:00:00Z",
    }));

    const result = await createTauriClient().entryGet("e1");

    expect(result.ok && result.value.customFields).toEqual([{ name: "PIN", protected: true, value: null }]);
  });

  it("RF-32: the `vault-locked` event reaches the handler and stops after unlisten", async () => {
    mockIPC(() => null, { shouldMockEvents: true });
    const handler = vi.fn();

    const unlisten = await createTauriClient().onVaultLocked(handler);
    await emit("vault-locked");
    unlisten();
    await emit("vault-locked");

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("RF-23: a `vault-save-state` payload outside the contract is ignored", async () => {
    mockIPC(() => null, { shouldMockEvents: true });
    const handler = vi.fn();

    await createTauriClient().onSaveState(handler);
    await emit("vault-save-state", "failed");
    await emit("vault-save-state", "<img src=x onerror=alert(1)>");

    expect(handler.mock.calls).toEqual([["failed"]]);
  });
});

describe("validation under the app CSP (script-src 'self', no unsafe-eval)", () => {
  it("parses objects without the Function constructor", () => {
    const constructor = vi.spyOn(globalThis, "Function");
    const schema = z.object({ a: z.string(), b: z.object({ c: z.int() }) });

    expect(schema.safeParse({ a: "x", b: { c: 1 } }).success).toBe(true);
    expect(z.config().jitless).toBe(true);
    expect(constructor).not.toHaveBeenCalled();
  });
});
