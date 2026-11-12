import { describe, expect, it } from "vitest";

import { formatShortDay, shortSite } from "./format";

describe("what a row of the entry list shows", () => {
  // Noon, so the day is the same in every time zone the tests may run in.
  const today = new Date("2026-11-12T12:00:00Z");

  it("RF-42: the day is short and carries the year only when it is not the current one", () => {
    expect(formatShortDay("2026-11-08T12:00:00Z", today)).toBe("8 nov");
    expect(formatShortDay("2026-09-03T12:00:00Z", today)).toBe("3 sept");
    expect(formatShortDay("2025-11-08T12:00:00Z", today)).toBe("8 nov 2025");
    expect(formatShortDay("2027-01-02T12:00:00Z", today)).toBe("2 ene 2027");
  });

  it("RF-42: a date that cannot be read is shown as it came", () => {
    expect(formatShortDay("no es una fecha", today)).toBe("no es una fecha");
  });

  it("RF-42: the site drops `https://` and nothing else", () => {
    expect(shortSite("https://github.com/login")).toBe("github.com/login");
    expect(shortSite("HTTPS://github.com")).toBe("github.com");
    expect(shortSite("http://intranet.example")).toBe("http://intranet.example");
    expect(shortSite("vpn.example")).toBe("vpn.example");
    expect(shortSite("javascript:alert(1)")).toBe("javascript:alert(1)");
    expect(shortSite("ver https://otro.example")).toBe("ver https://otro.example");
    expect(shortSite("")).toBe("");
  });
});
