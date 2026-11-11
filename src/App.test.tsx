import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "./App";

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("App", () => {
  it("shows the app name and the version reported by Tauri", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      if (cmd === "plugin:app|version") return "9.8.7";
      throw new Error(`unexpected command ${cmd}`);
    });

    render(<App />);

    expect(screen.getByRole("heading", { level: 1, name: "AlmarioKS" })).toBeDefined();
    expect(await screen.findByText("Versión 9.8.7")).toBeDefined();
    expect(calls).toEqual(["plugin:app|version"]);
  });

  it("says so when the version cannot be read", async () => {
    mockIPC(() => {
      throw new Error("denied");
    });

    render(<App />);

    expect(await screen.findByText("No se pudo leer la versión.")).toBeDefined();
  });
});
