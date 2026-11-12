/*
 * Development-only preview (`/preview.html` under `pnpm dev`): the real screens on the fake core, with sample data,
 * to judge the design in a browser. It is not an entry of the production build.
 *
 *   ?tema=oscuro|claro      forces a theme (dark is the same file production loads by media query)
 *   ?estado=…               the screen and state to show (default: copiado):
 *       without a vault     bienvenida · bienvenida-vacia · crear · crear-vacia · crear-existe · crear-ubicacion ·
 *                           crear-llave · crear-error · creando · creada · desbloquear · derivando · error-llave ·
 *                           bloqueada · ajustes
 *       open vault          copiado · detalle · ninguna · formulario · generador · editar · dialogo-mover ·
 *                           dialogo-borrar · dialogo-papelera · dialogo-renombrar · dialogo-recarga ·
 *                           aviso-guardado · cambio-externo · solo-lectura · ajustes-abierta
 *   ?densidad=compacta      the compact density, which no setting offers yet
 *   ?marco=480x360          draws the preview inside a frame of that exact viewport (for screenshots)
 *   ?hoja=a:b,c:d;e:f       with `marco`: one frame per `;` item, each overriding parameters as `name:value` pairs
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@fontsource-variable/overpass/index.css";
import "@fontsource-variable/overpass-mono/index.css";
import { App } from "./App";
import { createClient } from "./ipc/client";
import {
  createFakeBackend,
  FAKE_DEFAULT_FOLDER,
  type FakeBackend,
  type FakeEntry,
  type FakeGroup,
} from "./ipc/fake";
import "./styles.css";
import lightTokens from "./styles/tokens.css?inline";
import { VaultRoot } from "./vault/VaultRoot";

const params = new URLSearchParams(window.location.search);
const frame = /^(\d+)x(\d+)$/.exec(params.get("marco") ?? "");

/** The names the first version of this preview used. */
const LEGACY: Readonly<Record<string, string>> = {
  dialogo: "dialogo-mover",
  grupo: "dialogo-borrar",
  lectura: "solo-lectura",
  nueva: "formulario",
};
const asked = params.get("estado") ?? params.get("demo") ?? "copiado";
const state = asked === "cerrado" ? "bienvenida" : (LEGACY[asked] ?? asked);

const CLOSED = new Set([
  "bienvenida",
  "bienvenida-vacia",
  "crear",
  "crear-vacia",
  "crear-existe",
  "crear-ubicacion",
  "crear-llave",
  "crear-error",
  "creando",
  "creada",
  "desbloquear",
  "derivando",
  "error-llave",
  "ajustes",
]);

// Made-up paths: nothing here names a real folder.
const VAULT_PATH = "D:/bovedas/cuentas.kdbx";
const NEW_PATH = "D:/bovedas/familia.kdbx";
const KEY_PATH = "E:/llaves/familia.keyx";

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

/** Writes in a field the way a person would: the native value, then the `input` event React listens to. */
async function fill(id: string, value: string) {
  const field = await find(`#${id}`, /^/);
  const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(field, value);
  field.dispatchEvent(new Event("input", { bubbles: true }));
}

