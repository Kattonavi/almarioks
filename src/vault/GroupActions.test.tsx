import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeGroup, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";

afterEach(cleanup);

const OPEN = {
  state: "open",
  path: "D:/bovedas/cuentas.kdbx",
  name: "Cuentas",
  read_only: false,
  weak_kdf: false,
  save_pending: false,
  recovery: null,
} as const;

async function setup(options: FakeOptions = {}) {
  const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES, ...options });
  render(<VaultRoot client={createClient(backend.transport)} />);
  await screen.findByRole("heading", { name: "Cuentas" });
  return { backend };
}

const groupButton = (name: string) => screen.findByRole("button", { name: new RegExp(`^${name} `) });
const lastArgs = (backend: { calls: readonly { command: string; args: unknown }[] }, command: string) =>
  backend.calls.filter((call) => call.command === command).at(-1)?.args;

async function selectGroup(name: string) {
  fireEvent.click(await groupButton(name));
  await screen.findByRole("heading", { name: `Entradas de ${name}` });
}

/** Opens the menu of a group with its "more actions" button. */
async function openMenu(name: string) {
  const more = await screen.findByRole("button", { name: `Más acciones de ${name}` });
  fireEvent.click(more);
  return { more, menu: await screen.findByRole("menu", { name: `Acciones de ${name}` }) };
}

function ordersOf(menu: HTMLElement): (string | null)[] {
  return within(menu).getAllByRole("menuitem").map((item) => item.textContent);
}

/** Chooses an order in the menu of a group; returns the button the focus must go back to. */
async function order(name: string, label: string) {
  const { more, menu } = await openMenu(name);
  fireEvent.click(within(menu).getByRole("menuitem", { name: label }));
  return more;
}

/** True if the tree item of `child` sits inside the tree item of `parent`. */
async function nestedIn(child: string, parent: string): Promise<boolean> {
  const item = (await groupButton(parent)).closest("li");
  return item?.contains(await groupButton(child)) === true;
}

