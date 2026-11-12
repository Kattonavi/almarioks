import { describe, expect, it } from "vitest";

import { ERROR_CODES, ERROR_MESSAGES } from "./errors";

describe("error messages", () => {
  it("RF-83: every code has a Spanish message that does not show the code", () => {
    for (const code of ERROR_CODES) {
      const message = ERROR_MESSAGES[code];
      expect(message.length, code).toBeGreaterThan(20);
      expect(message, code).not.toContain(code);
      // docs/UX.md: no technical codes and no "inválido".
      expect(message.toLowerCase(), code).not.toMatch(/inv[aá]lid/);
      expect(message, code).toMatch(/\.$/);
    }
    expect(Object.keys(ERROR_MESSAGES).sort()).toEqual([...ERROR_CODES].sort());
  });

  it("RF-83: every message says what happened and what to do (two sentences or a colon)", () => {
    for (const code of ERROR_CODES) {
      const message = ERROR_MESSAGES[code];
      expect(/\. \S|: /.test(message), code).toBe(true);
    }
  });
});
