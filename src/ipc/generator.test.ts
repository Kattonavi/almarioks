import { describe, expect, it } from "vitest";

import { createClient } from "./client";
import { ERROR_MESSAGES, errorMessage } from "./errors";
import { createFakeBackend } from "./fake";

const ALL_TYPES = { lower: true, upper: true, digits: true, symbols: true, avoidLookalikes: false };

describe("generator through the typed client (second round of the owner, 2026-11-12)", () => {
  it("RF-65: the fake core generates 8 and 500 characters and the client hands over every one", async () => {
    const client = createClient(createFakeBackend().transport);
    for (const length of [8, 20, 500]) {
      const generated = await client.generatePassword({ ...ALL_TYPES, length });
      expect(generated.ok, String(length)).toBe(true);
      if (generated.ok) expect(Array.from(generated.value.password).length).toBe(length);
    }
  });

  it("RF-68: 7 and 501 generate nothing and answer ReglasImposibles, whose text names 8 to 500", async () => {
    const client = createClient(createFakeBackend().transport);
    for (const length of [7, 501]) {
      const generated = await client.generatePassword({ ...ALL_TYPES, length });
      expect(generated, String(length)).toEqual({ ok: false, error: { code: "ReglasImposibles" } });
    }
    expect(ERROR_MESSAGES.ReglasImposibles).toContain("entre 8 y 500");
  });

  it("RF-109: a core without randomness answers GeneradorNoDisponible, with the text the owner asked for", async () => {
    const backend = createFakeBackend();
    const client = createClient(backend.transport);
    backend.rejectOnce("generate_password", { code: "GeneradorNoDisponible", message: "core text" });

    const generated = await client.generatePassword({ ...ALL_TYPES, length: 20 });

    expect(generated).toEqual({ ok: false, error: { code: "GeneradorNoDisponible" } });
    if (!generated.ok) {
      expect(errorMessage(generated.error)).toBe("No se pudo generar la contraseña. Inténtalo otra vez.");
    }
    // It is not the text of a failed save, which is what the window showed before.
    expect(ERROR_MESSAGES.GeneradorNoDisponible).not.toBe(ERROR_MESSAGES.GuardadoFallido);
  });
});
