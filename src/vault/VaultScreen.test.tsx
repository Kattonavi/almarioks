import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { createClient } from "../ipc/client";
import { ERROR_MESSAGES } from "../ipc/errors";
import { createFakeBackend, SAMPLE_ENTRIES, SAMPLE_GROUPS, type FakeOptions } from "../ipc/fake";
import { VaultRoot } from "./VaultRoot";

afterEach(cleanup);

const PATH = "D:/bovedas/cuentas.kdbx";
const OPEN = {
  state: "open",
  path: PATH,
  name: "Cuentas",
  read_only: false,
  weak_kdf: false,
  save_pending: false,
  recovery: null,
} as const;

async function setup(options: FakeOptions = { groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES }) {
  const backend = createFakeBackend({ status: OPEN, ...options });
  const view = render(<VaultRoot client={createClient(backend.transport)} />);
  await screen.findByRole("heading", { name: "Cuentas" });
  return { backend, container: view.container };
}

const groupButton = (name: string) => screen.findByRole("button", { name: new RegExp(`^${name} `) });
const allButton = () => screen.findByRole("button", { name: /^Todas las entradas / });
const searchField = () => screen.getByRole<HTMLInputElement>("searchbox", { name: "Buscar en la bóveda" });

function entriesTable() {
  return screen.getByRole("table");
}

function rowTitles(): string[] {
  const rows = within(entriesTable()).getAllByRole("row").slice(1);
  return rows.map((row) => within(row).getByRole("button").textContent);
}

async function openGroup(name: string) {
  fireEvent.click(await groupButton(name));
  await screen.findByRole("heading", { name: `Entradas de ${name}` });
  await waitFor(() => {
    expect(screen.queryByText("Cargando las entradas…")).toBeNull();
  });
}

async function searchFor(query: string) {
  fireEvent.change(searchField(), { target: { value: query } });
  await waitFor(() => {
    expect(screen.queryByText("Buscando…")).toBeNull();
  });
}

