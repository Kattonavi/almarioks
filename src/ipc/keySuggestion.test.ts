import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "./client";
import { ERROR_MESSAGES, errorMessage } from "./errors";
import { createFakeBackend, isRememberableKeyFile, keyFileBeside } from "./fake";
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

// ---------- RF-111: the key file a recent vault was last opened with ----------

/** A key file that is not beside the vault. */
const ELSEWHERE = "E:/llaves/mi-llave.key";
const WIRE_REMEMBERED = { key_file_path: ELSEWHERE, key_file: "mi-llave.key", source: "remembered" };
const RECENT = [{ path: VAULT, exists: true }];
const WITH_BOTH = { path: VAULT, name: "Familia", password: "x", keyFilePath: ELSEWHERE };

describe("unlock_key_suggestion with a remembered key file over the Tauri channel", () => {
  it("RF-111: the remembered key file says where it comes from; the one beside the vault stays as it was", async () => {
    const client = createTauriClient();

    mockIPC(() => WIRE_REMEMBERED);
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: ELSEWHERE, keyFile: "mi-llave.key", source: "remembered" },
    });
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: { path: ELSEWHERE, name: "mi-llave.key", source: "remembered" }, rememberedMissing: null },
    });
    clearMocks();

    const seen: { cmd: string; payload: unknown }[] = [];
    mockIPC((cmd, payload) => {
      seen.push({ cmd, payload });
      return WIRE_SUGGESTION;
    });
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: { path: KEY, name: "Familia.keyx", source: "beside" }, rememberedMissing: null },
    });
    // The same command and the same arguments as `unlockKeySuggestion`.
    expect(seen).toEqual([{ cmd: "unlock_key_suggestion", payload: { args: { path: VAULT } } }]);
  });

  it("RF-111: a remembered key file that is missing is said with its path, with or without one beside the vault", async () => {
    const client = createTauriClient();

    mockIPC(() => ({ ...WIRE_SUGGESTION, remembered_missing: ELSEWHERE }));
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: KEY, keyFile: "Familia.keyx", rememberedMissing: ELSEWHERE },
    });
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: { path: KEY, name: "Familia.keyx", source: "beside" }, rememberedMissing: ELSEWHERE },
    });
    clearMocks();

    // Nothing to propose: whoever only reads `keyFilePath` and `keyFile` gets `null`, as with no key file at all.
    mockIPC(() => ({ remembered_missing: ELSEWHERE }));
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: null, rememberedMissing: ELSEWHERE },
    });
    clearMocks();

    mockIPC(() => null);
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: null, rememberedMissing: null },
    });
  });

  it("RF-83: an answer that mixes the shapes is not used, by either method", async () => {
    const client = createTauriClient();
    const broken: unknown[] = [
      {},
      { source: "remembered" },
      { ...WIRE_REMEMBERED, remembered_missing: ELSEWHERE },
      { ...WIRE_SUGGESTION, source: "beside" },
      { ...WIRE_SUGGESTION, source: "" },
      { ...WIRE_SUGGESTION, remembered_missing: "" },
      { ...WIRE_SUGGESTION, remembered_missing: 7 },
      { key_file_path: KEY, remembered_missing: ELSEWHERE },
      { key_file: "Familia.keyx", remembered_missing: ELSEWHERE },
      { remembered_missing: ELSEWHERE, source: "remembered" },
      { rememberedMissing: ELSEWHERE },
    ];
    for (const wire of broken) {
      mockIPC(() => wire);
      expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: false, error: { code: "RespuestaInvalida" } });
      expect(await client.unlockKeyProposal(VAULT)).toEqual({ ok: false, error: { code: "RespuestaInvalida" } });
      clearMocks();
    }
  });
});

