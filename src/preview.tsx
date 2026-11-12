/*
 * Development-only preview (`/preview.html` under `pnpm dev`): the real screens on the fake core, with sample data,
 * to judge the design in a browser. It is not an entry of the production build.
 *
 *   ?tema=oscuro|claro      forces a theme (dark is the same file production loads by media query)
 *   ?estado=…               the screen and state to show (default: copiado):
 *       without a vault     bienvenida · bienvenida-vacia · crear · crear-vacia · crear-existe · crear-ubicacion ·
 *                           crear-llave · crear-llave-nueva · crear-llave-ubicacion · crear-error · creando ·
 *                           creada · desbloquear · desbloquear-sin-llave · derivando · error-llave ·
 *                           bloqueada · ajustes
 *       open vault          todas · foco-todas · menu-grupo · menu-papelera · busqueda ·
 *                           copiado · detalle · mostrada · menu-entrada · ninguna · formulario · generador · editar ·
 *                           dialogo-nueva-boveda · dialogo-nueva-boveda-generada · dialogo-mover ·
 *                           dialogo-borrar · dialogo-papelera · dialogo-renombrar · dialogo-recarga ·
 *                           aviso-guardado · cambio-externo · solo-lectura · ajustes-abierta
 *       entry dialog alone  dialogo-nueva · dialogo-nueva-generada · dialogo-opciones · dialogo-editar ·
 *                           dialogo-error (the dialog by itself over the work ground; formulario, generador and editar open it over the vault)
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
  keyFileBeside,
} from "./ipc/fake";
import "./styles.css";
import lightTokens from "./styles/tokens.css?inline";
import { EntryDialog, type EntryDialogMode } from "./vault/EntryDialog";
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
  "crear-llave-nueva",
  "crear-llave-ubicacion",
  "crear-error",
  "creando",
  "creada",
  "desbloquear",
  "desbloquear-sin-llave",
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
    case "desbloquear-sin-llave":
    case "derivando":
    case "error-llave":
      await click(".recent-row button", "cuentas.kdbx");
      if (state === "desbloquear-sin-llave") return;
      // RF-110: the key file beside the vault comes already chosen (see `files` below).
      if (state !== "derivando") await find(".unlock-key .mono", /\.keyx$/);
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
    case "crear-llave-nueva":
    case "crear-llave-ubicacion":
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
      if (state === "crear-ubicacion" || state === "crear-llave-ubicacion") {
        await click("button", "Cambiar ubicación…");
        await find(".create-file", NEW_PATH);
        if (state === "crear-ubicacion") return;
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
      } else {
        // RF-99: "Sin archivo llave" is the marked one; the rest of these states are about a new key file.
        await click("label", "Crear uno nuevo");
        await find("#create-key-help .create-file", /\.keyx$/);
      }
      await fill("create-password", "Mucho-Mejor-2026");
      await fill("create-confirm", "Mucho-Mejor-2026");
      await find("meter", /^/);
      if (state !== "creando" && state !== "creada") return;
      if (state === "creando") backend.hold("vault_create");
      await click("button", "Crear bóveda");
      await find(state === "creando" ? "progress" : "h2", state === "creando" ? /^/ : "Bóveda creada");
      return;
    default:
  }
}

/** Opens the menu of a group in the rail and chooses one of its orders. */
async function groupOrder(group: string, label: string) {
  await click(`button[aria-label="Más acciones de ${group}"]`, /^/);
  await click('[role="menuitem"]', label);
}

/** The measures of the approved layout and the colours a screenshot cannot tell apart, as they are on screen. */
function measures(): string[] {
  const box = (selector: string) => document.querySelector(selector)?.getBoundingClientRect();
  const px = (value: number | undefined) => (value === undefined ? "-" : String(Math.round(value * 100) / 100));
  const paint = (selector: string) => {
    const node = document.querySelector(selector);
    return node === null ? "-" : getComputedStyle(node).backgroundColor;
  };
  const menu = box(".menu");
  const font = (selector: string) => {
    const node = document.querySelector(selector);
    return node === null ? "-" : getComputedStyle(node).fontSize;
  };
  const pad = getComputedStyle(document.querySelector(".detail-pane") ?? document.body);
  const icons = [...document.querySelectorAll(".card-actions > button")].map((node) => node.getBoundingClientRect());
  const lastIcons = [...document.querySelectorAll(".card-actions > button:last-child")].map(
    (node) => node.getBoundingClientRect().right,
  );
  const rows = [...document.querySelectorAll(".card-row:not(.card-row-notes)")].map(
    (node) => node.getBoundingClientRect().height,
  );
  // `?medidas`: the measures of the list and the detail instead of those of the frame.
  if (params.has("medidas")) {
    return [
      `fila-lista ${px(box(".entries tbody tr")?.height)}`,
      `circulo-fila ${px(box(".entries .bullet")?.width)}x${px(box(".entries .bullet")?.height)}`,
      `titulo-lista ${font("#entries-title")}`,
      `circulo-detalle ${px(box(".entry-badge")?.width)}x${px(box(".entry-badge")?.height)}`,
      `titulo-entrada ${font(".entry-title")}`,
      `filas-tarjeta ${[...new Set(rows.map(px))].join("/")}`,
      `etiqueta ${px(box(".card-row > dt")?.width)}`,
      `iconos ${[...new Set(icons.map((icon) => `${px(icon.width)}x${px(icon.height)}`))].join("/")} (${String(icons.length)})`,
      // The right edge of the last button of every row: one value means they line up.
      `bordes-derechos ${[...new Set(lastIcons.map(px))].join("/")} (${String(lastIcons.length)} filas)`,
      `relleno-detalle ${pad.paddingTop}/${pad.paddingRight}/${pad.paddingBottom}/${pad.paddingLeft}`,
      `detalle ${px(box(".detail-pane")?.width)}`,
    ];
  }
  return [
    `barra ${px(box(".vault-bar")?.height)}`,
    `riel ${px(box(".rail")?.width)}`,
    `lista ${px(box(".list-pane")?.width)}`,
    `detalle ${px(box(".detail-pane")?.width)}`,
    `fila ${px(box(".rail-row")?.height)}`,
    `buscar ${px(box(".vault-search input")?.width)}x${px(box(".vault-search input")?.height)}`,
    `marcado ${paint('.rail-row[data-current], .rail-all[aria-current="true"]')}`,
    `menu ${menu === undefined ? "-" : `${px(menu.left)},${px(menu.top)} ${px(menu.width)}x${px(menu.height)} ${paint(".menu")}`}`,
  ];
}