describe("P4 Bóveda", () => {
  it("RF-50: muestra el árbol de grupos con el conteo de cada uno y la papelera diferenciada", async () => {
    await setup();

    // RF-102: the root is not a group of the rail; "Todas las entradas" opens first and is the marked one.
    const all = await allButton();
    expect(all.getAttribute("aria-current")).toBe("true");
    expect(all.textContent).toBe("Todas las entradas · 6 entradas");
    expect(screen.queryByRole("button", { name: /^Cuentas / })).toBeNull();
    expect((await groupButton("Personal")).textContent).toBe("Personal · 3 entradas");
    expect((await groupButton("Trabajo")).textContent).toBe("Trabajo · 2 entradas");
    expect((await groupButton("Clientes")).textContent).toBe("Clientes · 1 entrada");
    expect((await groupButton("Papelera de reciclaje")).textContent).toBe(
      "Papelera de reciclaje (papelera) · 1 entrada",
    );
    // The hierarchy is in the markup: "Clientes" sits inside the item of "Trabajo".
    const work = (await groupButton("Trabajo")).closest("li");
    expect(work?.contains(await groupButton("Clientes"))).toBe(true);
    expect(screen.getByRole("navigation", { name: "Grupos" })).toBeDefined();
  });

  it("RF-42: éxito — abrir un grupo lista sus entradas por título con usuario, URL, etiquetas y fecha", async () => {
    const { backend } = await setup();

    await openGroup("Personal");

    expect(rowTitles()).toEqual(["Años", "Banco", "Canción favorita"]);
    const song = within(entriesTable()).getByRole("button", { name: "Canción favorita" }).closest("tr");
    if (song === null) throw new Error("missing row");
    expect(within(song).getByText("ana")).toBeDefined();
    expect(within(song).getByText("https://musica.example")).toBeDefined();
    expect(within(song).getByText("música")).toBeDefined();
    expect(song.querySelector("time")?.getAttribute("datetime")).toBe("2026-11-02T09:30:00Z");
    expect(screen.getByText("3 entradas")).toBeDefined();
    expect((await groupButton("Personal")).getAttribute("aria-current")).toBe("true");
    expect(backend.calls.filter((call) => call.command === "entry_list").at(-1)?.args).toEqual({
      group_id: "group-personal",
    });
  });

  it("RF-42: un campo estándar protegido llega como null y se ve «(protegido)»", async () => {
    await setup();

    await openGroup("Trabajo");

    const vpn = within(entriesTable()).getByRole("button", { name: "VPN" }).closest("tr");
    if (vpn === null) throw new Error("missing row");
    expect(within(vpn).getByText("(protegido)")).toBeDefined();
  });

  it("RF-42: elegir una entrada la muestra en el panel de detalle", async () => {
    await setup();
    await openGroup("Trabajo");

    const github = within(entriesTable()).getByRole("button", { name: "GitHub" });
    fireEvent.click(github);

    const detail = screen.getByRole("region", { name: "Detalle" });
    expect(await within(detail).findByRole("heading", { name: "GitHub" })).toBeDefined();
    expect(await within(detail).findByRole("button", { name: "Editar" })).toBeDefined();
    expect(github.getAttribute("aria-current")).toBe("true");
  });

  it("RF-50: carga — mientras llegan los grupos lo dice", async () => {
    const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES });
    const release = backend.hold("group_tree");
    render(<VaultRoot client={createClient(backend.transport)} />);

    expect(await screen.findByText("Cargando los grupos…")).toBeDefined();
    await act(async () => {
      release();
      await Promise.resolve();
    });
    expect(await groupButton("Personal")).toBeDefined();
  });

  it("RF-83: error — si no se pueden leer los grupos o las entradas lo dice y deja reintentar", async () => {
    const backend = createFakeBackend({ status: OPEN, groups: SAMPLE_GROUPS, entries: SAMPLE_ENTRIES });
    backend.rejectOnce("group_tree", { code: "Bloqueada", message: "core text" });
    render(<VaultRoot client={createClient(backend.transport)} />);

    const alert = await screen.findByText(ERROR_MESSAGES.Bloqueada);
    expect(alert.textContent).not.toContain("core text");
    fireEvent.click(screen.getByRole("button", { name: "Reintentar" }));
    expect(await groupButton("Personal")).toBeDefined();

    // The screen reads the entries of the group it opens on by itself. Arm the rejection only after that read
    // has started, or it sometimes takes the rejection meant for the click below.
    await waitFor(() => {
      expect(backend.count("entry_list")).toBeGreaterThanOrEqual(1);
    });
    backend.rejectOnce("entry_list", { code: "NoEncontrado", message: "core text" });
    fireEvent.click(await groupButton("Personal"));
    expect(await screen.findByText(ERROR_MESSAGES.NoEncontrado)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Reintentar" }));
    await waitFor(() => {
      expect(rowTitles()).toEqual(["Años", "Banco", "Canción favorita"]);
    });
  });

  it("RF-42: vacío — una bóveda sin entradas lo explica y ofrece «Nueva entrada»", async () => {
    await setup({});

    expect(await screen.findByText(/Esta bóveda todavía no tiene entradas/)).toBeDefined();
    const create = screen.getByRole("button", { name: "Nueva entrada" });
    expect(create.getAttribute("aria-disabled")).not.toBe("true");
    expect(screen.queryByText("Próximamente")).toBeNull();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("RF-42: vacío — un grupo sin entradas lo dice", async () => {
    await setup();

    await openGroup("Archivo");

    expect(screen.getByText("Este grupo no tiene entradas.")).toBeDefined();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("RF-59: la búsqueda del ejemplo 2 muestra lo que da el núcleo, con la ruta del grupo y el número de resultados", async () => {
    const { backend } = await setup();

    await searchFor("CANCION");
    expect(rowTitles()).toEqual(["Canción favorita", "Banco"]);
    expect(screen.getByText("2 resultados")).toBeDefined();
    expect(screen.getAllByText("Cuentas › Personal")).toHaveLength(2);
    expect(screen.queryByText("Canción borrada")).toBeNull();

    await searchFor("ana git");
    expect(rowTitles()).toEqual(["GitHub"]);
    expect(screen.getByText("Cuentas › Trabajo")).toBeDefined();
    expect(screen.getByText("1 resultado")).toBeDefined();

    await searchFor("  banco  ");
    expect(rowTitles()).toEqual(["Banco"]);

    await searchFor("anos");
    expect(rowTitles()).toEqual(["Años"]);

    await searchFor("ana");
    expect(rowTitles()).toContain("Banana");
    expect(backend.calls.filter((call) => call.command === "search").at(-1)?.args).toEqual({ query: "ana" });
  });

  it("RF-59: limita las llamadas mientras escribe y descarta una respuesta vieja", async () => {
    const { backend } = await setup();

    fireEvent.change(searchField(), { target: { value: "g" } });
    fireEvent.change(searchField(), { target: { value: "gi" } });
    fireEvent.change(searchField(), { target: { value: "git" } });
    await waitFor(() => {
      expect(rowTitles()).toEqual(["GitHub"]);
    });
    expect(backend.count("search")).toBe(1);

    const release = backend.hold("search");
    fireEvent.change(searchField(), { target: { value: "banco" } });
    await waitFor(() => {
      expect(backend.count("search")).toBe(2);
    });
    fireEvent.change(searchField(), { target: { value: "vpn" } });
    await waitFor(() => {
      expect(rowTitles()).toEqual(["VPN"]);
    });
    await act(async () => {
      release();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(rowTitles()).toEqual(["VPN"]);
  });

  it("RF-64: vacío — una búsqueda sin resultados lo dice y ofrece limpiarla", async () => {
    await setup();
    await openGroup("Personal");

    await searchFor("zzz");
    expect(screen.getByText("Sin resultados.")).toBeDefined();
    expect(screen.getByText(/No hay entradas que coincidan con «zzz»/)).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Limpiar búsqueda" }));

    expect(searchField().value).toBe("");
    expect(document.activeElement).toBe(searchField());
    expect(screen.getByRole("heading", { name: "Entradas de Personal" })).toBeDefined();
    expect(rowTitles()).toEqual(["Años", "Banco", "Canción favorita"]);
  });

  it("RF-92: una consulta vacía después de recortarla no busca y muestra el grupo actual", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");

    await searchFor("git");
    expect(screen.getByRole("heading", { name: "Resultados de la búsqueda" })).toBeDefined();
    const calls = backend.count("search");

    fireEvent.change(searchField(), { target: { value: "   " } });
    await new Promise((resolve) => setTimeout(resolve, 200));

    expect(backend.count("search")).toBe(calls);
    expect(screen.getByRole("heading", { name: "Entradas de Trabajo" })).toBeDefined();
    expect(rowTitles()).toEqual(["GitHub", "VPN"]);
  });

  it("RF-59: Ctrl+F y Ctrl+K llevan al campo de búsqueda; Escape lo limpia", async () => {
    await setup();
    await groupButton("Personal");

    // The shortcut listener is attached in an effect: press again until it is there (focusing twice is harmless).
    await waitFor(() => {
      fireEvent.keyDown(window, { key: "f", ctrlKey: true });
      expect(document.activeElement).toBe(searchField());
    });
    (await groupButton("Personal")).focus();
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(document.activeElement).toBe(searchField());

    await searchFor("git");
    fireEvent.keyDown(searchField(), { key: "Escape" });
    expect(searchField().value).toBe("");
  });

  it("RF-78: el contenido de la bóveda se pinta como texto, nunca como HTML ni enlace", async () => {
    const payloads = {
      group: "<img src=x onerror=alert(1)>",
      title: "<script>alert(2)</script>",
      username: "<img src=x onerror=alert(3)>",
      url: "javascript:alert(4)",
      tag: "<b onclick=alert(5)>negrita</b>",
    };
    const { container } = await setup({
      groups: { id: "r", name: "Cuentas", kind: "root", children: [{ id: "x", name: payloads.group }] },
      entries: [
        {
          id: "e",
          groupId: "x",
          title: payloads.title,
          username: payloads.username,
          url: payloads.url,
          tags: [payloads.tag],
        },
      ],
    });

    const group = await screen.findByRole("button", { name: `${payloads.group} · 1 entrada` });
    fireEvent.click(group);
    await screen.findByRole("button", { name: payloads.title });
    fireEvent.click(screen.getByRole("button", { name: payloads.title }));

    function expectLiteral() {
      for (const text of [payloads.title, payloads.username, payloads.url, payloads.tag]) {
        for (const node of screen.getAllByText(text)) {
          expect([...node.childNodes].every((child) => child.nodeType === Node.TEXT_NODE)).toBe(true);
        }
      }
      expect(container.querySelector("script, img, b, a, iframe")).toBeNull();
      expect(container.querySelector('[href^="javascript"], [onerror], [onclick]')).toBeNull();
    }
    expectLiteral();

    await searchFor("alert");
    expect(screen.getByText(`Cuentas › ${payloads.group}`)).toBeDefined();
    expectLiteral();
  });

  it("RF-32: bloqueada — al bloquearse retira el árbol, las entradas y la búsqueda y muestra P3", async () => {
    const { backend } = await setup();
    await openGroup("Personal");
    fireEvent.change(searchField(), { target: { value: "banco" } });

    act(() => {
      backend.lockFromCore();
    });

    expect(screen.queryByRole("button", { name: /^Personal / })).toBeNull();
    expect(screen.queryByText("Canción favorita")).toBeNull();
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(await screen.findByRole("heading", { name: "Desbloquear cuentas.kdbx" })).toBeDefined();
    expect(screen.queryByText("Banco")).toBeNull();
  });
});

describe("P4 Bóveda — «Todas las entradas»", () => {
  it("RF-102: al abrir queda elegido «Todas las entradas», con las entradas de todos los grupos por título y sin la papelera", async () => {
    const { backend } = await setup();

    expect(await screen.findByRole("heading", { name: "Todas las entradas" })).toBeDefined();
    await waitFor(() => {
      expect(rowTitles()).toEqual(["Años", "Banana", "Banco", "Canción favorita", "GitHub", "VPN"]);
    });
    expect(screen.getByText("6 entradas")).toBeDefined();
    expect(screen.queryByText("Canción borrada")).toBeNull();
    // The core has no command for it: the window asks for each group outside the bin, and never for the bin.
    const asked = backend.calls.filter((call) => call.command === "entry_list").map((call) => call.args);
    for (const id of ["group-root", "group-personal", "group-work", "group-clients", "group-archive"]) {
      expect(asked).toContainEqual({ group_id: id });
    }
    expect(asked).not.toContainEqual({ group_id: "group-bin" });
  });

  it("RF-102: el total suma los grupos fuera de la papelera y el grupo raíz no aparece como un grupo más", async () => {
    await setup({
      groups: SAMPLE_GROUPS,
      entries: [...SAMPLE_ENTRIES, { id: "entry-root", groupId: "group-root", title: "Wifi de casa" }],
    });

    expect((await allButton()).textContent).toBe("Todas las entradas · 7 entradas");
    // The entries that hang from the root itself are seen here, and only here.
    await waitFor(() => {
      expect(rowTitles()).toContain("Wifi de casa");
    });
    expect(screen.queryByRole("button", { name: /^Cuentas / })).toBeNull();
    // It is the first element of the rail, before every group.
    const rail = screen.getByRole("navigation", { name: "Grupos" });
    expect(within(rail).getAllByRole("button")[0]).toBe(await allButton());

    await openGroup("Personal");
    expect(rowTitles()).not.toContain("Wifi de casa");
    fireEvent.click(await allButton());
    await screen.findByRole("heading", { name: "Todas las entradas" });
    await waitFor(() => {
      expect(rowTitles()).toHaveLength(7);
    });
  });

  it("RF-102: «Nueva entrada» desde «Todas las entradas» crea la entrada en la raíz y la lista sigue en «Todas las entradas»", async () => {
    const { backend } = await setup();
    await screen.findByRole("heading", { name: "Todas las entradas" });

    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    fireEvent.change(await screen.findByLabelText("Título"), { target: { value: "Wifi de casa" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));

    await waitFor(() => {
      expect(backend.calls.filter((call) => call.command === "entry_create").at(-1)?.args).toMatchObject({
        group_id: "group-root",
        title: "Wifi de casa",
      });
    });
    await waitFor(() => {
      expect(rowTitles()).toContain("Wifi de casa");
    });
    expect(screen.getByRole("heading", { name: "Todas las entradas" })).toBeDefined();
    expect((await allButton()).textContent).toBe("Todas las entradas · 7 entradas");
  });

  it("RF-102: mientras se busca nada queda marcado en el riel, y elegir un grupo o «Todas las entradas» termina la búsqueda", async () => {
    await setup();
    await openGroup("Personal");

    await searchFor("git");
    expect((await groupButton("Personal")).getAttribute("aria-current")).toBeNull();
    expect((await allButton()).getAttribute("aria-current")).toBeNull();

    fireEvent.click(await groupButton("Trabajo"));
    await screen.findByRole("heading", { name: "Entradas de Trabajo" });
    expect(searchField().value).toBe("");
    expect((await groupButton("Trabajo")).getAttribute("aria-current")).toBe("true");

    await searchFor("banco");
    fireEvent.click(await allButton());
    await screen.findByRole("heading", { name: "Todas las entradas" });
    expect(searchField().value).toBe("");
    expect((await allButton()).getAttribute("aria-current")).toBe("true");
    expect((await groupButton("Trabajo")).getAttribute("aria-current")).toBeNull();
  });
});