describe("the fake core remembers the key file like the real one", () => {
  it("RF-111 (C-2): only a full path on a lettered drive may be remembered, as in the core", () => {
    const accepted = [
      "C:\\llaves\\cuentas.keyx",
      "d:/llaves/cuentas.keyx",
      "E:\\cuentas.keyx",
      "C:\\carpeta con espacios\\ñandú.key",
      `C:\\${"ñ".repeat(1021)}`,
    ];
    const refused = [
      "",
      "C:",
      "C:\\",
      "C:\\llaves\\",
      "C:\\llaves\\..",
      "\\\\servidor\\recurso\\cuentas.keyx",
      "//servidor/recurso/cuentas.keyx",
      "\\\\?\\C:\\llaves\\cuentas.keyx",
      "\\\\?\\UNC\\servidor\\recurso\\cuentas.keyx",
      "\\\\.\\C:\\llaves\\cuentas.keyx",
      "cuentas.keyx",
      "llaves\\cuentas.keyx",
      "..\\cuentas.keyx",
      "\\llaves\\cuentas.keyx",
      "C:cuentas.keyx",
      "C:\\llaves\\cuentas.keyx:flujo",
      "C:\\D:\\cuentas.keyx",
      "1:\\cuentas.keyx",
      "é:\\cuentas.keyx",
      "CC:\\cuentas.keyx",
      `C:\\${"ñ".repeat(1022)}`,
    ];
    for (const path of accepted) expect(isRememberableKeyFile(path), path).toBe(true);
    for (const path of refused) expect(isRememberableKeyFile(path), path).toBe(false);
  });

  it("RF-111: chosen once in the dialog and opened with the password, the key file comes already chosen", async () => {
    const backend = createFakeBackend({
      vaults: [WITH_BOTH],
      recent: RECENT,
      files: [ELSEWHERE, KEY],
      picks: { pick_key_file: [ELSEWHERE] },
    });
    const client = createClient(backend.transport);

    // Nothing remembered yet: the one beside the vault.
    expect(await client.unlockKeyProposal(VAULT)).toMatchObject({ value: { keyFile: { path: KEY, source: "beside" } } });

    expect(await client.pickKeyFile()).toEqual({ ok: true, value: ELSEWHERE });
    expect(await client.vaultOpen({ path: VAULT, password: "x", keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await client.vaultLock();

    // The remembered one wins over the one beside the vault, in both shapes.
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: ELSEWHERE, keyFile: "mi-llave.key", source: "remembered" },
    });
    expect(await client.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: { path: ELSEWHERE, name: "mi-llave.key", source: "remembered" }, rememberedMissing: null },
    });
    // The one already remembered needs no dialog the next time.
    expect(await client.vaultOpen({ path: VAULT, password: "x", keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await client.vaultClose();
    expect(await client.unlockKeyProposal(VAULT)).toMatchObject({ value: { keyFile: { source: "remembered" } } });
  });

  it("RF-111 (C-3, C-10): nothing is remembered without the dialog, without a password or for the one beside", async () => {
    // A path the window sends that no dialog answered.
    const unpicked = createClient(
      createFakeBackend({ vaults: [WITH_BOTH], recent: RECENT, files: [ELSEWHERE] }).transport,
    );
    expect(await unpicked.vaultOpen({ path: VAULT, password: "x", keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await unpicked.vaultClose();
    expect(await unpicked.unlockKeyProposal(VAULT)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: null } });

    // The other dialogs are not "the key file to open with".
    const others = createClient(
      createFakeBackend({
        vaults: [WITH_BOTH],
        recent: RECENT,
        files: [ELSEWHERE],
        picks: { pick_new_key_file: [ELSEWHERE], pick_vault_to_open: [ELSEWHERE], pick_vault_to_create: [ELSEWHERE] },
      }).transport,
    );
    await others.pickNewKeyFile();
    await others.pickVaultToOpen();
    await others.pickVaultToCreate();
    expect(await others.vaultOpen({ path: VAULT, password: "x", keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await others.vaultClose();
    expect(await others.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });

    // A vault that opens with its key file alone: also what was remembered for it goes.
    const keyOnly = createClient(
      createFakeBackend({
        vaults: [{ path: VAULT, name: "Familia", keyFilePath: ELSEWHERE }],
        recent: RECENT,
        keyFiles: { [VAULT]: ELSEWHERE },
        files: [ELSEWHERE],
        picks: { pick_key_file: [ELSEWHERE] },
      }).transport,
    );
    expect(await keyOnly.unlockKeyProposal(VAULT)).toMatchObject({ value: { keyFile: { source: "remembered" } } });
    await keyOnly.pickKeyFile();
    expect(await keyOnly.vaultOpen({ path: VAULT, keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await keyOnly.vaultClose();
    expect(await keyOnly.unlockKeyProposal(VAULT)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: null } });

    // The one beside the vault, even chosen in the dialog: it is found on its own, and the remembered one goes.
    const beside = createClient(
      createFakeBackend({
        vaults: [{ ...WITH_BOTH, keyFilePath: KEY }],
        recent: RECENT,
        keyFiles: { [VAULT]: ELSEWHERE },
        files: [ELSEWHERE, KEY],
        picks: { pick_key_file: [KEY] },
      }).transport,
    );
    await beside.pickKeyFile();
    expect(await beside.vaultOpen({ path: VAULT, password: "x", keyFilePath: KEY })).toMatchObject({ ok: true });
    await beside.vaultClose();
    expect(await beside.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: { keyFilePath: KEY, keyFile: "Familia.keyx" } });
  });

  it("RF-111 (C-1): an unlock that fails, and creating a vault, leave what is remembered as it was", async () => {
    const other = "E:/llaves/otra.key";
    const backend = createFakeBackend({
      vaults: [WITH_BOTH],
      recent: RECENT,
      keyFiles: { [VAULT]: ELSEWHERE },
      files: [ELSEWHERE, other],
      picks: { pick_key_file: [other] },
    });
    const client = createClient(backend.transport);
    const remembered = { keyFilePath: ELSEWHERE, keyFile: "mi-llave.key", source: "remembered" };

    await client.pickKeyFile();
    expect(await client.vaultOpen({ path: VAULT, password: "x", keyFilePath: other })).toMatchObject({ ok: false });
    expect(await client.vaultOpen({ path: VAULT, password: "otra", keyFilePath: ELSEWHERE })).toMatchObject({
      ok: false,
    });
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: remembered });

    // A new vault with a key file elsewhere is remembered as a vault, without a key file.
    const created = "D:/bovedas/Nueva.kdbx";
    await client.vaultCreate({ path: created, name: "Nueva", password: "x", keyFile: { kind: "existing", path: other } });
    await client.vaultClose();
    expect(await client.unlockKeySuggestion(created)).toEqual({ ok: true, value: null });
    expect(await client.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: remembered });
  });

  it("RF-111 (C-6): a remembered key file that is missing is said, falls back to the one beside, and is not forgotten", async () => {
    const seed = { vaults: [WITH_BOTH], recent: RECENT, keyFiles: { [VAULT]: ELSEWHERE } };

    const alone = createClient(createFakeBackend(seed).transport);
    expect(await alone.unlockKeyProposal(VAULT)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: ELSEWHERE } });
    expect(await alone.unlockKeySuggestion(VAULT)).toEqual({ ok: true, value: null });
    // Asking again gives the same answer: it was not forgotten for being missing.
    expect(await alone.unlockKeyProposal(VAULT)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: ELSEWHERE } });

    const withBeside = createClient(createFakeBackend({ ...seed, files: [KEY] }).transport);
    expect(await withBeside.unlockKeyProposal(VAULT)).toEqual({
      ok: true,
      value: { keyFile: { path: KEY, name: "Familia.keyx", source: "beside" }, rememberedMissing: ELSEWHERE },
    });
    expect(await withBeside.unlockKeySuggestion(VAULT)).toEqual({
      ok: true,
      value: { keyFilePath: KEY, keyFile: "Familia.keyx", rememberedMissing: ELSEWHERE },
    });
  });

  it("RF-111 (C-7): forgotten with the vault, when it opens without a key file, and replaced by another one chosen", async () => {
    const moved = "F:/llaves/movida.key";
    const seed = { recent: RECENT, keyFiles: { [VAULT]: ELSEWHERE }, files: [ELSEWHERE, moved] };
    const nothing = { ok: true, value: { keyFile: null, rememberedMissing: null } };

    // (a) Removed from the recent list; the vault is still the locked one, so the core still answers about it.
    const removed = createClient(
      createFakeBackend({ ...seed, status: { state: "locked", path: VAULT, recovery: null } }).transport,
    );
    await removed.recentRemove(VAULT);
    expect(await removed.unlockKeyProposal(VAULT)).toEqual(nothing);

    // (a) Pushed out by five newer vaults, and back in the list later.
    const pushed = createClient(createFakeBackend({ ...seed, vaults: [WITH_BOTH] }).transport);
    for (const n of [1, 2, 3, 4, 5]) {
      const path = `D:/bovedas/nueva-${String(n)}.kdbx`;
      await pushed.vaultCreate({ path, name: "Nueva", password: "x", keyFile: { kind: "none" } });
    }
    await pushed.vaultClose();
    expect(await pushed.recentList()).toMatchObject({ value: { length: 5 } });
    expect(await pushed.vaultOpen({ path: VAULT, password: "x", keyFilePath: ELSEWHERE })).toMatchObject({ ok: true });
    await pushed.vaultClose();
    expect(await pushed.unlockKeyProposal(VAULT)).toEqual(nothing);

    // (b) The vault at that path now opens with the password alone.
    const plain = createClient(
      createFakeBackend({ ...seed, vaults: [{ path: VAULT, name: "Familia", password: "x" }] }).transport,
    );
    expect(await plain.vaultOpen({ path: VAULT, password: "x" })).toMatchObject({ ok: true });
    await plain.vaultClose();
    expect(await plain.unlockKeyProposal(VAULT)).toEqual(nothing);

    // (c) Another key file, chosen in the dialog.
    const replaced = createClient(
      createFakeBackend({
        ...seed,
        vaults: [{ ...WITH_BOTH, keyFilePath: moved }],
        picks: { pick_key_file: [moved] },
      }).transport,
    );
    await replaced.pickKeyFile();
    expect(await replaced.vaultOpen({ path: VAULT, password: "x", keyFilePath: moved })).toMatchObject({ ok: true });
    await replaced.vaultClose();
    expect(await replaced.unlockKeyProposal(VAULT)).toMatchObject({
      value: { keyFile: { path: moved, name: "movida.key", source: "remembered" }, rememberedMissing: null },
    });
  });

  it("RF-111 (C-4): a seed the core would drop when reading its settings is dropped", async () => {
    const unc = "\\\\servidor\\recurso\\llave.key";
    const notRecent = "D:/bovedas/Otra.kdbx";
    const backend = createFakeBackend({
      recent: RECENT,
      keyFiles: { [VAULT]: unc, [notRecent]: ELSEWHERE },
      files: [unc, ELSEWHERE],
      picks: { pick_vault_to_open: [notRecent] },
    });
    const client = createClient(backend.transport);

    // The same answer as with nothing remembered: not even "the remembered one is missing".
    expect(await client.unlockKeyProposal(VAULT)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: null } });
    await client.pickVaultToOpen();
    expect(await client.unlockKeyProposal(notRecent)).toEqual({ ok: true, value: { keyFile: null, rememberedMissing: null } });
  });
});

describe("the message of a failed unlock", () => {
  it("RF-112: says the key does not match without saying whether the vault uses a key file", () => {
    const expected = "La contraseña o el archivo llave no coinciden. Si esta bóveda usa archivo llave, elígelo.";

    expect(ERROR_MESSAGES.LlaveIncorrecta).toBe(expected);
    expect(errorMessage({ code: "LlaveIncorrecta" })).toBe(expected);
  });
});