const ENTRY_DIALOG = new Set([
  "dialogo-nueva",
  "dialogo-nueva-generada",
  "dialogo-opciones",
  "dialogo-editar",
  "dialogo-error",
]);

/** The measures of the entry dialog the approved prototype fixes, as the browser computed them. */
function entryDialogMeasures(): string[] {
  const node = (selector: string) => document.querySelector<HTMLElement>(selector);
  const px = (value: number) => String(Math.round(value * 100) / 100);
  const size = (selector: string) => {
    const box = node(selector)?.getBoundingClientRect();
    return box === undefined ? "-" : `${px(box.width)}x${px(box.height)}`;
  };
  const css = (selector: string, read: (style: CSSStyleDeclaration) => string) => {
    const found = node(selector);
    return found === null ? "-" : read(getComputedStyle(found));
  };
  const type = (selector: string) => css(selector, (style) => `${style.fontSize}/${style.fontWeight}`);
  const first = node("#entry-dialog-title")?.getBoundingClientRect();
  const second = node('label[for="entry-dialog-username"]')?.getBoundingClientRect();
  return [
    `dialogo ${size('dialog[data-layout="form"]')}`,
    `radio ${css('dialog[data-layout="form"]', (style) => style.borderTopLeftRadius)}`,
    `fondo ${css('dialog[data-layout="form"]', (style) => style.backgroundColor)}`,
    `titulo ${type(".dialog-head h2")}`,
    `grupo ${type(".dialog-lead")}`,
    `punto ${size(".dialog-dot")}`,
    `cerrar ${size(".dialog-close")}`,
    `etiqueta ${type(".entry-field > label")}`,
    `campo ${size("#entry-dialog-title")}`,
    `entre-campos ${first === undefined || second === undefined ? "-" : px(second.top - first.bottom)}`,
    `clave ${size("#entry-dialog-password")} caja ${size(".password-box")}`,
    `ojo ${size(".password-eye")}`,
    `generar ${size(".password-generate")}`,
    `fortaleza ${size(".strength-bar")}`,
    `opciones ${css(".generator-options", (style) => `${style.borderTopLeftRadius} ${style.backgroundColor}`)}`,
    `notas ${size("#entry-dialog-notes")}`,
    `cancelar ${size(".entry-dialog-foot > button:not(.btn-primary)")}`,
    `guardar ${size(".entry-dialog-foot > .btn-primary")}`,
  ];
}

