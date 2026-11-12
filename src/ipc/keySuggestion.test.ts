import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "./client";
import { ERROR_MESSAGES, errorMessage } from "./errors";
import { createFakeBackend, keyFileBeside } from "./fake";
import { createTauriClient } from "./tauri";

afterEach(() => {
  clearMocks();
});

const VAULT = "D:/bovedas/Familia.kdbx";
const KEY = "D:/bovedas/Familia.keyx";
const WIRE_SUGGESTION = { key_file_path: KEY, key_file: "Familia.keyx" };
const INVALID_ARGUMENT = { ok: false, error: { code: "ArgumentoInvalido" } };

describe("unlock_key_suggestion over the Tauri channel", () => {
  it("RF-110: sends the vault path in one snake_case `args` object and parses the suggestion", async () => {
    const seen: { cmd: string; payload: unknown }[] = [];
    mockIPC((cmd, payload) => {
      seen.push({ cmd, payload });
      return WIRE_SUGGESTION;
    });

    const result = await createTauriClient().unlockKeySuggestion(VAULT);

    expect(seen).toEqual([{ cmd: "unlock_key_suggestion", payload: { args: { path: VAULT } } }]);
    expect(result).toEqual({ ok: true, value: { keyFilePath: KEY, keyFile: "Familia.keyx" } });
  });

  it("RF-110: `null` means there is nothing to propose", async () => {
    mockIPC(() => null);

    expect(await createTauriClient().unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
  });

  it("RF-83: a suggestion that does not match the contract is not used", async () => {
    const client = createTauriClient();
    const broken: unknown[] = [
      { ...WIRE_SUGGESTION, key_file_path: "" },
      { ...WIRE_SUGGESTION, key_file: "" },
      { key_file_path: KEY },
      { keyFilePath: KEY, keyFile: "Familia.keyx" },
      KEY,
      true,
      undefined,
    ];
    for (const wire of broken) {
      mockIPC(() => wire);
      expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: false, error: { code: "RespuestaInvalida" } });
      clearMocks();
    }
  });
});

describe("the fake core proposes the key file like the real one", () => {
  it("RF-110: the key file beside a vault keeps its folder and takes `.keyx`", () => {
    expect(keyFileBeside(VAULT)).toBe(KEY);
    expect(keyFileBeside("C:\\bovedas\\Trabajo.KDBX")).toBe("C:\\bovedas\\Trabajo.keyx");
    expect(keyFileBeside("D:/bovedas/sin-extension")).toBe("D:/bovedas/sin-extension.keyx");
  });

  it("RF-110: a recent vault gets the key file beside it, and `null` when there is none", async () => {
    const recent = [
      { path: VAULT, exists: true },
      { path: "D:/bovedas/Sola.kdbx", exists: true },
    ];
    const backend = createFakeBackend({ recent, files: [keyFileBeside(VAULT)] });
    const client = createClient(backend.transport);

    expect(await client.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: KEY, keyFile: "Familia.keyx" },
    });
    expect(await client.unlockKeySuggestion("D:/bovedas/Sola.kdbx")).toEqual({ ok: true, value: null });
  });

  it("RF-110: the locked vault gets it, and so does the one just chosen in the open dialog", async () => {
    const locked = createClient(
      createFakeBackend({ status: { state: "locked", path: VAULT, recovery: null }, files: [KEY] }).transport,
    );
    expect(await locked.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: KEY, keyFile: "Familia.keyx" },
    });

    const picked = createClient(
      createFakeBackend({
        files: [KEY, "D:/bovedas/Otra.keyx"],
        picks: { pick_vault_to_open: [VAULT, null, "D:/bovedas/Otra.kdbx"] },
      }).transport,
    );
    // Not chosen yet.
    expect(await picked.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
    await picked.pickVaultToOpen();
    expect(await picked.unlockKeySuggestion(VAULT)).toMatchObject({ value: { keyFile: "Familia.keyx" } });
    // A cancelled dialog keeps the last answer; a new answer replaces it.
    await picked.pickVaultToOpen();
    expect(await picked.unlockKeySuggestion(VAULT)).toMatchObject({ value: { keyFile: "Familia.keyx" } });
    await picked.pickVaultToOpen();
    expect(await picked.unlockKeySuggestion("D:/bovedas/Otra.kdbx")).toMatchObject({ value: { keyFile: "Otra.keyx" } });
    expect(await picked.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
  });

  it("RF-110: a vault the core does not know answers `null` even when its key file is there", async () => {
    const backend = createFakeBackend({
      vaults: [{ path: VAULT, name: "Familia", password: "x" }],
      files: [KEY],
      picks: { pick_key_file: [VAULT], pick_vault_to_create: [VAULT], pick_new_key_file: [VAULT] },
    });
    const client = createClient(backend.transport);

    expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
    // The other dialogs are not "the vault to open", and a failed unlock does not make the vault known.
    await client.pickKeyFile();
    await client.pickVaultToCreate();
    await client.pickNewKeyFile();
    expect(await client.vaultOpen({ path: VAULT, password: "otra" })).toMatchObject({ ok: false });
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
  });

  it("RF-110: after a vault with a new key file is created and locked, the key file is proposed", async () => {
    const backend = createFakeBackend();
    const client = createClient(backend.transport);

    await client.vaultCreate({ path: VAULT, name: "Familia", password: "x", keyFile: { kind: "generate", path: KEY } });
    await client.vaultLock();

    const suggestion = await client.unlockKeySuggestion(VAULT);
    expect(suggestion).toEqual({ ok: true, value: { keyFilePath: KEY, keyFile: "Familia.keyx" } });
    const opened = await client.vaultOpen({ path: VAULT, password: "x", keyFilePath: KEY });
    expect(opened).toMatchObject({ ok: true, value: { state: "open" } });
  });

  it("RF-80: takes exactly one path that is not empty", async () => {
    const backend = createFakeBackend({ recent: [{ path: VAULT, exists: true }], files: [KEY] });
    const client = createClient(backend.transport);

    expect(await client.unlockKeySuggestion("")).toEqual(INVALID_ARGUMENT);
    for (const args of [{}, { path: 7 }, { path: null }, { path: VAULT, key_file_path: KEY }]) {
      await expect(backend.transport.invoke("unlock_key_suggestion", { args })).rejects.toMatchObject({
        code: "ArgumentoInvalido",
      });
    }
  });
});

describe("the message of a failed unlock", () => {
  it("RF-112: says the key does not match without saying whether the vault uses a key file", () => {
    const expected = "La contraseña o el archivo llave no coinciden. Si esta bóveda usa archivo llave, elígelo.";

    expect(ERROR_MESSAGES.LlaveIncorrecta).toBe(expected);
    expect(errorMessage({ code: "LlaveIncorrecta" })).toBe(expected);
  });
});
