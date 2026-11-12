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
  // The title button of each row: the chosen row has two more buttons, at its end.
  return rows.map((row) => within(within(row).getByRole("rowheader")).getByRole("button").textContent);
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
    // The row shows the site without `https://`; the `title` of its cell is the help of the double click (RF-106).
    expect(within(song).getByText("musica.example").closest("td")?.getAttribute("title")).toBe(
      "Doble clic para abrir el sitio en el navegador",
    );
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

  it("RF-42: RF-43: abrir una entrada la muestra en su ventana y su fila queda elegida", async () => {
    await setup();
    await openGroup("Trabajo");

    const github = within(entriesTable()).getByRole("button", { name: "GitHub" });
    fireEvent.click(github);
    fireEvent.doubleClick(github);

    const detail = await screen.findByRole("dialog", { name: "GitHub" });
    expect(await within(detail).findByRole("heading", { name: "GitHub" })).toBeDefined();
    expect(await within(detail).findByRole("button", { name: "Editar" })).toBeDefined();
    expect(github.getAttribute("aria-current")).toBe("true");
    // RF-42: two columns. The detail zone and its invitation are gone.
    expect(screen.queryByRole("region", { name: "Detalle" })).toBeNull();
    expect(screen.queryByText("Elige una entrada para ver su detalle.")).toBeNull();
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

    expect(await screen.findByText("No hay entradas aquí. Crea la primera con «Nueva entrada».")).toBeDefined();
    expect(screen.getByText("0 entradas")).toBeDefined();
    const create = screen.getByRole("button", { name: "Nueva entrada" });
    expect(create.getAttribute("aria-disabled")).not.toBe("true");
    expect(screen.queryByText("Próximamente")).toBeNull();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("RF-42: vacío — un grupo sin entradas lo dice", async () => {
    await setup();

    await openGroup("Archivo");

    expect(screen.getByText("No hay entradas aquí. Crea la primera con «Nueva entrada».")).toBeDefined();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("RF-42: RF-14: vacío — en solo lectura no nombra «Nueva entrada», que no se ofrece; la papelera vacía dice lo suyo", async () => {
    await setup({
      status: { ...OPEN, read_only: true },
      groups: SAMPLE_GROUPS,
      entries: SAMPLE_ENTRIES.filter((entry) => entry.groupId !== "group-bin"),
    });

    await openGroup("Archivo");
    expect(screen.getByText("No hay entradas aquí.")).toBeDefined();
    expect(screen.queryByRole("button", { name: "Nueva entrada" })).toBeNull();

    await openGroup("Papelera de reciclaje");
    expect(screen.getByText("La papelera está vacía.")).toBeDefined();
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
    // The window of the entry too: a double click on the title opens it.
    fireEvent.doubleClick(screen.getByRole("button", { name: payloads.title }));
    await screen.findByRole("dialog", { name: payloads.title });

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

describe("P4 Bóveda — la fila de una entrada", () => {
  /** The circle of each row, in order: the initial it draws and the line colour under it. */
  function circles(): string[] {
    const rows = within(entriesTable()).getAllByRole("row").slice(1);
    return rows.map((row) => {
      const circle = row.querySelector(".bullet");
      return `${circle?.getAttribute("data-initial") ?? "?"}:${circle?.getAttribute("data-line") ?? "?"}`;
    });
  }

  it("RF-42: cada fila lleva la inicial de su título, no la del grupo, sobre el color de su grupo", async () => {
    await setup();
    await openGroup("Personal");

    expect(rowTitles()).toEqual(["Años", "Banco", "Canción favorita"]);
    expect(circles()).toEqual(["A:1", "B:1", "C:1"]);
    // Decoration: it adds nothing to the name of the row's button, and it is hidden from assistive technology.
    const circle = within(entriesTable()).getAllByRole("row")[1]?.querySelector(".bullet");
    expect(circle?.getAttribute("aria-hidden")).toBe("true");
    expect(circle?.textContent).toBe("");
  });

  it("RF-42: RF-102: en «Todas las entradas» cada fila lleva el color de SU grupo; un subgrupo, el de su grupo de primer nivel", async () => {
    await setup({
      groups: SAMPLE_GROUPS,
      entries: [...SAMPLE_ENTRIES, { id: "entry-root", groupId: "group-root", title: "Wifi de casa" }],
    });

    await waitFor(() => {
      expect(rowTitles()).toEqual(["Años", "Banana", "Banco", "Canción favorita", "GitHub", "VPN", "Wifi de casa"]);
    });
    // Personal is line 1 and Trabajo line 2; "Banana" is in Clientes, under Trabajo. The root itself has no line.
    expect(circles()).toEqual(["A:1", "B:2", "B:1", "C:1", "G:2", "V:2", "W:neutral"]);
  });

  it("RF-59: en los resultados de la búsqueda cada fila lleva la inicial de su título y el color de su grupo", async () => {
    await setup();

    await searchFor("ana");

    expect(rowTitles()).toEqual(expect.arrayContaining(["Banana", "GitHub", "Canción favorita"]));
    const byTitle = new Map(rowTitles().map((title, index) => [title, circles()[index]]));
    expect(byTitle.get("Banana")).toBe("B:2");
    expect(byTitle.get("GitHub")).toBe("G:2");
    expect(byTitle.get("Canción favorita")).toBe("C:1");
  });

  it("RF-42: un título vacío, protegido o sin letras ni números lleva el signo neutro «#»", async () => {
    await setup({
      groups: SAMPLE_GROUPS,
      entries: [
        { id: "e1", groupId: "group-personal", title: "" },
        { id: "e2", groupId: "group-personal", title: null },
        { id: "e3", groupId: "group-personal", title: "—·—" },
        { id: "e4", groupId: "group-personal", title: "  ñandú" },
        { id: "e5", groupId: "group-personal", title: "9 de julio" },
      ],
    });
    await openGroup("Personal");

    const drawn = new Map(rowTitles().map((title, index) => [title, circles()[index]]));
    expect(drawn.get("(sin título)")).toBe("#:1");
    // A protected title never reaches the circle: not even its first letter.
    expect(drawn.get("(protegido)")).toBe("#:1");
    expect(drawn.get("—·—")).toBe("#:1");
    expect(drawn.get("  ñandú")).toBe("Ñ:1");
    expect(drawn.get("9 de julio")).toBe("9:1");
  });

  it("RF-42: usuario y sitio van cada uno en su columna, el sitio sin https://; si falta uno, su celda lleva una raya", async () => {
    await setup({
      groups: SAMPLE_GROUPS,
      entries: [
        { id: "e1", groupId: "group-personal", title: "Ambos", username: "ana", url: "https://uno.example/entrar" },
        { id: "e2", groupId: "group-personal", title: "Solo sitio", username: "", url: "http://dos.example" },
        { id: "e3", groupId: "group-personal", title: "Solo usuario", username: "ana.g", url: "" },
      ],
    });
    await openGroup("Personal");

    const cells = (title: string) => {
      const row = within(entriesTable()).getByRole("button", { name: title }).closest("tr");
      return [row?.querySelector(".cell-user")?.textContent, row?.querySelector(".cell-url")?.textContent];
    };
    expect(cells("Ambos")).toEqual(["ana", "uno.example/entrar"]);
    // Only `https://` is dropped: a site without encryption keeps its scheme in sight.
    expect(cells("Solo sitio")).toEqual(["—", "http://dos.example"]);
    // A cell without data draws a dash, and has no help: a double click on it does nothing (RF-106).
    expect(cells("Solo usuario")).toEqual(["ana.g", "—"]);
    const row = within(entriesTable()).getByRole("button", { name: "Solo usuario" }).closest("tr");
    expect(row?.querySelector(".cell-url")?.hasAttribute("data-empty")).toBe(true);
    expect(row?.querySelector(".cell-url")?.hasAttribute("title")).toBe(false);
  });

  it("RF-42: la fecha de la fila es corta, sin año cuando es el actual, y su `title` es la ayuda del doble clic", async () => {
    const year = new Date().getFullYear();
    await setup({
      groups: SAMPLE_GROUPS,
      entries: [
        { id: "e1", groupId: "group-personal", title: "Este año", modified: `${String(year)}-06-08T15:00:00Z` },
        { id: "e2", groupId: "group-personal", title: "Otro año", modified: `${String(year - 1)}-06-08T15:00:00Z` },
      ],
    });
    await openGroup("Personal");

    const day = (title: string) =>
      within(entriesTable()).getByRole("button", { name: title }).closest("tr")?.querySelector("time");
    expect(day("Este año")?.textContent).toBe("8 jun");
    expect(day("Otro año")?.textContent).toBe(`8 jun ${String(year - 1)}`);
    // The whole date, with the hour, is in the window of the entry; here the cell says what a double click does.
    expect(day("Este año")?.getAttribute("datetime")).toBe(`${String(year)}-06-08T15:00:00Z`);
    expect(day("Este año")?.closest("td")?.getAttribute("title")).toBe("Doble clic para abrir la entrada");
  });

  it("RF-59: al buscar la cabecera dice «Resultados» y sigue ofreciendo «Nueva entrada», que va al grupo que estaba abierto", async () => {
    const { backend } = await setup();
    await openGroup("Trabajo");

    await searchFor("git");
    const heading = screen.getByRole("heading", { name: "Resultados de la búsqueda" });
    // What is read is "Resultados": the rest of the name is only for assistive technology.
    const read = [...heading.childNodes].filter((node) => node.nodeType === Node.TEXT_NODE);
    expect(read.map((node) => node.textContent).join("").trim()).toBe("Resultados");
    expect(heading.querySelector(".sr-only")?.textContent).toBe("de la búsqueda");

    fireEvent.click(screen.getByRole("button", { name: "Nueva entrada" }));
    fireEvent.change(await screen.findByLabelText("Título"), { target: { value: "Gitea" } });
    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    await waitFor(() => {
      expect(backend.calls.filter((call) => call.command === "entry_create").at(-1)?.args).toMatchObject({
        group_id: "group-work",
        title: "Gitea",
      });
    });
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

describe("P4 Bóveda — la tabla y sus gestos", () => {
  /** Two entries of one group, one with every datum and one without user or site; the passwords differ in length. */
  const PASSWORDS = { full: "canario-corto-7Q", bare: "canario-bastante-mas-largo-que-el-otro-91xZ" } as const;
  const GESTURES: FakeOptions = {
    groups: SAMPLE_GROUPS,
    entries: [
      {
        id: "e-full",
        groupId: "group-personal",
        title: "Con todo",
        username: "ana",
        url: "https://uno.example/entrar",
        password: PASSWORDS.full,
        modified: "2026-06-08T15:00:00Z",
      },
      { id: "e-bare", groupId: "group-personal", title: "Sin nada", username: "", url: "", password: PASSWORDS.bare },
      { id: "e-safe", groupId: "group-personal", title: "Usuario protegido", username: null, url: "https://dos.example" },
    ],
  };

  function row(title: string): HTMLElement {
    const found = within(entriesTable()).getByRole("button", { name: title }).closest("tr");
    if (found === null) throw new Error(`missing row ${title}`);
    return found;
  }

  function cell(title: string, selector: string): HTMLElement {
    const found = row(title).querySelector<HTMLElement>(selector);
    if (found === null) throw new Error(`missing cell ${selector} of ${title}`);
    return found;
  }

  const titleButton = (title: string) => within(entriesTable()).getByRole("button", { name: title });

  /** A double click the way the browser sends it: two clicks and then the double click, on the same element. */
  function doubleClick(target: HTMLElement) {
    fireEvent.click(target);
    fireEvent.click(target);
    fireEvent.doubleClick(target);
  }

  function argsOf(backend: { calls: readonly { command: string; args: unknown }[] }, command: string): unknown[] {
    return backend.calls.filter((call) => call.command === command).map((call) => call.args);
  }

  /** The value is nowhere in the page: not in its markup, not in an attribute, not in a form field. */
  function expectAbsent(value: string) {
    expect(document.body.innerHTML).not.toContain(value);
    for (const element of document.body.querySelectorAll("*")) {
      for (const attribute of element.attributes) expect(attribute.value).not.toContain(value);
    }
    for (const field of document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea")) {
      expect(field.value).not.toContain(value);
    }
  }

  async function openGestures() {
    const result = await setup(GESTURES);
    await openGroup("Personal");
    return result;
  }

  it("RF-42: la tabla tiene sus rótulos a la vista y por fila título, usuario, una máscara fija, sitio y día; sin dato, una raya", async () => {
    await openGestures();

    const table = entriesTable();
    // A real table whose row of labels is seen: only the tags stay for the screen reader alone.
    expect(table.tagName).toBe("TABLE");
    expect(table.querySelector("thead")?.classList.contains("sr-only")).toBe(false);
    const labels = within(table).getAllByRole("columnheader");
    expect(labels.map((label) => label.textContent)).toEqual([
      "Título",
      "Usuario",
      "Contraseña",
      "Sitio",
      "Etiquetas",
      "Modificada",
      "Acciones",
    ]);
    expect(labels.every((label) => label.getAttribute("scope") === "col")).toBe(true);
    expect(labels.filter((label) => label.className === "cell-tags").map((label) => label.textContent)).toEqual([
      "Etiquetas",
    ]);

    expect(titleButton("Con todo").closest("th")?.getAttribute("scope")).toBe("row");
    expect(cell("Con todo", ".cell-user").textContent).toBe("ana");
    expect(cell("Con todo", ".cell-url").textContent).toBe("uno.example/entrar");
    expect(cell("Con todo", ".cell-date").textContent).toBe(`8 jun${new Date().getFullYear() === 2026 ? "" : " 2026"}`);
    // The mask is the same for every entry with a password, whatever it is: it does not give away the length.
    const masks = ["Con todo", "Sin nada"].map((title) => cell(title, ".cell-password").textContent);
    expect(new Set(masks).size).toBe(1);
    // RF-105: an entry without a password leaves the cell empty.
    expect(cell("Usuario protegido", ".cell-password").textContent).toBe("");
    expect(cell("Con todo", ".cell-password .mask").textContent).toBe("••••••••");
    // Without user or site: a dash. Protected in the file: "(protegido)", as before.
    expect(cell("Sin nada", ".cell-user").textContent).toBe("—");
    expect(cell("Sin nada", ".cell-url").textContent).toBe("—");
    expect(cell("Usuario protegido", ".cell-user").textContent).toBe("(protegido)");
    // Every row keeps the cell of the actions, empty until the row is chosen.
    expect(cell("Con todo", ".cell-actions").childElementCount).toBe(0);
  });

  it("RF-105: la lista no contiene ninguna contraseña del núcleo: ni en el DOM, ni en un atributo, ni tras elegir o copiar", async () => {
    const { backend } = await openGestures();
    // The fake core does hold them.
    expect(backend.entry("e-full")?.password).toBe(PASSWORDS.full);
    expect(backend.entry("e-bare")?.password).toBe(PASSWORDS.bare);

    const everywhere = () => {
      for (const password of Object.values(PASSWORDS)) expectAbsent(password);
    };
    everywhere();
    fireEvent.click(cell("Con todo", ".cell-password"));
    everywhere();
    doubleClick(cell("Sin nada", ".cell-password"));
    await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s");
    everywhere();
    fireEvent.click(await allButton());
    await screen.findByRole("heading", { name: "Todas las entradas" });
    await waitFor(() => {
      expect(rowTitles()).toContain("Con todo");
    });
    everywhere();
    await searchFor("canario");
    everywhere();
    // The list never asked for a value: only the copy went to the core, and its answer brings no value.
    expect(backend.count("entry_reveal")).toBe(0);
    expect(backend.count("entry_get")).toBe(0);
  });

  it("RF-106: un clic en cualquier celda elige la fila y nada más", async () => {
    const { backend } = await openGestures();

    for (const selector of [".cell-user", ".cell-password", ".cell-url", ".cell-date", ".cell-title button"]) {
      // Another row first, so each cell is seen to choose its own.
      fireEvent.click(titleButton("Sin nada"));
      expect(row("Con todo").hasAttribute("data-selected")).toBe(false);

      fireEvent.click(cell("Con todo", selector));

      expect(row("Con todo").hasAttribute("data-selected")).toBe(true);
      expect(titleButton("Con todo").getAttribute("aria-current")).toBe("true");
      expect(row("Sin nada").hasAttribute("data-selected")).toBe(false);
      // Only the chosen row shows its two buttons, in the cell it already had.
      expect(within(cell("Con todo", ".cell-actions")).getAllByRole("button").map((button) => button.getAttribute("aria-label"))).toEqual(
        ["Copiar contraseña de Con todo", "Abrir Con todo"],
      );
      expect(cell("Sin nada", ".cell-actions").childElementCount).toBe(0);
    }
    expect(screen.queryByRole("dialog")).toBeNull();
    for (const command of ["entry_get", "entry_copy", "entry_reveal", "entry_open_url"]) {
      expect(backend.count(command)).toBe(0);
    }
  });

  it("RF-106: doble clic en el título abre la ventana de la entrada", async () => {
    await openGestures();

    const title = titleButton("Con todo");
    expect(title.getAttribute("title")).toBe("Doble clic para abrir la entrada");
    doubleClick(title);

    const entryWindow = await screen.findByRole<HTMLDialogElement>("dialog", { name: "Con todo" });
    expect(entryWindow.open).toBe(true);
    expect(await within(entryWindow).findByRole("button", { name: "Editar" })).toBeDefined();
    // The first click chose the row without replacing its button: the double click landed on the same element.
    expect(titleButton("Con todo")).toBe(title);
    expect(title.getAttribute("aria-current")).toBe("true");
  });

  it("RF-106: doble clic en «Modificada» abre la ventana de la entrada", async () => {
    await openGestures();

    const day = cell("Con todo", ".cell-date");
    expect(day.getAttribute("title")).toBe("Doble clic para abrir la entrada");
    doubleClick(day);

    expect(await screen.findByRole("dialog", { name: "Con todo" })).toBeDefined();
    expect(cell("Con todo", ".cell-date")).toBe(day);
  });

  it("RF-106: doble clic en el usuario lo copia por el núcleo; uno protegido también, sin que su valor llegue a la lista", async () => {
    const { backend } = await openGestures();

    const user = cell("Con todo", ".cell-user");
    expect(user.getAttribute("title")).toBe("Doble clic para copiar el usuario");
    doubleClick(user);

    // Not protected: nothing clears it, so the notice has no countdown.
    expect(await screen.findByText("Usuario copiado")).toBeDefined();
    expect(argsOf(backend, "entry_copy")).toEqual([{ id: "e-full", field: "UserName" }]);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(cell("Con todo", ".cell-user")).toBe(user);

    doubleClick(cell("Usuario protegido", ".cell-user"));
    await waitFor(() => {
      expect(argsOf(backend, "entry_copy").at(-1)).toEqual({ id: "e-safe", field: "UserName" });
    });
    expect(cell("Usuario protegido", ".cell-user").textContent).toBe("(protegido)");
    expect(cell("Usuario protegido", ".cell-user").getAttribute("title")).toBe("Doble clic para copiar el usuario");
  });

  it("RF-106: doble clic en la contraseña la copia por el núcleo sin mostrarla: el valor no llega al DOM", async () => {
    const { backend } = await openGestures();

    const password = cell("Con todo", ".cell-password");
    expect(password.getAttribute("title")).toBe("Doble clic para copiar la contraseña");
    doubleClick(password);

    // The notice is the one of the list, with the countdown the core gave.
    const notice = await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s");
    expect(notice.className).toBe("toast");
    expect(notice.closest('[role="status"]')?.closest(".list-foot")).not.toBeNull();
    expect(argsOf(backend, "entry_copy")).toEqual([{ id: "e-full", field: "Password" }]);
    expect(backend.count("entry_reveal")).toBe(0);
    expect(backend.count("entry_get")).toBe(0);
    expectAbsent(PASSWORDS.full);
    expect(cell("Con todo", ".cell-password .mask").textContent).toBe("••••••••");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("RF-106: RF-49: doble clic en el sitio lo abre por el núcleo, sin enlace ni ventana del navegador propia", async () => {
    const { backend, container } = await openGestures();

    const site = cell("Con todo", ".cell-url");
    expect(site.getAttribute("title")).toBe("Doble clic para abrir el sitio en el navegador");
    doubleClick(site);

    expect(await screen.findByText("La dirección se abrió en el navegador.")).toBeDefined();
    expect(argsOf(backend, "entry_open_url")).toEqual([{ id: "e-full" }]);
    expect(container.querySelector("a")).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("RF-106: Enter con el foco en el título de la fila abre la ventana de la entrada", async () => {
    await openGestures();

    const title = titleButton("Con todo");
    title.focus();
    fireEvent.keyDown(title, { key: "Enter" });

    expect(await screen.findByRole("dialog", { name: "Con todo" })).toBeDefined();
    expect(title.getAttribute("aria-current")).toBe("true");
  });

  it("RF-106: en una celda sin dato el doble clic no hace nada, y no promete nada en su `title`", async () => {
    const { backend } = await openGestures();

    for (const selector of [".cell-user", ".cell-url"]) {
      const empty = cell("Sin nada", selector);
      expect(empty.hasAttribute("title")).toBe(false);
      doubleClick(empty);
    }
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(backend.count("entry_copy")).toBe(0);
    expect(backend.count("entry_open_url")).toBe(0);
    expect(screen.queryByRole("dialog")).toBeNull();
    // The clicks still chose the row.
    expect(row("Sin nada").hasAttribute("data-selected")).toBe(true);
  });

  it("RF-106: lo que hace el doble clic también lo hacen los dos botones de la fila elegida", async () => {
    const { backend } = await openGestures();
    fireEvent.click(cell("Con todo", ".cell-user"));

    const copy = screen.getByRole("button", { name: "Copiar contraseña de Con todo" });
    expect(copy.getAttribute("title")).toBe("Copiar contraseña");
    fireEvent.click(copy);
    expect(await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s")).toBeDefined();
    expect(argsOf(backend, "entry_copy")).toEqual([{ id: "e-full", field: "Password" }]);
    expectAbsent(PASSWORDS.full);

    fireEvent.click(screen.getByRole("button", { name: "Abrir Con todo" }));
    expect(await screen.findByRole("dialog", { name: "Con todo" })).toBeDefined();
  });

  it("RF-108: Esc cierra la ventana, devuelve el foco a la fila, que sigue elegida, y lo mostrado se oculta", async () => {
    await openGestures();
    doubleClick(titleButton("Con todo"));
    const entryWindow = await screen.findByRole("dialog", { name: "Con todo" });

    fireEvent.click(await within(entryWindow).findByRole("button", { name: "Mostrar contraseña" }));
    expect((await within(entryWindow).findByText(PASSWORDS.full)).className).toBe("secret");

    // Escape reaches a modal dialog as its `cancel` event.
    fireEvent(entryWindow, new Event("cancel", { cancelable: true }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expectAbsent(PASSWORDS.full);
    expect(titleButton("Con todo").getAttribute("aria-current")).toBe("true");
    expect(document.activeElement).toBe(titleButton("Con todo"));

    // Opened again, it starts hidden: nothing of what was shown came back.
    doubleClick(titleButton("Con todo"));
    await within(await screen.findByRole("dialog", { name: "Con todo" })).findByRole("button", { name: "Mostrar contraseña" });
    expectAbsent(PASSWORDS.full);
  });

  it("RF-74: RF-106: el aviso de copiado sale al pie de la lista y, con la ventana abierta, dentro de ella", async () => {
    await openGestures();
    const zones = () => [...document.querySelectorAll('.toast-zone[role="status"]')];
    // It exists before it has anything to say.
    expect(zones()).toHaveLength(1);
    expect(zones()[0]?.childElementCount).toBe(0);

    doubleClick(cell("Con todo", ".cell-user"));
    expect((await screen.findByText("Usuario copiado")).closest(".list-foot")).not.toBeNull();

    doubleClick(cell("Con todo", ".cell-date"));
    const entryWindow = await screen.findByRole("dialog", { name: "Con todo" });
    fireEvent.click(await within(entryWindow).findByRole("button", { name: "Copiar contraseña" }));
    const inside = await screen.findByText("Contraseña copiada. Se borra del portapapeles en 20 s");
    expect(entryWindow.contains(inside)).toBe(true);
    // One notice at a time: the one of the list is silent while the window is open.
    expect(screen.getAllByText(/copiad/)).toEqual([inside]);
  });
});
