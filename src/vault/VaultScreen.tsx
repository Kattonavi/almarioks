import { useEffect, useRef, useState } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { EntrySummary, GroupNode, SearchResult } from "../ipc/schemas";

/** RF-59: wait this long after the last keystroke before asking the core to search. */
export const SEARCH_DELAY_MS = 150;

type TreeState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly root: GroupNode }
  | { readonly kind: "failed"; readonly error: AppError };

type SearchState =
  | { readonly kind: "idle" }
  | { readonly kind: "searching" }
  | { readonly kind: "done"; readonly items: readonly SearchResult[] }
  | { readonly kind: "failed"; readonly error: AppError };

interface ListState {
  readonly groupId: string;
  readonly attempt: number;
  readonly result: Result<EntrySummary[]>;
}

interface SelectedEntry {
  readonly id: string;
  readonly title: string | null;
}

interface VaultScreenProps {
  readonly client: Pick<VaultClient, "groupTree" | "entryList" | "search">;
}

function findGroup(node: GroupNode, id: string): GroupNode | undefined {
  if (node.id === id) return node;
  for (const child of node.children) {
    const found = findGroup(child, id);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** Entries outside the recycle bin; `entry_count` is per group, so the sum works whether it is direct or not. */
function liveEntryCount(node: GroupNode): number {
  if (node.kind === "recycle_bin") return 0;
  return node.entryCount + node.children.reduce((sum, child) => sum + liveEntryCount(child), 0);
}

function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

/** A protected standard field arrives as `null` and is shown as "(protegido)", never asked for. */
function shown(value: string | null, empty = ""): string {
  if (value === null) return "(protegido)";
  return value === "" ? empty : value;
}

const DATE_FORMAT = new Intl.DateTimeFormat("es-CO", { dateStyle: "medium", timeStyle: "short" });

function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : DATE_FORMAT.format(date);
}

/**
 * P4: the open vault (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92). Three zones: groups, entries of the chosen group or
 * search results, and the detail. Only metadata: no protected value is asked for or shown. Every vault datum is
 * rendered as a React text child, never as HTML or as a link (RF-78). All its data is local state, so a lock, which
 * unmounts this screen, drops it at once (RF-32).
 */
export function VaultScreen({ client }: VaultScreenProps) {
  const [tree, setTree] = useState<TreeState>({ kind: "loading" });
  const [treeAttempt, setTreeAttempt] = useState(0);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [list, setList] = useState<ListState | null>(null);
  const [listAttempt, setListAttempt] = useState(0);
  const [selectedEntry, setSelectedEntry] = useState<SelectedEntry | null>(null);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<SearchState>({ kind: "idle" });
  const searchRef = useRef<HTMLInputElement>(null);
  const searchSeq = useRef(0);
  const searchTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    const timer = searchTimer;
    const seq = searchSeq;
    return () => {
      mounted.current = false;
      clearTimeout(timer.current);
      seq.current += 1;
    };
  }, []);

  useEffect(() => {
    let active = true;
    void client.groupTree().then((result) => {
      if (!active) return;
      setTree(result.ok ? { kind: "ready", root: result.value } : { kind: "failed", error: result.error });
    });
    return () => {
      active = false;
    };
  }, [client, treeAttempt]);

  const groupId = tree.kind === "ready" ? (selectedGroupId ?? tree.root.id) : null;

  useEffect(() => {
    if (groupId === null) return;
    let active = true;
    void client.entryList(groupId).then((result) => {
      if (active) setList({ groupId, attempt: listAttempt, result });
    });
    return () => {
      active = false;
    };
  }, [client, groupId, listAttempt]);

  // Ctrl+F or Ctrl+K go to the search field from anywhere in the vault.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key !== "f" && key !== "k") return;
      event.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  function cancelSearch() {
    clearTimeout(searchTimer.current);
    searchSeq.current += 1;
  }

  function onQueryChange(value: string) {
    setQuery(value);
    cancelSearch();
    // RF-92: an empty query (after trimming) does not filter: back to the current group.
    if (value.trim() === "") {
      setSearch({ kind: "idle" });
      return;
    }
    const mine = searchSeq.current;
    setSearch({ kind: "searching" });
    searchTimer.current = setTimeout(() => {
      void client.search(value).then((result) => {
        if (!mounted.current || mine !== searchSeq.current) return;
        setSearch(result.ok ? { kind: "done", items: result.value } : { kind: "failed", error: result.error });
      });
    }, SEARCH_DELAY_MS);
  }

  function clearSearch() {
    onQueryChange("");
    searchRef.current?.focus();
  }

  function selectGroup(id: string) {
    setSelectedGroupId(id);
    setSelectedEntry(null);
  }

  const searching = query.trim() !== "";

  return (
    <div className="vault">
      <div role="search" className="field">
        <label htmlFor="vault-search">Buscar en la bóveda</label>
        <input
          ref={searchRef}
          id="vault-search"
          type="search"
          value={query}
          autoComplete="off"
          spellCheck={false}
          aria-keyshortcuts="Control+F Control+K"
          aria-describedby="vault-search-help"
          onChange={(event) => {
            onQueryChange(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape" && query !== "") {
              event.preventDefault();
              clearSearch();
            }
          }}
        />
        <p id="vault-search-help" className="path">
          Atajo: Ctrl+F o Ctrl+K. Escape limpia la búsqueda.
        </p>
      </div>

      <div className="vault-zones">
        <nav aria-labelledby="groups-title" className="vault-zone">
          <h3 id="groups-title">Grupos</h3>
          {renderTree()}
        </nav>

        <section aria-labelledby="entries-title" className="vault-zone">
          {searching ? renderSearch() : renderGroup()}
        </section>

        <section aria-labelledby="detail-title" className="vault-zone">
          <h3 id="detail-title">Detalle</h3>
          {selectedEntry === null ? (
            <p>Elige una entrada para ver su detalle.</p>
          ) : (
            <>
              <p>{shown(selectedEntry.title, "(sin título)")}</p>
              <p className="path">El detalle completo de la entrada llega en la tarea siguiente.</p>
            </>
          )}
        </section>
      </div>
    </div>
  );

  function renderTree() {
    switch (tree.kind) {
      case "loading":
        return <p role="status">Cargando los grupos…</p>;
      case "failed":
        return (
          <>
            <p role="alert" className="error">
              {errorMessage(tree.error)}
            </p>
            <button
              type="button"
              onClick={() => {
                setTree({ kind: "loading" });
                setTreeAttempt((value) => value + 1);
              }}
            >
              Reintentar
            </button>
          </>
        );
      case "ready":
        return <GroupList nodes={[tree.root]} selectedId={groupId} onSelect={selectGroup} />;
      default:
        return assertNever(tree);
    }
  }

  function renderGroup() {
    if (tree.kind !== "ready" || groupId === null) {
      return <h3 id="entries-title">Entradas</h3>;
    }
    const group = findGroup(tree.root, groupId) ?? tree.root;
    const loaded = list !== null && list.groupId === groupId && list.attempt === listAttempt ? list.result : null;
    return (
      <>
        <h3 id="entries-title">Entradas de {shown(group.name, "(sin nombre)")}</h3>
        <div className="actions">
          <button type="button" aria-disabled="true" aria-describedby="new-entry-soon">
            Nueva entrada
          </button>
          <span id="new-entry-soon" className="path">
            Próximamente
          </span>
        </div>
        {loaded === null ? (
          <p role="status">Cargando las entradas…</p>
        ) : !loaded.ok ? (
          <>
            <p role="alert" className="error">
              {errorMessage(loaded.error)}
            </p>
            <button
              type="button"
              onClick={() => {
                setListAttempt((value) => value + 1);
              }}
            >
              Reintentar
            </button>
          </>
        ) : loaded.value.length === 0 ? (
          liveEntryCount(tree.root) === 0 && group.kind !== "recycle_bin" ? (
            <p>
              Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardarás la primera cuenta; llega en la
              tarea siguiente.
            </p>
          ) : (
            <p>{group.kind === "recycle_bin" ? "La papelera está vacía." : "Este grupo no tiene entradas."}</p>
          )
        ) : (
          <>
            <p role="status">{plural(loaded.value.length, "entrada", "entradas")}</p>
            <EntryTable
              items={loaded.value}
              groupPaths={null}
              selectedId={selectedEntry?.id ?? null}
              onSelect={setSelectedEntry}
            />
          </>
        )}
      </>
    );
  }

  function renderSearch() {
    return (
      <>
        <h3 id="entries-title">Resultados de la búsqueda</h3>
        <div role="status" aria-live="polite">
          {search.kind === "searching" && <p>Buscando…</p>}
          {search.kind === "done" && (
            <p>
              {search.items.length === 0
                ? "Sin resultados."
                : plural(search.items.length, "resultado", "resultados")}
            </p>
          )}
        </div>
        {search.kind === "failed" && (
          <p role="alert" className="error">
            {errorMessage(search.error)}
          </p>
        )}
        {search.kind === "done" && search.items.length === 0 && (
          <>
            <p>
              No hay entradas que coincidan con «{query.trim()}». Prueba con menos palabras o con otra parte del
              título, el usuario, la URL, las etiquetas o las notas.
            </p>
            <button type="button" onClick={clearSearch}>
              Limpiar búsqueda
            </button>
          </>
        )}
        {search.kind === "done" && search.items.length > 0 && (
          <EntryTable
            items={search.items}
            groupPaths={search.items.map((item) => item.groupPath)}
            selectedId={selectedEntry?.id ?? null}
            onSelect={setSelectedEntry}
          />
        )}
      </>
    );
  }
}

