import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

// The table the core is tested with (`crates/almarioks-core/tests/rf_location.rs`): one rule, two implementations.
import fileNames from "../../crates/almarioks-core/tests/fixtures/location/file-names.json";
import { createClient } from "./client";
import { createFakeBackend, FAKE_DEFAULT_FOLDER, fileStem, keyFileName } from "./fake";
import { createTauriClient } from "./tauri";

afterEach(() => {
  clearMocks();
});

const WIRE_LOCATION = {
  vault_path: "C:/Docs/AlmarioKS/Casa.kdbx",
  vault_file: "Casa.kdbx",
  key_file_path: "C:/Docs/AlmarioKS/Casa.keyx",
  key_file: "Casa.keyx",
  in_default_folder: true,
  vault_exists: false,
  key_file_exists: true,
};

describe("vault_create_location over the Tauri channel", () => {
  it("RF-97: sends the name and `chosen_path` (also when it is null) in one snake_case `args` object", async () => {
    const seen: { cmd: string; payload: unknown }[] = [];
    mockIPC((cmd, payload) => {
      seen.push({ cmd, payload });
      return WIRE_LOCATION;
    });
    const client = createTauriClient();

    const proposed = await client.vaultCreateLocation("Casa", null);
    await client.vaultCreateLocation("Casa", "E:/otra/Casa.kdbx");

    expect(seen).toEqual([
      { cmd: "vault_create_location", payload: { args: { name: "Casa", chosen_path: null } } },
      { cmd: "vault_create_location", payload: { args: { name: "Casa", chosen_path: "E:/otra/Casa.kdbx" } } },
    ]);
    expect(proposed).toEqual({
      ok: true,
      value: {
        vaultPath: "C:/Docs/AlmarioKS/Casa.kdbx",
        vaultFile: "Casa.kdbx",
        keyFilePath: "C:/Docs/AlmarioKS/Casa.keyx",
        keyFile: "Casa.keyx",
        inDefaultFolder: true,
        vaultExists: false,
        keyFileExists: true,
      },
    });
  });

  it("RF-83: a location that does not match the contract is not used", async () => {
    const client = createTauriClient();
    const broken: unknown[] = [
      { ...WIRE_LOCATION, vault_exists: "no" },
      { ...WIRE_LOCATION, key_file_path: "" },
      { ...WIRE_LOCATION, in_default_folder: undefined },
      { vaultPath: "C:/Docs/AlmarioKS/Casa.kdbx" },
      null,
    ];
    for (const answer of broken) {
      mockIPC(() => answer);
      expect(await client.vaultCreateLocation("Casa", null)).toEqual({
        ok: false,
        error: { code: "RespuestaInvalida" },
      });
    }
  });

  it("RF-97: without a Documents folder the proposal fails with a code the window knows", async () => {
    mockIPC(() => {
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- Tauri rejects with the serialized object.
      throw { code: "ErrorInterno", message: "texto del núcleo" };
    });

    expect(await createTauriClient().vaultCreateLocation("Casa", null)).toEqual({
      ok: false,
      error: { code: "ErrorInterno" },
    });
  });
});