/** Puts a screen without a vault in the state worth looking at. */
async function stageClosed(backend: FakeBackend) {
  switch (state) {
    case "ajustes":
      await click("button", "Ajustes y Acerca de");
      await find("label", /^Bloquear la bóveda/);
      return;
    case "desbloquear":
    case "derivando":
    case "error-llave":
      await click(".recent-row button", "Abrir cuentas.kdbx");
      if (state === "desbloquear") return;
      if (state === "derivando") backend.holdUnlock();
      await fill("unlock-password", "no-es-la-clave");
      await click("button", "Desbloquear");
      await find(state === "derivando" ? "progress" : "#unlock-error", state === "derivando" ? /^/ : /./);
      return;
    case "crear":
    case "crear-vacia":
    case "crear-existe":
    case "crear-ubicacion":
    case "crear-llave":
    case "crear-error":
    case "creando":
    case "creada":
      await click("button", "Crear bóveda nueva");
      await find("h2", "Crear bóveda");
      await find(".create-file", /\.kdbx$/);
      if (state === "crear-vacia") return;
      // "Cuentas" is the file the fake disk already has in the proposed folder.
      await fill("create-name", state === "crear-existe" ? "Cuentas" : "Familia");
      await find(".create-file", state === "crear-existe" ? "Cuentas.kdbx" : "Familia.kdbx");
      if (state === "crear-existe") {
        await find("#create-name-error", /./);
        return;
      }
      if (state === "crear-ubicacion") {
        await click("button", "Cambiar ubicación…");
        await find(".create-file", NEW_PATH);
        return;
      }
      if (state === "crear-error") {
        await fill("create-password", "una-clave");
        await fill("create-confirm", "otra-clave");
        await find("#create-confirm-error", /./);
        return;
      }
      if (state === "crear") {
        await fill("create-password", "abcdefgh1");
        await find("meter", /^/);
        return;
      }
      if (state === "crear-llave") {
        await click("label", "Usar uno mío");
        await click("button", "Elegir archivo llave…");
        await find(".create-file", "familia.keyx");
      }
      await fill("create-password", "Mucho-Mejor-2026");
      await fill("create-confirm", "Mucho-Mejor-2026");
      await find("meter", /^/);
      if (state === "crear-llave") return;
      if (state === "creando") backend.hold("vault_create");
      await click("button", "Crear bóveda");
      await find(state === "creando" ? "progress" : "h2", state === "creando" ? /^/ : "Bóveda creada");
      return;
    default:
  }
}

/** Puts the open vault in the state worth looking at, with the same clicks a person would make. */
async function stageOpen(backend: FakeBackend) {
  if (state === "ninguna" || state === "solo-lectura") return;
  if (state === "ajustes-abierta") {
    await click("button", "Ajustes y Acerca de");
    await find("label", /^Bloquear la bóveda/);
    return;
  }
  await click(".rail-item", /^Trabajo/);
  switch (state) {
    case "dialogo-borrar":
      await click("button", "Enviar grupo a la papelera…");
      return;
    case "dialogo-renombrar":
      await click("button", "Renombrar grupo…");
      return;
    case "dialogo-papelera":
      await click(".rail-item", /^Papelera/);
      await click("button", "Vaciar papelera…");
      return;
    case "formulario":
    case "generador":
      await click("button", "Nueva entrada");
      await fill("entry-title", "Banco de la esquina");
      await fill("entry-username", "ana@example.com");
      if (state === "formulario") {
        await fill("entry-password", "abcdefgh1");
        await find("meter", /^/);
        return;
      }
      await click("button", "Generar contraseña");
      await click(".generator button", "Generar");
      await find(".generator output", /./);
      return;
    case "aviso-guardado":
    case "cambio-externo":
    case "dialogo-recarga":
      backend.emit("vault-save-state", state === "aviso-guardado" ? "failed" : "external-change");
      if (state === "dialogo-recarga") await click("button", "Recargar descartando lo pendiente…");
      else await click(".entries button", "GitHub");
      return;
    case "copiado":
      // Copy from one entry, then move on: the stamp must stay on the row that was copied.
      await click(".entries button", "Nómina");
      await click("button", "Copiar contraseña");
      await find(".stamp", /copiado/i);
      break;
    default:
  }
  await click(".entries button", "GitHub");
  await find("button", "Copiar contraseña");
  if (state === "dialogo-mover") await click("button", "Mover a otro grupo…");
  if (state === "editar") await click("button", "Editar");
}

/** What a screenshot cannot show by itself: the real viewport, horizontal overflow and whether the fonts loaded. */
function badge() {
  const node = document.createElement("output");
  node.style.cssText =
    "position:fixed;right:4px;bottom:2px;z-index:9;padding:1px 6px;font:11px monospace;color:#000;background:#ffe;";
  const root = document.documentElement;
  const widest = Math.max(...[...document.querySelectorAll("*")].map((element) => element.scrollWidth));
  node.textContent = [
    `${String(window.innerWidth)}x${String(window.innerHeight)}`,
    `ancho max ${String(Math.max(widest, root.scrollWidth))}`,
    `sans ${document.fonts.check('16px "Overpass Variable"') ? "ok" : "NO"}`,
    `esquema ${getComputedStyle(root).colorScheme}`,
    `foco ${document.activeElement?.textContent.trim().slice(0, 24) ?? ""}`,
  ].join(" · ");
  // Inside an open modal dialog everything else is inert and hidden behind the top layer.
  (document.querySelector("dialog[open]") ?? document.body).append(node);
}

