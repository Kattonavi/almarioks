import { useEffect, useRef, useState } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { EntrySummary, GroupNode, SearchResult } from "../ipc/schemas";
import { EntryForm } from "./EntryForm";
import { EntryPanel } from "./EntryPanel";
import { findGroup, formatDate, liveEntryCount, plural, shown } from "./format";
import { GroupActions, type GroupChange } from "./GroupActions";

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

/** What the detail zone shows. */
type DetailState =
  | { readonly kind: "none" }
  | { readonly kind: "entry"; readonly entry: SelectedEntry }
  | { readonly kind: "new"; readonly groupId: string };

interface VaultScreenProps {
  readonly client: VaultClient;
  /** RF-14: nothing that changes the vault is offered; the notice above the screen says why. */
  readonly readOnly?: boolean;
}

/**
 * P4: the open vault (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92), with P5 in the detail zone. Three zones: groups,
 * entries of the chosen group or search results, and the detail. Every vault datum is rendered as a React text child,
 * never as HTML or as a link (RF-78). All its data is local state, so a lock, which unmounts this screen, drops it at
 * once (RF-32).
 */
export function VaultScreen({ client, readOnly = false }: VaultScreenProps) {
  const [tree, setTree] = useState<TreeState>({ kind: "loading" });
  const [treeAttempt, setTreeAttempt] = useState(0);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [list, setList] = useState<ListState | null>(null);
  const [listAttempt, setListAttempt] = useState(0);
  /** Asks the list again after a change without showing "Cargando" in between. */
  const [listRefresh, setListRefresh] = useState(0);
  const [detail, setDetail] = useState<DetailState>({ kind: "none" });
  const [notice, setNotice] = useState("");
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<SearchState>({ kind: "idle" });
  const searchRef = useRef<HTMLInputElement>(null);
  const detailTitleRef = useRef<HTMLHeadingElement>(null);
  const groupsTitleRef = useRef<HTMLHeadingElement>(null);
  /** After a group action removed what had focus, focus goes back to the tree once it is painted. */
  const focusTree = useRef(false);
  const focusDetail = useRef(false);
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

  useEffect(() => {
    if (!focusDetail.current) return;
    focusDetail.current = false;
    detailTitleRef.current?.focus();
  }, [detail]);

  useEffect(() => {
    if (!focusTree.current) return;
    focusTree.current = false;
    groupsTitleRef.current?.focus();
  }, [tree]);

  const selectedExists =tree.kind === "ready" && selectedGroupId !== null && findGroup(tree.root, selectedGroupId);
  const groupId = tree.kind === "ready" ? (selectedExists ? selectedGroupId : tree.root.id) : null;

  useEffect(() => {
    if (groupId === null) return;
    let active = true;
    void client.entryList(groupId).then((result) => {
      if (active) setList({ groupId, attempt: listAttempt, result });
    });
    return () => {
      active = false;
    };
  }, [client, groupId, listAttempt, listRefresh]);

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
    setDetail({ kind: "none" });
  }

  /** After a change: the tree (counts), the list and an active search, without unmounting the detail. */
  function refresh() {
    setTreeAttempt((value) => value + 1);
    setListRefresh((value) => value + 1);
    if (query.trim() !== "") onQueryChange(query);
  }

  /** P7: the core answered with the new tree. */
  function onGroupChange(change: GroupChange) {
    setTree({ kind: "ready", root: change.root });
    setSelectedGroupId(change.select);
    setListRefresh((value) => value + 1);
    if (query.trim() !== "") onQueryChange(query);
    if (change.removed) {
      setDetail({ kind: "none" });
      focusTree.current = true;
    }
  }

  function selectEntry(entry: SelectedEntry) {
    setNotice("");
    setDetail({ kind: "entry", entry });
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
          <h3 id="groups-title" ref={groupsTitleRef} tabIndex={-1}>
            Grupos
          </h3>
          {renderTree()}
        </nav>

        <section aria-labelledby="entries-title" className="vault-zone">
          {searching ? renderSearch() : renderGroup()}
        </section>

        <section aria-labelledby="detail-title" className="vault-zone">
          <h3 id="detail-title" ref={detailTitleRef} tabIndex={-1}>
            Detalle
          </h3>
          <div aria-live="polite">{notice !== "" && <p className="notice">{notice}</p>}</div>
          {renderDetail()}
        </section>
      </div>
    </div>
  );

  function renderDetail() {
    switch (detail.kind) {
      case "none":
        return <p>Elige una entrada para ver su detalle.</p>;
      case "new": {
        if (tree.kind !== "ready") return null;
        const group = findGroup(tree.root, detail.groupId) ?? tree.root;
        return (
          <EntryForm
            client={client}
            mode={{ kind: "create", groupId: group.id, groupName: shown(group.name, "(sin nombre)") }}
            onSaved={(entry) => {
              setNotice("Entrada guardada.");
              setDetail({ kind: "entry", entry: { id: entry.id, title: entry.title } });
              refresh();
            }}
            onCancel={() => {
              setDetail({ kind: "none" });
              detailTitleRef.current?.focus();
            }}
          />
        );
      }
      case "entry":
        if (tree.kind !== "ready") return null;
        return (
          <EntryPanel
            key={detail.entry.id}
            client={client}
            entryId={detail.entry.id}
            listTitle={detail.entry.title}
            root={tree.root}
            readOnly={readOnly}
            onChanged={refresh}
            onDeleted={(message) => {
              setDetail({ kind: "none" });
              setNotice(message);
              refresh();
              // After the commit: while a modal dialog is open, everything outside it is inert.
              focusDetail.current = true;
            }}
          />
        );
      default:
        return assertNever(detail);
    }
  }

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
      case "ready": {
        const group = groupId === null ? undefined : findGroup(tree.root, groupId);
        return (
          <>
            <GroupList nodes={[tree.root]} selectedId={groupId} onSelect={selectGroup} />
            {readOnly ? (
              <p className="path">Solo lectura: los grupos no se pueden crear, renombrar, mover ni borrar.</p>
            ) : group !== undefined && (
              <GroupActions client={client} root={tree.root} group={group} onChange={onGroupChange} />
            )}
          </>
        );
      }
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
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
    return (
      <>
        <h3 id="entries-title">Entradas de {shown(group.name, "(sin nombre)")}</h3>
        {!readOnly && (
          <div className="actions">
            <button
              type="button"
              onClick={() => {
                setNotice("");
                setDetail({ kind: "new", groupId: group.id });
              }}
            >
              Nueva entrada
            </button>
          </div>
        )}
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
            <p>Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardas la primera cuenta.</p>
          ) : (
            <p>{group.kind === "recycle_bin" ? "La papelera está vacía." : "Este grupo no tiene entradas."}</p>
          )
        ) : (
          <>
            <p role="status">{plural(loaded.value.length, "entrada", "entradas")}</p>
            <EntryTable items={loaded.value} groupPaths={null} selectedId={selectedId} onSelect={selectEntry} />
          </>
        )}
      </>
    );
  }

  function renderSearch() {
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
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
            selectedId={selectedId}
            onSelect={selectEntry}
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
