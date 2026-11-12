import { describe, expect, it } from "vitest";

import { fileNameOf, initialState, reducer, type AppState } from "./state";

const OPEN: AppState = {
  kind: "open",
  vault: { path: "C:/v/cuentas.kdbx", fileName: "cuentas.kdbx" },
  name: "Cuentas",
  readOnly: false,
  recovery: null,
};

describe("app state machine", () => {
  it("RF-10: starts loading and follows the status of the core", () => {
    expect(initialState).toEqual({ kind: "loading" });
    expect(reducer(initialState, { type: "status", status: { state: "closed" } })).toEqual({
      kind: "closed",
      view: { kind: "welcome" },
    });
    expect(
      reducer(initialState, {
        type: "status",
        status: { state: "locked", path: "C:/v/cuentas.kdbx", recovery: null },
      }),
    ).toEqual({
      kind: "locked",
      vault: { path: "C:/v/cuentas.kdbx", fileName: "cuentas.kdbx" },
      recovery: null,
      error: null,
    });
  });

  it("RF-32: `vault-locked` drops the open vault, whatever the state", () => {
    const states: AppState[] = [
      OPEN,
      { kind: "locked", vault: OPEN.vault, recovery: null, error: { code: "LlaveIncorrecta" } },
      { kind: "deriving", vault: OPEN.vault, origin: "request", back: "locked", recovery: null },
    ];
    for (const state of states) {
      expect(reducer(state, { type: "vault-locked" })).toEqual({ kind: "loading" });
    }
  });

  it("RF-11: an unlock that fails goes back to where it started, with the error and the vault still locked", () => {
    const locked: AppState = { kind: "locked", vault: OPEN.vault, recovery: { kind: "lost" }, error: null };
    const deriving = reducer(locked, { type: "unlock-started" });
    expect(deriving).toEqual({
      kind: "deriving",
      vault: OPEN.vault,
      origin: "request",
      back: "locked",
      recovery: { kind: "lost" },
    });
    expect(reducer(deriving, { type: "unlock-failed", error: { code: "LlaveIncorrecta" } })).toEqual({
      ...locked,
      error: { code: "LlaveIncorrecta" },
    });

    const chosen = reducer(
      { kind: "closed", view: { kind: "welcome" } },
      { type: "vault-chosen", path: "C:/v/otra.kdbx" },
    );
    const failed = reducer(reducer(chosen, { type: "unlock-started" }), {
      type: "unlock-failed",
      error: { code: "ArchivoDanado" },
    });
    expect(failed).toEqual({
      kind: "closed",
      view: {
        kind: "unlock",
        vault: { path: "C:/v/otra.kdbx", fileName: "otra.kdbx" },
        error: { code: "ArchivoDanado" },
      },
    });
  });

  it("RF-18: a status that says `deriving` shows the derivation", () => {
    expect(
      reducer(initialState, { type: "status", status: { state: "deriving", path: "C:/v/cuentas.kdbx" } }),
    ).toMatchObject({ kind: "deriving", origin: "status" });
  });

  it("RF-28: a lock never turns into an unlock screen for another state by a stale action", () => {
    expect(reducer(OPEN, { type: "unlock-failed", error: { code: "LlaveIncorrecta" } })).toBe(OPEN);
    expect(reducer(OPEN, { type: "vault-chosen", path: "C:/x.kdbx" })).toBe(OPEN);
  });

  it("RF-19: names a vault by its file name on Windows and POSIX paths", () => {
    expect(fileNameOf("C:\\Users\\x\\cuentas.kdbx")).toBe("cuentas.kdbx");
    expect(fileNameOf("/home/x/cuentas.kdbx")).toBe("cuentas.kdbx");
  });
});
