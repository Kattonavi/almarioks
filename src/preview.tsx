/*
 * Development-only preview (`/preview.html` under `pnpm dev`): the real screens on the fake core, with sample data,
 * to judge the design in a browser. It is not an entry of the production build.
 *
 *   ?tema=oscuro                 forces the dark theme (the same file production loads by media query)
 *   ?estado=cerrado              the welcome screen instead of the open vault
 *   ?demo=copiado|dialogo|lectura|nueva|ninguna   what the open vault shows (default: copiado)
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@fontsource-variable/overpass/index.css";
import "@fontsource-variable/overpass-mono/index.css";
import { App } from "./App";
import { createClient } from "./ipc/client";
import { createFakeBackend, type FakeEntry, type FakeGroup } from "./ipc/fake";
import "./styles.css";
import { VaultRoot } from "./vault/VaultRoot";

const params = new URLSearchParams(window.location.search);
const demo = params.get("demo") ?? "copiado";
const closed = params.get("estado") === "cerrado";

if (params.get("tema") === "oscuro") await import("./styles/theme-dark.css");

const VAULT_PATH = "D:/bovedas/cuentas.kdbx";

const GROUPS: FakeGroup = {
  id: "g-root",
  name: "Cuentas",
  kind: "root",
  children: [
    { id: "g-personal", name: "Personal" },
    { id: "g-work", name: "Trabajo", children: [{ id: "g-clients", name: "Clientes" }] },
    { id: "g-banks", name: "Bancos" },
    { id: "g-shopping", name: "Compras" },
    { id: "g-bin", name: "Papelera de reciclaje", kind: "recycle_bin" },
  ],
};

function entry(id: string, groupId: string, title: string, username: string, modified: string, url = ""): FakeEntry {
  return { id, groupId, title, username, url, modified, created: "2026-03-02T14:00:00Z", password: "demo-Clave-0001" };
}

const ENTRIES: readonly FakeEntry[] = [
  entry("e-mail", "g-work", "Correo de la oficina", "ana@example.com", "2026-11-10T15:20:00Z"),
  entry("e-figma", "g-work", "Figma", "ana@example.com", "2026-10-12T09:00:00Z", "https://figma.example"),
  {
    ...entry("e-github", "g-work", "GitHub", "ana@example.com", "2026-11-08T18:45:00Z", "https://github.com"),
    notes: "Cuenta de la organización. Renovar el token de acceso en enero.",
    tags: ["código", "trabajo"],
    customFields: [{ name: "PIN", protected: true, value: "4821" }],
  },
  entry("e-jira", "g-work", "Jira", "ana.g", "2026-09-03T11:10:00Z"),
  entry("e-payroll", "g-work", "Nómina", "1032456789", "2026-08-30T08:00:00Z"),
  entry("e-railway", "g-work", "Railway", "ana@example.com", "2026-08-28T20:30:00Z", "https://railway.example"),
  entry("e-slack", "g-work", "Slack", "ana@example.com", "2026-08-15T13:00:00Z"),
  entry("e-vpn", "g-work", "VPN de la oficina", "agomez", "2026-07-02T07:45:00Z"),
  entry("e-client", "g-clients", "Portal del cliente", "compras", "2026-06-11T10:00:00Z"),
  entry("e-music", "g-personal", "Música", "ana", "2026-11-02T09:30:00Z", "https://musica.example"),
  entry("e-photos", "g-personal", "Fotos", "ana@example.com", "2026-05-20T16:00:00Z"),
  entry("e-bank", "g-banks", "Banco principal", "cliente-01", "2026-10-01T08:00:00Z", "https://banco.example"),
  entry("e-shop", "g-shopping", "Tienda de libros", "ana@example.com", "2026-04-18T19:00:00Z"),
  entry("e-old", "g-bin", "Cuenta vieja", "ana", "2026-01-09T12:00:00Z"),
];

// The shell asks Tauri for the version; outside Tauri the preview answers for it.
mockIPC((command) => {
  if (command === "plugin:app|version") return "0.0.2";
  throw new Error(`preview: unexpected command ${command}`);
});

const backend = createFakeBackend({
  status: closed
    ? { state: "closed" }
    : {
        state: "open",
        path: VAULT_PATH,
        name: "Cuentas",
        read_only: demo === "lectura",
        weak_kdf: demo === "lectura",
        save_pending: false,
        recovery: null,
      },
  recent: [{ path: VAULT_PATH, exists: true }],
  groups: GROUPS,
  entries: ENTRIES,
});

const container = document.getElementById("root");
if (container === null) throw new Error("Missing #root element in preview.html");

createRoot(container).render(
  <StrictMode>
    <App>
      <VaultRoot client={createClient(backend.transport)} />
    </App>
  </StrictMode>,
);

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, ms);
  });
}

/** The first element matching `selector` whose text is (or matches) `text`, once it exists. */
async function find(selector: string, text: string | RegExp): Promise<HTMLElement> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    for (const node of document.querySelectorAll<HTMLElement>(selector)) {
      const label = node.textContent.trim();
      if (typeof text === "string" ? label === text : text.test(label)) return node;
    }
    await sleep(30);
  }
  throw new Error(`preview: nothing matches ${selector} ${String(text)}`);
}

async function click(selector: string, text: string | RegExp) {
  (await find(selector, text)).click();
}

/** Puts the screen in the state worth looking at, with the same clicks a person would make. */
async function stage() {
  if (closed || demo === "ninguna" || demo === "lectura") return;
  await click(".rail-item", /^Trabajo/);
  if (demo === "nueva") {
    await click("button", "Nueva entrada");
    return;
  }
  if (demo === "copiado") {
    // Copy from one entry, then move on: the stamp must stay on the row that was copied.
    await click(".entries button", "Nómina");
    await click("button", "Copiar contraseña");
    await find(".stamp", /copiado/i);
  }
  await click(".entries button", "GitHub");
  await find("button", "Copiar contraseña");
  if (demo === "dialogo") await click("button", "Mover a otro grupo…");
}

/** What a screenshot cannot show by itself: the real viewport, horizontal overflow and whether the fonts loaded. */
function badge() {
  const node = document.createElement("output");
  node.style.cssText =
    "position:fixed;right:4px;bottom:4px;z-index:9;padding:1px 6px;font:11px monospace;color:#000;background:#ffe;";
  const root = document.documentElement;
  const fonts = document.fonts.check('16px "Overpass Variable"') && document.fonts.check('16px "Overpass Mono Variable"');
  node.textContent = [
    `${String(window.innerWidth)}x${String(window.innerHeight)}`,
    `scrollW ${String(root.scrollWidth)}`,
    `fuentes ${fonts ? "ok" : "NO"}`,
    `esquema ${getComputedStyle(root).colorScheme}`,
  ].join(" · ");
  document.body.append(node);
}

void stage()
  .catch((error: unknown) => {
    console.error(error);
  })
  .then(() => document.fonts.ready)
  .then(badge);