/** The entry dialog by itself, over the work ground, to judge it without the vault behind. */
async function stageEntryDialog(container: HTMLElement, backend: FakeBackend) {
  const client = createClient(backend.transport);
  const group = { id: "g-work", name: "Trabajo", line: "2" } as const;
  let mode: EntryDialogMode = { kind: "new", group };
  if (state === "dialogo-editar") {
    const found = await client.entryGet("e-github");
    if (found.ok) mode = { kind: "edit", entry: found.value, group };
  }
  const root = createRoot(container);
  root.render(
    <StrictMode>
      <EntryDialog client={client} mode={mode} onClose={() => undefined} onSaved={() => undefined} />
    </StrictMode>,
  );
  await find("#entry-dialog-title", /^/);
  if (state === "dialogo-error") {
    await click("button", "Guardar entrada");
    await find(".field-error", /título/);
    return;
  }
  if (state === "dialogo-nueva-generada" || state === "dialogo-opciones") {
    await fill("entry-dialog-title", "GitHub");
    await fill("entry-dialog-username", "ana@example.com");
    await click("button", "Generar");
    await find(".strength-row", /Fortaleza/);
    await fill("entry-dialog-url", "https://github.com");
    if (state === "dialogo-opciones") await click("button", "Opciones del generador");
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
  // A save reached the disk: the foot of the rail says so.
  await find(".rail-all", /^Todas/);
  backend.emit("vault-save-state", "saved");
  if (state === "todas") return;
  if (state === "foco-todas") {
    (await find(".rail-all", /^Todas/)).focus();
    return;
  }
  if (state === "busqueda") {
    await fill("vault-search", "ana");
    await find(".list-title p", /resultado/);
    return;
  }
  await click(".rail-item", /^Trabajo/);
  switch (state) {
    case "menu-grupo":
      await click('button[aria-label="Más acciones de Trabajo"]', /^/);
      await find('[role="menuitem"]', /./);
      return;
    case "menu-papelera":
      // The menu of the last row of the rail: without room below, it opens upward.
      await click('button[aria-label="Más acciones de Papelera de reciclaje"]', /^/);
      await find('[role="menuitem"]', /./);
      return;
    case "dialogo-borrar":
      await groupOrder("Trabajo", "Enviar a la papelera…");
      return;
    case "dialogo-renombrar":
      await groupOrder("Trabajo", "Renombrar…");
      return;
    case "dialogo-papelera":
      await click(".rail-item", /^Papelera/);
      await groupOrder("Papelera de reciclaje", "Vaciar papelera…");
      return;
    case "formulario":
    case "generador":
      await click("button", "Nueva entrada");
      await fill("entry-dialog-title", "Banco de la esquina");
      await fill("entry-dialog-username", "ana@example.com");
      if (state === "formulario") {
        await fill("entry-dialog-password", "abcdefgh1");
        await find(".strength-row", /Fortaleza/);
        return;
      }
      await click("button", "Generar");
      await find(".strength-row", /Fortaleza/);
      return;
    case "aviso-guardado":
    case "cambio-externo":
    case "dialogo-recarga":
      backend.emit("vault-save-state", state === "aviso-guardado" ? "failed" : "external-change");
      if (state === "dialogo-recarga") await click("button", "Recargar descartando lo pendiente…");
      else await click(".entries button", "GitHub");
      return;
    default:
  }
  await click(".entries button", "GitHub");
  // Icon buttons have no text: they are found by their name.
  await find('button[aria-label="Copiar contraseña"]', /^/);
  if (state === "menu-entrada" || state === "dialogo-mover") {
    await click('button[aria-label="Más acciones de la entrada"]', /^/);
    await find('[role="menuitem"]', /./);
  }
  if (state === "dialogo-mover") await click('[role="menuitem"]', "Mover a otro grupo…");
  if (state === "editar") await click("button", "Editar");
  // Screen 2 of the approved prototype: the vault behind, as it was, and the new entry dialog over it.
  if (state === "dialogo-nueva-boveda" || state === "dialogo-nueva-boveda-generada") {
    await click("button", "Nueva entrada");
    await find("#entry-dialog-title", /^/);
    if (state === "dialogo-nueva-boveda-generada") {
      await fill("entry-dialog-title", "GitLab");
      await click("button", "Generar");
      await find(".strength-row", /Fortaleza/);
    }
  }
  if (state === "mostrada") {
    await click('button[aria-label="Mostrar contraseña"]', /^/);
    await find(".secret:not([data-masked])", /./);
  }
  if (state === "copiado") {
    await click('button[aria-label="Copiar contraseña"]', /^/);
    await find(".toast", /copiada/);
  }
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
    // The document is what scrolls; `max` also counts what is clipped on purpose (text only for screen readers).
    `ancho doc ${String(root.scrollWidth)} max ${String(Math.max(widest, root.scrollWidth))}`,
    `sans ${document.fonts.check('16px "Overpass Variable"') ? "ok" : "NO"}`,
    `esquema ${getComputedStyle(root).colorScheme}`,
    `foco ${document.activeElement?.textContent.trim().slice(0, 24) ?? ""}`,
    // The entry dialog brings its own measures, alone or over the vault.
    ...(document.querySelector('dialog[data-layout="form"]') === null ? measures() : entryDialogMeasures()),
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
            // The lock countdown of the bar, next to "Bloquear".
            locks_in_seconds: 252,
          },
    vaults: [{ path: VAULT_PATH, name: "Cuentas", password: "demo-clave" }],
    // "Crear bóveda" proposes this folder: a file already there is the "ya existe" state (`crear-existe`).
    files: [
      `${FAKE_DEFAULT_FOLDER}/Cuentas.kdbx`,
      // RF-110: in these states there is a key file beside the vault, so unlocking proposes it.
      ...(state === "desbloquear" || state === "error-llave" ? [keyFileBeside(VAULT_PATH)] : []),
    ],
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

  if (ENTRY_DIALOG.has(state)) {
    try {
      await stageEntryDialog(container, backend);
    } catch (error) {
      console.error(error);
    }
    await document.fonts.ready;
    badge();
    return;
  }

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
