import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "./client";
import { createFakeBackend, type FakeEntry } from "./fake";
import { createTauriClient } from "./tauri";

afterEach(() => {
  clearMocks();
});

/** Fake data: it must never show up in a summary. */
const CANARY = "CANARIO-rf105";

const WIRE_SUMMARY = {
  id: "e1",
  title: "GitHub",
  username: "ana",
  has_password: true,
  url: "https://github.com",
  tags: ["trabajo"],
  modified: "2026-11-11T10:00:00Z",
};

const WIRE_DETAIL = {
  ...WIRE_SUMMARY,
  group_id: "g1",
  notes: "",
  protected_fields: ["Password"],
  custom_fields: [],
  attachments: [],
  created: "2026-11-11T09:00:00Z",
};

describe("the entry summary over the Tauri channel", () => {
  it("RF-105: `has_password` arrives as `hasPassword` in the list, the search and the detail", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "entry_list":
          return [WIRE_SUMMARY, { ...WIRE_SUMMARY, id: "e2", has_password: false }];
        case "search":
          return [{ ...WIRE_SUMMARY, has_password: false, group_path: ["Raíz", "Trabajo"] }];
        default:
          return WIRE_DETAIL;
      }
    });
    const client = createTauriClient();

    const list = await client.entryList("g1");
    const hits = await client.search("git");
    const detail = await client.entryGet("e1");

    expect(list).toEqual({
      ok: true,
      value: [
        {
          id: "e1",
          title: "GitHub",
          username: "ana",
          hasPassword: true,
          url: "https://github.com",
          tags: ["trabajo"],
          modified: "2026-11-11T10:00:00Z",
        },
        {
          id: "e2",
          title: "GitHub",
          username: "ana",
          hasPassword: false,
          url: "https://github.com",
          tags: ["trabajo"],
          modified: "2026-11-11T10:00:00Z",
        },
      ],
    });
    expect(hits.ok && hits.value).toEqual([
      {
        id: "e1",
        title: "GitHub",
        username: "ana",
        hasPassword: false,
        url: "https://github.com",
        tags: ["trabajo"],
        modified: "2026-11-11T10:00:00Z",
        groupPath: ["Raíz", "Trabajo"],
      },
    ]);
    expect(detail.ok && detail.value.hasPassword).toBe(true);
    expect(detail.ok && "has_password" in detail.value).toBe(false);
  });

  it("RF-105: a summary that does not say whether there is a password is not used", async () => {
    const withoutIt = Object.fromEntries(Object.entries(WIRE_SUMMARY).filter(([key]) => key !== "has_password"));
    const broken: unknown[] = [withoutIt, { ...WIRE_SUMMARY, has_password: null }, { ...WIRE_SUMMARY, has_password: 12 }];
    const client = createTauriClient();

    for (const summary of broken) {
      mockIPC((cmd) => (cmd === "search" ? [{ ...(summary as object), group_path: [] }] : [summary]));
      expect(await client.entryList("g1")).toEqual({ ok: false, error: { code: "RespuestaInvalida" } });
      expect(await client.search("git")).toEqual({ ok: false, error: { code: "RespuestaInvalida" } });
      clearMocks();
    }
  });

  it("RF-105: whatever else the core sent about the password does not reach the window", async () => {
    mockIPC(() => [{ ...WIRE_SUMMARY, password: CANARY, password_length: CANARY.length }]);

    const list = await createTauriClient().entryList("g1");

    expect(list.ok).toBe(true);
    expect(JSON.stringify(list)).not.toContain(CANARY);
    expect(list.ok && Object.keys(list.value[0] ?? {}).sort()).toEqual([
      "hasPassword",
      "id",
      "modified",
      "tags",
      "title",
      "url",
      "username",
    ]);
  });
});

describe("the fake core summarizes entries like the real one", () => {
  const ROOT = "group-root";
  const ENTRIES: readonly FakeEntry[] = [
    { id: "con", groupId: ROOT, title: "rf105 con", password: `${CANARY}-corta` },
    { id: "larga", groupId: ROOT, title: "rf105 larga", password: `${CANARY}-${"b".repeat(200)}` },
    { id: "vacia", groupId: ROOT, title: "rf105 vacía", password: "" },
    { id: "sin", groupId: ROOT, title: "rf105 sin" },
  ];

  function openBackend() {
    return createFakeBackend({
      status: {
        state: "open",
        path: "D:/b/casa.kdbx",
        name: "Casa",
        read_only: false,
        weak_kdf: false,
        save_pending: false,
        recovery: null,
      },
      entries: ENTRIES,
    });
  }

  it("RF-105: says yes or no in the list, the search and the detail", async () => {
    const client = createClient(openBackend().transport);

    const list = await client.entryList(ROOT);
    const hits = await client.search("rf105");

    const byId = (items: readonly { id: string; hasPassword: boolean }[]) =>
      Object.fromEntries(items.map((item) => [item.id, item.hasPassword]));
    const expected = { con: true, larga: true, vacia: false, sin: false };
    expect(list.ok && byId(list.value)).toEqual(expected);
    expect(hits.ok && byId(hits.value)).toEqual(expected);
    for (const [id, hasPassword] of Object.entries(expected)) {
      const detail = await client.entryGet(id);
      expect(detail.ok && detail.value.hasPassword, id).toBe(hasPassword);
    }
  });

  it("RF-105: the raw answers never carry the password nor anything taken from it", async () => {
    const backend = openBackend();

    const list = await backend.transport.invoke("entry_list", { args: { group_id: ROOT } });
    const hits = await backend.transport.invoke("search", { args: { query: "rf105" } });

    expect(JSON.stringify([list, hits])).not.toContain(CANARY);
    const rows = list as Record<string, unknown>[];
    const comparable = (id: string) =>
      Object.fromEntries(
        Object.entries(rows.find((row) => row.id === id) ?? {}).filter(([key]) => key !== "id" && key !== "title"),
      );
    // Two entries that differ only in their passwords give the same summary.
    expect(comparable("con")).toEqual(comparable("larga"));
    expect(Object.keys(rows[0] ?? {}).sort()).toEqual([
      "has_password",
      "id",
      "modified",
      "tags",
      "title",
      "url",
      "username",
    ]);
  });

  it("RF-105: an entry that gets a password says yes afterwards", async () => {
    const client = createClient(openBackend().transport);

    const edited = await client.entryUpdate("sin", { password: `${CANARY}-nueva` });
    const list = await client.entryList(ROOT);

    expect(edited.ok && edited.value.hasPassword).toBe(true);
    expect(list.ok && list.value.find((entry) => entry.id === "sin")?.hasPassword).toBe(true);
    expect(JSON.stringify([edited, list])).not.toContain(CANARY);
  });
});