interface GroupListProps {
  readonly nodes: readonly GroupNode[];
  readonly selectedId: string | null;
  readonly onSelect: (id: string) => void;
}

/** RF-50: nested lists of buttons; Tab moves, Enter or Space opens the group, `aria-current` marks the open one. */
function GroupList({ nodes, selectedId, onSelect }: GroupListProps) {
  return (
    <ul className="groups">
      {nodes.map((node) => (
        <li key={node.id}>
          <button
            type="button"
            aria-current={node.id === selectedId ? "true" : undefined}
            onClick={() => {
              onSelect(node.id);
            }}
          >
            {/* One text node, so the accessible name keeps its spaces. */}
            {`${shown(node.name, "(sin nombre)")}${node.kind === "recycle_bin" ? " (papelera)" : ""} · ${plural(
              node.entryCount,
              "entrada",
              "entradas",
            )}`}
          </button>
          {node.children.length > 0 && <GroupList nodes={node.children} selectedId={selectedId} onSelect={onSelect} />}
        </li>
      ))}
    </ul>
  );
}

interface EntryTableProps {
  readonly items: readonly EntrySummary[];
  /** Search results: the group path of each item, from the root down. */
  readonly groupPaths: readonly (readonly string[])[] | null;
  readonly selectedId: string | null;
  readonly onSelect: (entry: SelectedEntry) => void;
}

/** RF-42: entries in the order the core sends (by title). The URL is text, never a link (RF-78, RF-49). */
function EntryTable({ items, groupPaths, selectedId, onSelect }: EntryTableProps) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th scope="col">Título</th>
            <th scope="col">Usuario</th>
            <th scope="col">URL</th>
            <th scope="col">Etiquetas</th>
            {groupPaths !== null && <th scope="col">Grupo</th>}
            <th scope="col">Modificada</th>
          </tr>
        </thead>
        <tbody>
          {items.map((entry, index) => (
            <tr key={entry.id}>
              <th scope="row">
                <button
                  type="button"
                  aria-current={entry.id === selectedId ? "true" : undefined}
                  onClick={() => {
                    onSelect({ id: entry.id, title: entry.title });
                  }}
                >
                  {shown(entry.title, "(sin título)")}
                </button>
              </th>
              <td>{shown(entry.username)}</td>
              <td className="path">{shown(entry.url)}</td>
              <td>{entry.tags.join(", ")}</td>
              {groupPaths !== null && <td>{(groupPaths[index] ?? []).join(" › ")}</td>}
              <td>
                <time dateTime={entry.modified}>{formatDate(entry.modified)}</time>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