describe("P7 Grupo", () => {
  it("RF-51: éxito — crea un grupo dentro del grupo elegido y devuelve el foco al cerrar", async () => {
    const { backend } = await setup();
    await selectGroup("Trabajo");

    const opener = await order("Trabajo", "Nuevo subgrupo…");
    const dialog = await screen.findByRole("dialog", { name: "Nuevo grupo dentro de «Trabajo»" });
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "  Proyectos  " } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Crear grupo" }));

    expect(await screen.findByText("Grupo creado.")).toBeDefined();
    expect(lastArgs(backend, "group_create")).toEqual({ parent_id: "group-work", name: "  Proyectos  " });
    expect((await groupButton("Proyectos")).textContent).toBe("Proyectos · 0 entradas");
    expect(await nestedIn("Proyectos", "Trabajo")).toBe(true);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it("RF-51: el «+» junto a «Grupos» crea un grupo en la raíz, sin cambiar lo que muestra la lista", async () => {
    const { backend } = await setup();
    await selectGroup("Personal");
    const add = screen.getByRole("button", { name: "Nuevo grupo" });
    add.focus();

    fireEvent.click(add);
    const dialog = await screen.findByRole("dialog", { name: "Nuevo grupo dentro de «Cuentas»" });
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Viajes" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Crear grupo" }));

    expect(await screen.findByText("Grupo creado.")).toBeDefined();
    expect(lastArgs(backend, "group_create")).toEqual({ parent_id: "group-root", name: "Viajes" });
    expect((await groupButton("Viajes")).textContent).toBe("Viajes · 0 entradas");
    expect(await nestedIn("Viajes", "Personal")).toBe(false);
    expect(await nestedIn("Viajes", "Trabajo")).toBe(false);
    expect(screen.getByRole("heading", { name: "Entradas de Personal" })).toBeDefined();
    expect(document.activeElement).toBe(add);
  });

  it("RF-52: error — un nombre vacío o de más de 255 caracteres se rechaza en el diálogo", async () => {
    const { backend } = await setup();
    await selectGroup("Personal");
    await order("Personal", "Nuevo subgrupo…");
    const dialog = await screen.findByRole("dialog");
    const name = within(dialog).getByLabelText("Nombre del grupo");

    fireEvent.change(name, { target: { value: "   " } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Crear grupo" }));
    expect(await within(dialog).findByText(ERROR_MESSAGES.NombreVacio)).toBeDefined();
    expect(name.getAttribute("aria-invalid")).toBe("true");

    fireEvent.change(name, { target: { value: "g".repeat(256) } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Crear grupo" }));
    expect(await within(dialog).findByText(/admite hasta 255 caracteres/)).toBeDefined();
    expect(backend.count("group_create")).toBe(0);

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("RF-53: renombra con el nombre actual precargado", async () => {
    const { backend } = await setup();
    await selectGroup("Personal");

    await order("Personal", "Renombrar…");
    const dialog = await screen.findByRole("dialog", { name: "Renombrar «Personal»" });
    const name = within(dialog).getByLabelText<HTMLInputElement>("Nombre del grupo");
    expect(name.value).toBe("Personal");
    fireEvent.change(name, { target: { value: " Casa " } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar nombre" }));

    expect(await groupButton("Casa")).toBeDefined();
    expect(lastArgs(backend, "group_rename")).toEqual({ id: "group-personal", name: " Casa " });
    expect(screen.getByRole("heading", { name: "Entradas de Casa" })).toBeDefined();
  });

  it("RF-54: RF-55: ejemplo 3 —mover por menú; el grupo no se ofrece dentro de sí ni de sus hijos", async () => {
    const { backend } = await setup();
    await selectGroup("Trabajo");

    await order("Trabajo", "Mover a otro grupo…");
    let dialog = await screen.findByRole("dialog", { name: "Mover «Trabajo»" });
    const labels = within(dialog).getAllByRole("option").map((option) => option.textContent);
    expect(labels).toEqual(["Cuentas › Personal", "Cuentas › Archivo"]);
    // D-7: the missing destinations are explained, tied to the control.
    const select = within(dialog).getByLabelText("Grupo de destino");
    const help = document.getElementById(select.getAttribute("aria-describedby") ?? "");
    expect(help?.textContent).toBe("No aparecen el propio grupo ni sus subgrupos: un grupo no puede ir dentro de sí mismo.");
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    await selectGroup("Clientes");
    expect(await nestedIn("Clientes", "Trabajo")).toBe(true);
    await order("Clientes", "Mover a otro grupo…");
    dialog = await screen.findByRole("dialog", { name: "Mover «Clientes»" });
    fireEvent.change(within(dialog).getByLabelText("Grupo de destino"), { target: { value: "group-root" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Mover" }));

    expect(await screen.findByText("«Clientes» se movió a «Cuentas».")).toBeDefined();
    expect(lastArgs(backend, "group_move")).toEqual({ id: "group-clients", parent_id: "group-root" });
    expect(await nestedIn("Clientes", "Trabajo")).toBe(false);
    expect(backend.groupTrail("group-clients")).toEqual(["Cuentas", "Clientes"]);
    expect((await groupButton("Clientes")).textContent).toBe("Clientes · 1 entrada");
  });

  it("RF-55: error — si el núcleo rechaza el movimiento lo explica dentro del diálogo", async () => {
    const { backend } = await setup();
    await selectGroup("Trabajo");
    backend.rejectOnce("group_move", { code: "MovimientoInvalido", message: "core text" });

    await order("Trabajo", "Mover a otro grupo…");
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Mover" }));

    expect(await within(dialog).findByText(ERROR_MESSAGES.MovimientoInvalido)).toBeDefined();
    expect(await nestedIn("Clientes", "Trabajo")).toBe(true);
  });

  it("RF-57: la raíz no se mueve ni se borra, la papelera solo se vacía y un grupo que la contiene es protegido", async () => {
    const groups: FakeGroup = {
      id: "group-root",
      name: "Cuentas",
      kind: "root",
      children: [
        { id: "group-personal", name: "Personal" },
        { id: "group-old", name: "Viejo", children: [{ id: "group-bin", name: "Papelera", kind: "recycle_bin" }] },
      ],
    };
    const { backend } = await setup({ groups, entries: [] });

    // The root is not a row of the rail: what can be asked of it is in the menu next to the name of the vault.
    const root = await openMenu("Cuentas");
    expect(ordersOf(root.menu)).toEqual(["Renombrar…"]);
    fireEvent.keyDown(root.menu, { key: "Escape" });

    await selectGroup("Papelera");
    const bin = await openMenu("Papelera");
    expect(ordersOf(bin.menu)).toEqual(["Vaciar papelera…"]);
    fireEvent.keyDown(bin.menu, { key: "Escape" });

    await selectGroup("Viejo");
    await order("Viejo", "Enviar a la papelera…");
    expect(await screen.findByText(ERROR_MESSAGES.GrupoProtegido)).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(backend.count("group_delete")).toBe(0);

    // RF-57 as corrected: a group that holds the bin can be renamed (only deleting it is protected)…
    await order("Viejo", "Renombrar…");
    let dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Otro" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar nombre" }));
    expect(await groupButton("Otro")).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();

    // …and so can the root, even though the bin is below it.
    await order("Cuentas", "Renombrar…");
    dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Mis cuentas" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar nombre" }));
    expect(await screen.findByRole("button", { name: "Más acciones de Mis cuentas" })).toBeDefined();
    expect(backend.groupTrail("group-bin")).toEqual(["Mis cuentas", "Otro", "Papelera"]);
  });

  it("RF-56: ejemplo 4 — el aviso dice «5 entradas y 1 subgrupo» y al confirmar todo queda en la papelera", async () => {
    const entries = [
      { id: "w1", groupId: "group-work", title: "Uno" },
      { id: "w2", groupId: "group-work", title: "Dos" },
      { id: "w3", groupId: "group-work", title: "Tres" },
      { id: "c1", groupId: "group-clients", title: "Cuatro" },
      { id: "c2", groupId: "group-clients", title: "Cinco" },
    ];
    const { backend } = await setup({ entries });
    await selectGroup("Trabajo");

    await order("Trabajo", "Enviar a la papelera…");
    const dialog = await screen.findByRole("dialog", { name: "Enviar «Trabajo» a la papelera" });
    expect(within(dialog).getByText(/contiene 5 entradas y 1 subgrupo/)).toBeDefined();
    expect(lastArgs(backend, "group_delete_preview")).toEqual({ id: "group-work" });
    expect(backend.count("group_delete")).toBe(0);

    fireEvent.click(within(dialog).getByRole("button", { name: "Enviar a la papelera" }));

    expect(await screen.findByText("«Trabajo» se movió a la papelera.")).toBeDefined();
    expect(backend.groupTrail("group-work")).toEqual(["Cuentas", "Papelera de reciclaje", "Trabajo"]);
    expect(backend.groupTrail("group-clients")).toEqual(["Cuentas", "Papelera de reciclaje", "Trabajo", "Clientes"]);
    expect(await nestedIn("Trabajo", "Papelera de reciclaje")).toBe(true);
    // The list was showing the group that left: it goes back to "Todas las entradas" (RF-102).
    expect(screen.getByRole("heading", { name: "Todas las entradas" })).toBeDefined();
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Grupos" }));
    });
  });

  it("RF-58: borrar un grupo que ya está en la papelera avisa que es definitivo y lo elimina", async () => {
    const groups: FakeGroup = {
      id: "group-root",
      name: "Cuentas",
      kind: "root",
      children: [
        {
          id: "group-bin",
          name: "Papelera de reciclaje",
          kind: "recycle_bin",
          children: [{ id: "group-gone", name: "Viejo" }],
        },
      ],
    };
    const { backend } = await setup({ groups, entries: [{ id: "x", groupId: "group-gone", title: "Vieja" }] });
    await selectGroup("Viejo");

    const { menu } = await openMenu("Viejo");
    // Nothing new is made inside the bin, and what is in it is deleted for good.
    expect(ordersOf(menu)).toEqual(["Renombrar…", "Mover a otro grupo…", "Borrar…"]);
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Borrar…" }));
    const dialog = await screen.findByRole("dialog", { name: "Borrar «Viejo» de forma definitiva" });
    expect(within(dialog).getByText(/1 entrada y 0 subgrupos.*no se puede deshacer/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Borrar definitivamente" }));

    expect(await screen.findByText("Se borró «Viejo» de forma definitiva.")).toBeDefined();
    expect(backend.groupTrail("group-gone")).toBeUndefined();
    expect(backend.entry("x")).toBeUndefined();
  });

  it("RF-58: vaciar la papelera confirma, dice que es definitivo y la deja vacía", async () => {
    const { backend } = await setup();
    await selectGroup("Papelera de reciclaje");
    await screen.findByRole("button", { name: "Canción borrada" });

    await order("Papelera de reciclaje", "Vaciar papelera…");
    const dialog = await screen.findByRole("dialog", { name: "Vaciar la papelera" });
    expect(within(dialog).getByText(/de forma definitiva 1 entrada y 0 subgrupos/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.count("recycle_bin_empty")).toBe(0);

    await order("Papelera de reciclaje", "Vaciar papelera…");
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Vaciar papelera" }));

    expect(await screen.findByText("La papelera quedó vacía.")).toBeDefined();
    expect(backend.entry("entry-old")).toBeUndefined();
    await waitFor(() => {
      expect(screen.getByText("La papelera está vacía.")).toBeDefined();
    });
    expect((await groupButton("Papelera de reciclaje")).textContent).toBe(
      "Papelera de reciclaje (papelera) · 0 entradas",
    );
  });

  it("RF-83: error — si borrar falla al confirmar, el diálogo lo dice y sigue abierto", async () => {
    const { backend } = await setup();
    await selectGroup("Archivo");
    backend.rejectOnce("group_delete", { code: "GuardadoFallido", message: "core text" });

    await order("Archivo", "Enviar a la papelera…");
    const dialog = await screen.findByRole("dialog", { name: "Enviar «Archivo» a la papelera" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Enviar a la papelera" }));

    expect(await within(dialog).findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(backend.groupTrail("group-archive")).toEqual(["Cuentas", "Archivo"]);
  });
});

describe("P7 Grupo — el menú del grupo", () => {
  it("RF-102: menú del grupo — se abre con su botón, con la orden destructiva al final, y Escape lo cierra con el foco de vuelta", async () => {
    await setup();

    const closed = await screen.findByRole("button", { name: "Más acciones de Trabajo" });
    expect(closed.getAttribute("aria-haspopup")).toBe("menu");
    expect(closed.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("menu")).toBeNull();

    const { more, menu } = await openMenu("Trabajo");
    expect(more.getAttribute("aria-expanded")).toBe("true");
    expect(ordersOf(menu)).toEqual(["Nuevo subgrupo…", "Renombrar…", "Mover a otro grupo…", "Enviar a la papelera…"]);
    const first = within(menu).getAllByRole("menuitem")[0];
    await waitFor(() => {
      expect(document.activeElement).toBe(first);
    });

    fireEvent.keyDown(first ?? menu, { key: "Escape" });

    expect(screen.queryByRole("menu")).toBeNull();
    expect(more.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).toBe(more);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("RF-102: menú del grupo — las flechas lo recorren en círculo y un clic fuera o el mismo botón lo cierran", async () => {
    await setup();
    const { more, menu } = await openMenu("Personal");
    const items = within(menu).getAllByRole("menuitem");
    await waitFor(() => {
      expect(document.activeElement).toBe(items[0]);
    });

    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toBe(items[1]);
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(document.activeElement).toBe(items.at(-1));
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toBe(items[0]);
    fireEvent.keyDown(menu, { key: "End" });
    expect(document.activeElement).toBe(items.at(-1));
    fireEvent.keyDown(menu, { key: "Home" });
    expect(document.activeElement).toBe(items[0]);

    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();

    fireEvent.click(more);
    expect(await screen.findByRole("menu")).toBeDefined();
    fireEvent.click(more);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("RF-102: menú del grupo — cada orden abre su diálogo y al cancelarlo el foco vuelve al botón del menú", async () => {
    await setup();
    const dialogs = [
      ["Nuevo subgrupo…", "Nuevo grupo dentro de «Trabajo»"],
      ["Renombrar…", "Renombrar «Trabajo»"],
      ["Mover a otro grupo…", "Mover «Trabajo»"],
      ["Enviar a la papelera…", "Enviar «Trabajo» a la papelera"],
    ] as const;

    for (const [label, title] of dialogs) {
      const more = await order("Trabajo", label);
      const dialog = await screen.findByRole("dialog", { name: title });
      expect(screen.queryByRole("menu")).toBeNull();
      fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
      await waitFor(() => {
        expect(screen.queryByRole("dialog")).toBeNull();
        expect(document.activeElement).toBe(more);
      });
    }
  });
});