async function boot() {
  const theme = params.get("tema");
  if (theme === "oscuro") await import("./styles/theme-dark.css");
  if (theme === "claro") {
    // The light colour roles again, after everything else: they win over the dark ones the system preference
    // brought. Colours only: the measures change with the window and must not be reset.
    const colors = [...lightTokens.matchAll(/(--[a-z0-9-]+):\s*((?:#|rgb\()[^;]+);/g)]
      .map((match) => `${match[1] ?? ""}:${match[2] ?? ""};`)
      .join("");
    const style = document.createElement("style");
    style.textContent = `:root,dialog{color-scheme:light;${colors}}`;
    document.head.append(style);
  }

  // The shell asks Tauri for the version; outside Tauri the preview answers for it.
  mockIPC((command) => {
    if (command === "plugin:app|version") return "0.0.2";
    throw new Error(`preview: unexpected command ${command}`);
  });

  const closed = CLOSED.has(state);
  const backend = createFakeBackend({
    status: closed
      ? { state: "closed" }
      : state === "bloqueada"
        ? { state: "locked", path: VAULT_PATH, recovery: { kind: "saved", path: "D:/bovedas/cuentas.recuperado.kdbx" } }
        : {
            state: "open",
            path: VAULT_PATH,
            name: "Cuentas",
            read_only: state === "solo-lectura",
            weak_kdf: state === "solo-lectura",
            save_pending: false,
            recovery: null,
          },
    vaults: [{ path: VAULT_PATH, name: "Cuentas", password: "demo-clave" }],
    // "Crear bóveda" proposes this folder: a file already there is the "ya existe" state (`crear-existe`).
    files: [`${FAKE_DEFAULT_FOLDER}/Cuentas.kdbx`],
    recent:
      state === "bienvenida-vacia"
        ? []
        : [
            { path: VAULT_PATH, exists: true },
            { path: "D:/bovedas/trabajo/equipo-de-proyectos.kdbx", exists: true },
            { path: "E:/respaldo/antigua.kdbx", exists: false },
          ],
    picks: {
      pick_vault_to_create: [NEW_PATH, NEW_PATH],
      pick_new_key_file: [KEY_PATH, KEY_PATH],
      pick_key_file: [KEY_PATH, KEY_PATH],
    },
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

  try {
    if (closed) await stageClosed(backend);
    else if (state !== "bloqueada") await stageOpen(backend);
  } catch (error) {
    console.error(error);
  }
  if (params.get("densidad") === "compacta") document.querySelector(".app")?.setAttribute("data-density", "compact");
  await document.fonts.ready;
  badge();
}

/** The same preview inside frames of an exact size: a headless window does not give an exact viewport. */
function host(width: string, height: string) {
  const base = new URLSearchParams(params);
  base.delete("marco");
  base.delete("hoja");
  const items = (params.get("hoja") ?? "").split(";");
  document.body.style.cssText =
    "display:flex;flex-wrap:wrap;gap:8px;align-items:flex-start;margin:0;padding:8px;background:#7a7a7a;";
  document.body.replaceChildren(
    ...items.map((item) => {
      const inner = new URLSearchParams(base);
      for (const pair of item.split(",")) {
        const [name, value] = pair.split(":");
        if (name !== undefined && name !== "" && value !== undefined) inner.set(name, value);
      }
      const node = document.createElement("iframe");
      node.src = `/preview.html?${inner.toString()}`;
      node.width = width;
      node.height = height;
      node.title = `Vista previa: ${inner.toString()}`;
      node.style.cssText = "display:block;border:0;";
      return node;
    }),
  );
}

if (frame === null) void boot();
else host(frame[1] ?? "", frame[2] ?? "");
