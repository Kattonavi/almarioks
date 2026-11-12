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

function toolbar(name: string) {
  return screen.getByRole("group", { name: `Acciones del grupo «${name}»` });
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
    const opener = within(toolbar("Trabajo")).getByRole("button", { name: "Nuevo grupo…" });
    opener.focus();

    fireEvent.click(opener);
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

  it("RF-52: error — un nombre vacío o de más de 255 caracteres se rechaza en el diálogo", async () => {
    const { backend } = await setup();
    await selectGroup("Personal");
    fireEvent.click(within(toolbar("Personal")).getByRole("button", { name: "Nuevo grupo…" }));
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

    fireEvent.click(within(toolbar("Personal")).getByRole("button", { name: "Renombrar grupo…" }));
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

    fireEvent.click(within(toolbar("Trabajo")).getByRole("button", { name: "Mover grupo…" }));
    let dialog = await screen.findByRole("dialog", { name: "Mover «Trabajo»" });
    const labels = within(dialog).getAllByRole("option").map((option) => option.textContent);
    expect(labels).toEqual(["Cuentas › Personal", "Cuentas › Archivo"]);
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    await selectGroup("Clientes");
    expect(await nestedIn("Clientes", "Trabajo")).toBe(true);
    fireEvent.click(within(toolbar("Clientes")).getByRole("button", { name: "Mover grupo…" }));
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

    fireEvent.click(within(toolbar("Trabajo")).getByRole("button", { name: "Mover grupo…" }));
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

    await groupButton("Cuentas");
    const root = toolbar("Cuentas");
    expect(within(root).queryByRole("button", { name: "Mover grupo…" })).toBeNull();
    expect(within(root).queryByRole("button", { name: "Borrar grupo…" })).toBeNull();

    await selectGroup("Papelera");
    const bin = toolbar("Papelera");
    expect(within(bin).getAllByRole("button").map((button) => button.textContent)).toEqual(["Vaciar papelera…"]);

    await selectGroup("Viejo");
    fireEvent.click(within(toolbar("Viejo")).getByRole("button", { name: "Borrar grupo…" }));
    expect(await screen.findByText(ERROR_MESSAGES.GrupoProtegido)).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(backend.count("group_delete")).toBe(0);

    // RF-57 as corrected: a group that holds the bin can be renamed (only deleting it is protected)…
    fireEvent.click(within(toolbar("Viejo")).getByRole("button", { name: "Renombrar grupo…" }));
    let dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Otro" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar nombre" }));
    expect(await groupButton("Otro")).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();

    // …and so can the root, even though the bin is below it.
    await selectGroup("Cuentas");
    fireEvent.click(within(toolbar("Cuentas")).getByRole("button", { name: "Renombrar grupo…" }));
    dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Nombre del grupo"), { target: { value: "Mis cuentas" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Guardar nombre" }));
    expect(await groupButton("Mis cuentas")).toBeDefined();
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

    fireEvent.click(within(toolbar("Trabajo")).getByRole("button", { name: "Borrar grupo…" }));
    const dialog = await screen.findByRole("dialog", { name: "Borrar «Trabajo»" });
    expect(within(dialog).getByText(/contiene 5 entradas y 1 subgrupo/)).toBeDefined();
    expect(lastArgs(backend, "group_delete_preview")).toEqual({ id: "group-work" });
    expect(backend.count("group_delete")).toBe(0);

    fireEvent.click(within(dialog).getByRole("button", { name: "Mover a la papelera" }));

    expect(await screen.findByText("«Trabajo» se movió a la papelera.")).toBeDefined();
    expect(backend.groupTrail("group-work")).toEqual(["Cuentas", "Papelera de reciclaje", "Trabajo"]);
    expect(backend.groupTrail("group-clients")).toEqual(["Cuentas", "Papelera de reciclaje", "Trabajo", "Clientes"]);
    expect(await nestedIn("Trabajo", "Papelera de reciclaje")).toBe(true);
    expect(screen.getByRole("heading", { name: "Entradas de Cuentas" })).toBeDefined();
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Grupos" }));
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

    expect(within(toolbar("Viejo")).queryByRole("button", { name: "Nuevo grupo…" })).toBeNull();
    fireEvent.click(within(toolbar("Viejo")).getByRole("button", { name: "Borrar grupo…" }));
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

    fireEvent.click(within(toolbar("Papelera de reciclaje")).getByRole("button", { name: "Vaciar papelera…" }));
    const dialog = await screen.findByRole("dialog", { name: "Vaciar la papelera" });
    expect(within(dialog).getByText(/de forma definitiva 1 entrada y 0 subgrupos/)).toBeDefined();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.count("recycle_bin_empty")).toBe(0);

    fireEvent.click(within(toolbar("Papelera de reciclaje")).getByRole("button", { name: "Vaciar papelera…" }));
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

    fireEvent.click(within(toolbar("Archivo")).getByRole("button", { name: "Borrar grupo…" }));
    const dialog = await screen.findByRole("dialog", { name: "Borrar «Archivo»" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Mover a la papelera" }));

    expect(await within(dialog).findByText(ERROR_MESSAGES.GuardadoFallido)).toBeDefined();
    expect(backend.groupTrail("group-archive")).toEqual(["Cuentas", "Archivo"]);
  });
});
