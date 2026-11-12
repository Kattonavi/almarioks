import { describe, expect, it } from "vitest";

import { formatDate, formatShortDay, shortSite } from "./format";

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

describe("what the window of an entry says of its dates", () => {
  /** The locale separates "a. m." with spaces that are not the plain one: the words are what is compared. */
  const words = (text: string) => text.replace(/\s/gu, " ");
  /** A moment given in the time of the computer the tests run on, as the core sends it: an instant in UTC. */
  const local = (year: number, month: number, day: number, hour: number, minute: number) =>
    new Date(year, month - 1, day, hour, minute).toISOString();

  it("RF-107: the day and the hour of this computer, to the minute", () => {
    expect(words(formatDate(local(2026, 3, 2, 9, 14)))).toBe("2 mar 2026, 9:14 a. m.");
    expect(words(formatDate(local(2026, 11, 8, 16, 32)))).toBe("8 nov 2026, 4:32 p. m.");
    expect(words(formatDate(local(2026, 9, 3, 0, 5)))).toBe("3 sept 2026, 12:05 a. m.");
  });

  it("RF-107: a date that cannot be read is shown as it came", () => {
    expect(formatDate("no es una fecha")).toBe("no es una fecha");
  });
});