describe("the fake core proposes locations like the real one", () => {
  it("RF-97: the file name follows the same table of cases as the core", () => {
    expect(fileNames.length).toBeGreaterThanOrEqual(20);
    for (const { why, name, stem } of fileNames) {
      expect(fileStem(name), `${why}: ${JSON.stringify(name)}`).toBe(stem);
    }
  });

  it("RF-97: the file name is cut to 100 characters, never in the middle of one, and trimmed again", () => {
    expect(fileStem("a".repeat(101))).toBe("a".repeat(100));
    expect(fileStem("🔑".repeat(250))).toBe("🔑".repeat(100));
    expect(fileStem(`${"a".repeat(99)} final`)).toBe("a".repeat(99));
    expect(fileStem(`${"a".repeat(99)}.final`)).toBe("a".repeat(99));
    expect(fileStem(`NUL${" ".repeat(97)}x`)).toBe("NUL_");
    expect(fileStem(`${"?".repeat(300)}${"b".repeat(100)}`)).toBe("b".repeat(100));
  });

  it("RF-97: proposes Documents › AlmarioKS › <name>.kdbx and writes nothing", async () => {
    const fake = createFakeBackend();
    const client = createClient(fake.transport);

    expect(await client.vaultCreateLocation("Mi banco: 2026", null)).toEqual({
      ok: true,
      value: {
        vaultPath: `${FAKE_DEFAULT_FOLDER}/Mi banco 2026.kdbx`,
        vaultFile: "Mi banco 2026.kdbx",
        keyFilePath: `${FAKE_DEFAULT_FOLDER}/Mi banco 2026.keyx`,
        keyFile: "Mi banco 2026.keyx",
        inDefaultFolder: true,
        vaultExists: false,
        keyFileExists: false,
      },
    });
    const unnamed = await client.vaultCreateLocation("  ???  ", null);
    expect(unnamed.ok && unnamed.value.vaultFile).toBe("bóveda.kdbx");
    // Asking again gives the same answer: the first call created nothing.
    const again = await client.vaultCreateLocation("Mi banco: 2026", null);
    expect(again.ok && again.value.vaultExists).toBe(false);
  });

  it("RF-98: says that the vault or the key file already exist, and creating there answers ArchivoExiste", async () => {
    const taken = `${FAKE_DEFAULT_FOLDER}/Casa.kdbx`;
    const takenKey = `${FAKE_DEFAULT_FOLDER}/Trabajo.keyx`;
    const fake = createFakeBackend({ files: [taken, takenKey] });
    const client = createClient(fake.transport);

    const home = await client.vaultCreateLocation("Casa", null);
    expect(home.ok && [home.value.vaultExists, home.value.keyFileExists]).toEqual([true, false]);
    const work = await client.vaultCreateLocation("Trabajo", null);
    expect(work.ok && [work.value.vaultExists, work.value.keyFileExists]).toEqual([false, true]);

    expect(
      await client.vaultCreate({ path: taken, name: "Casa", password: "fixture-password", keyFile: { kind: "none" } }),
    ).toEqual({ ok: false, error: { code: "ArchivoExiste" } });
    // RF-99: a new key file never replaces the one beside the vault.
    expect(
      await client.vaultCreate({
        path: `${FAKE_DEFAULT_FOLDER}/Trabajo.kdbx`,
        name: "Trabajo",
        keyFile: { kind: "generate", path: takenKey },
      }),
    ).toEqual({ ok: false, error: { code: "ArchivoExiste" } });
    expect(await client.vaultStatus()).toEqual({ ok: true, value: { state: "closed" } });
  });

  it("RF-99: a vault created at the proposed location with its key file beside it is there afterwards", async () => {
    const fake = createFakeBackend();
    const client = createClient(fake.transport);
    const location = await client.vaultCreateLocation("Familia", null);
    if (!location.ok) throw new Error("the fake proposes a location");

    const created = await client.vaultCreate({
      path: location.value.vaultPath,
      name: "Familia",
      keyFile: { kind: "generate", path: location.value.keyFilePath },
    });
    expect(created.ok && created.value.state).toBe("open");

    const after = await client.vaultCreateLocation("Familia", null);
    expect(after.ok && [after.value.vaultExists, after.value.keyFileExists]).toEqual([true, true]);
  });

  it("RF-99: a chosen location is answered as it is, with the key file beside it", async () => {
    const client = createClient(createFakeBackend().transport);

    expect(await client.vaultCreateLocation("No se usa", "E:\\Bóvedas\\v1.2")).toEqual({
      ok: true,
      value: {
        vaultPath: "E:\\Bóvedas\\v1.2",
        vaultFile: "v1.2",
        keyFilePath: "E:\\Bóvedas\\v1.2.keyx",
        keyFile: "v1.2.keyx",
        inDefaultFolder: false,
        vaultExists: false,
        keyFileExists: false,
      },
    });
    const byHand = await client.vaultCreateLocation("x", `${FAKE_DEFAULT_FOLDER}/A mano.KDBX`);
    expect(byHand.ok && byHand.value).toMatchObject({ keyFile: "A mano.keyx", inDefaultFolder: true });

    expect(keyFileName("Cuentas.keyx")).toBe("Cuentas.keyx.keyx");
    expect(keyFileName("Cuentas.kdbx.bak")).toBe("Cuentas.kdbx.bak.keyx");
  });

  it("RF-80: `chosen_path` must be sent, as null or as a path with a file name", async () => {
    const fake = createFakeBackend();
    const malformed: Record<string, unknown>[] = [
      { name: "Casa" },
      { chosen_path: null },
      { name: "Casa", chosen_path: null, create: true },
      { name: "Casa", chosenPath: null },
      { name: 7, chosen_path: null },
      { name: "Casa", chosen_path: 7 },
      { name: "Casa", chosen_path: "" },
      { name: "Casa", chosen_path: "E:/carpeta/" },
      { name: "Casa", chosen_path: "E:/carpeta/.." },
    ];
    for (const args of malformed) {
      await expect(fake.transport.invoke("vault_create_location", { args })).rejects.toMatchObject({
        code: "ArgumentoInvalido",
      });
    }
  });
});
