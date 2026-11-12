import { useEffect, useRef, useState, type ReactNode } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { EntrySummary, GroupNode, SearchResult } from "../ipc/schemas";
import { PlusIcon, SearchIcon, TrashIcon } from "../ui/icons";
import { EntryForm } from "./EntryForm";
import { EntryPanel } from "./EntryPanel";
import { findGroup, formatDate, formatDay, liveEntryCount, plural, shown } from "./format";
import { GroupActions, type GroupChange } from "./GroupActions";
import { assignLines, initialOf, lineOfPath, type GroupLine } from "./lines";

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

/** What a group badge shows: the line colour of the group and its initial (the recycle bin shows its icon). */
interface Badge {
  readonly line: GroupLine;
  readonly initial: string;
  readonly bin: boolean;
}

/** The row whose value is on the clipboard, and the seconds left until the core clears it. */
interface Stamped {
  readonly entryId: string;
  readonly seconds: number;
}

const NO_LINES: ReadonlyMap<string, GroupLine> = new Map();

interface VaultScreenProps {
  readonly client: VaultClient;
  /** RF-14: nothing that changes the vault is offered; the notice above the screen says why. */
  readonly readOnly?: boolean;
  /** Frame slots, filled by whoever owns the open vault: the end of the top bar (lock, settings)… */
  readonly barEnd?: ReactNode;
  /** …the head of the group rail (name and file of the vault)… */
  readonly railHead?: ReactNode;
  /** …and the notices above the work area (read-only, save problems). */
  readonly banner?: ReactNode;
}

/**
 * P4: the open vault (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92), with P5 in the detail zone. A signal frame (top bar
 * with the search, rail with the groups) around two work zones: entries of the chosen group or search results, and
 * the detail. Every vault datum is rendered as a React text child, never as HTML or as a link (RF-78). All its data
 * is local state, so a lock, which unmounts this screen, drops it at once (RF-32).
 */
export function VaultScreen({ client, readOnly = false, barEnd, railHead, banner }: VaultScreenProps) {
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
  /** RF-74 on the list: which row holds the clipboard and until when. It outlives the detail of that entry. */
  const [stamp, setStamp] = useState<{ readonly entryId: string; readonly deadline: number } | null>(null);
  const [now, setNow] = useState(() => Date.now());
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

  // The stamp on the copied row counts down with the clipboard and goes away when the core clears it.
  useEffect(() => {
    if (stamp === null) return;
    const timer = setInterval(() => {
      const at = Date.now();
      if (at >= stamp.deadline) setStamp(null);
      else setNow(at);
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  }, [stamp]);

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
  const stamped: Stamped | null =
    stamp === null ? null : { entryId: stamp.entryId, seconds: Math.max(1, Math.ceil((stamp.deadline - now) / 1000)) };
  /** The line colour of every group: by position, so it only changes when the tree does. */
  const lines = tree.kind === "ready" ? assignLines(tree.root) : NO_LINES;

  return (
    <div className="vault">
      <div className="vault-bar">
        <div role="search" className="vault-search">
          <label htmlFor="vault-search" className="sr-only">
            Buscar en la bóveda
          </label>
          <SearchIcon />
          <input
            ref={searchRef}
            id="vault-search"
            type="search"
            value={query}
            placeholder="Buscar en la bóveda"
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
          <kbd aria-hidden="true">Ctrl K</kbd>
          <p id="vault-search-help" className="sr-only">
            Atajo: Ctrl+F o Ctrl+K. Escape limpia la búsqueda.
          </p>
        </div>
        {/* The lock countdown belongs here once the core reports the time left: today no status carries it. */}
        <div className="vault-bar-end">{barEnd}</div>
      </div>

      <div className="rail">
        {railHead !== undefined && <div className="rail-head">{railHead}</div>}
        <nav aria-labelledby="groups-title" className="rail-nav">
          <h3 id="groups-title" ref={groupsTitleRef} tabIndex={-1}>
            Grupos
          </h3>
          {renderTree()}
        </nav>
      </div>

      <div className="work" data-detail={detail.kind}>
        <div className="work-banner">{banner}</div>
        <div className="vault-zones">
          <section aria-labelledby="entries-title" className="list-pane">
            {searching ? renderSearch() : renderGroup()}
          </section>

          <section aria-labelledby="detail-title" className="detail-pane">
            {/* The focus target after an entry leaves; the entry title is the visible heading of this zone. */}
            <h3 id="detail-title" ref={detailTitleRef} tabIndex={-1} className="sr-only">
              Detalle
            </h3>
            <div aria-live="polite">{notice !== "" && <p className="notice">{notice}</p>}</div>
            {renderDetail()}
          </section>
        </div>
      </div>
    </div>
  );

  function renderDetail() {
    switch (detail.kind) {
      case "none":
        return <p className="pane-state">Elige una entrada para ver su detalle.</p>;
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
            onCopied={(deadline) => {
              setNow(Date.now());
              setStamp(deadline === null ? null : { entryId: detail.entry.id, deadline });
            }}
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
        return (
          <p role="status" className="pane-state">
            Cargando los grupos…
          </p>
        );
      case "failed":
        return (
          <div className="pane-state">
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
          </div>
        );
      case "ready": {
        const group = groupId === null ? undefined : findGroup(tree.root, groupId);
        return (
          <>
            <GroupList nodes={[tree.root]} selectedId={groupId} lines={lines} depth={0} onSelect={selectGroup} />
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
      return (
        <header className="list-head">
          <div className="list-title">
            <h3 id="entries-title">Entradas</h3>
          </div>
        </header>
      );
    }
    const group = findGroup(tree.root, groupId) ?? tree.root;
    const name = shown(group.name, "(sin nombre)");
    const badge: Badge = {
      line: lines.get(group.id) ?? "neutral",
      initial: initialOf(name),
      bin: group.kind === "recycle_bin",
    };
    const loaded = list !== null && list.groupId === groupId && list.attempt === listAttempt ? list.result : null;
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
    return (
      <>
        <header className="list-head">
          <Bullet badge={badge} />
          <div className="list-title">
            <h3 id="entries-title">
              {/* The space is its own text node: inside the hidden span it would be trimmed out of the name. */}
              <span className="sr-only">Entradas de</span> {name}
            </h3>
            {loaded !== null && loaded.ok && loaded.value.length > 0 && (
              <p role="status">{plural(loaded.value.length, "entrada", "entradas")}</p>
            )}
          </div>
          {!readOnly && (
            <button
              type="button"
              className="btn-primary"
              onClick={() => {
                setNotice("");
                setDetail({ kind: "new", groupId: group.id });
              }}
            >
              <PlusIcon />
              Nueva entrada
            </button>
          )}
        </header>
        {loaded === null ? (
          <p role="status" className="pane-state">
            Cargando las entradas…
          </p>
        ) : !loaded.ok ? (
          <div className="pane-state">
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
          </div>
        ) : loaded.value.length === 0 ? (
          liveEntryCount(tree.root) === 0 && group.kind !== "recycle_bin" ? (
            <p className="pane-state">
              Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardas la primera cuenta.
            </p>
          ) : (
            <p className="pane-state">
              {group.kind === "recycle_bin" ? "La papelera está vacía." : "Este grupo no tiene entradas."}
            </p>
          )
        ) : (
          <EntryTable
            items={loaded.value}
            groupPaths={null}
            badgeOf={() => badge}
            stamped={stamped}
            selectedId={selectedId}
            onSelect={selectEntry}
          />
        )}
      </>
    );
  }

  function renderSearch() {
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
    const root = tree.kind === "ready" ? tree.root : null;
    return (
      <>
        <header className="list-head">
          <span className="bullet" data-line="station" aria-hidden="true">
            <SearchIcon />
          </span>
          <div className="list-title">
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
          </div>
        </header>
        {search.kind === "failed" && (
          <p role="alert" className="error pane-state">
            {errorMessage(search.error)}
          </p>
        )}
        {search.kind === "done" && search.items.length === 0 && (
          <div className="pane-state">
            <p>
              No hay entradas que coincidan con «{query.trim()}». Prueba con menos palabras o con otra parte del
              título, el usuario, la URL, las etiquetas o las notas.
            </p>
            <button type="button" onClick={clearSearch}>
              Limpiar búsqueda
            </button>
          </div>
        )}
        {search.kind === "done" && search.items.length > 0 && (
          <EntryTable
            items={search.items}
            groupPaths={search.items.map((item) => item.groupPath)}
            badgeOf={(index) => {
              const path = search.items[index]?.groupPath ?? [];
              return {
                line: root === null ? "neutral" : lineOfPath(root, lines, path),
                initial: initialOf(path.at(-1) ?? ""),
                bin: false,
              };
            }}
            stamped={stamped}
            selectedId={selectedId}
            onSelect={selectEntry}
          />
        )}
      </>
    );
  }
}

/**
 * The round badge of a group: decoration next to the group name, hidden from assistive technology. The initial is
 * drawn by CSS from `data-initial`, so the badge adds no text to the control it sits in.
 */
function Bullet({ badge, sub = false }: { readonly badge: Badge; readonly sub?: boolean }) {
  return (
    <span
      className="bullet"
      data-line={badge.line}
      data-initial={badge.bin ? undefined : badge.initial}
      data-sub={sub ? "" : undefined}
      aria-hidden="true"
    >
      {badge.bin && <TrashIcon />}
    </span>
  );
}

interface GroupListProps {
  readonly nodes: readonly GroupNode[];
  readonly selectedId: string | null;
  readonly lines: ReadonlyMap<string, GroupLine>;
  /** 0: the root group; 1: the groups that own a line; deeper: stops on that line. */
  readonly depth: number;
  readonly onSelect: (id: string) => void;
}

/** RF-50: nested lists of buttons; Tab moves, Enter or Space opens the group, `aria-current` marks the open one. */
function GroupList({ nodes, selectedId, lines, depth, onSelect }: GroupListProps) {
  return (
    <ul className="groups">
      {nodes.map((node) => {
        const name = shown(node.name, "(sin nombre)");
        const bin = node.kind === "recycle_bin";
        return (
          <li key={node.id}>
            <button
              type="button"
              className="rail-item"
              title={name}
              aria-current={node.id === selectedId ? "true" : undefined}
              onClick={() => {
                onSelect(node.id);
              }}
            >
              <Bullet
                badge={{ line: lines.get(node.id) ?? "neutral", initial: initialOf(name), bin }}
                sub={depth > 1}
              />
              {/*
                The text of the button stays "Nombre · N entradas" (its accessible name): the parts that are not
                drawn are hidden, and every space is its own text node so it is not trimmed out of the name.
              */}
              <span className="rail-name">{name}</span>
              {bin && " "}
              {bin && <span className="sr-only">(papelera)</span>} <span className="sr-only">·</span>{" "}
              <span className="rail-count">{node.entryCount}</span>{" "}
              <span className="sr-only">{node.entryCount === 1 ? "entrada" : "entradas"}</span>
            </button>
            {node.children.length > 0 && (
              <GroupList
                nodes={node.children}
                selectedId={selectedId}
                lines={lines}
                depth={depth + 1}
                onSelect={onSelect}
              />
            )}
          </li>
        );
      })}
    </ul>
  );
}

interface EntryTableProps {
  readonly items: readonly EntrySummary[];
  /** Search results: the group path of each item, from the root down. */
  readonly groupPaths: readonly (readonly string[])[] | null;
  /** The badge of the group each row sits in. */
  readonly badgeOf: (index: number) => Badge;
  readonly stamped: Stamped | null;
  readonly selectedId: string | null;
  readonly onSelect: (entry: SelectedEntry) => void;
}

/**
 * RF-42: entries in the order the core sends (by title). The URL is text, never a link (RF-78, RF-49). It is a real
 * table drawn as a list of rows: the title button stretches over its row, and the header row stays for screen readers.
 */
function EntryTable({ items, groupPaths, badgeOf, stamped, selectedId, onSelect }: EntryTableProps) {
  return (
    <div className="table-scroll">
      <table className="entries" data-search={groupPaths === null ? undefined : ""}>
        <thead className="sr-only">
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
            <tr key={entry.id} data-selected={entry.id === selectedId ? "" : undefined}>
              <th scope="row">
                <Bullet badge={badgeOf(index)} />
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
              <td className="cell-user">{shown(entry.username)}</td>
              <td className="cell-url">{shown(entry.url)}</td>
              <td className="cell-tags">{entry.tags.join(", ")}</td>
              {groupPaths !== null && <td className="cell-group">{(groupPaths[index] ?? []).join(" › ")}</td>}
              <td className="cell-date">
                <time
                  dateTime={entry.modified}
                  title={formatDate(entry.modified)}
                  className={stamped?.entryId === entry.id ? "sr-only" : undefined}
                >
                  {formatDay(entry.modified)}
                </time>
                {stamped?.entryId === entry.id && <span className="stamp">Copiado · {stamped.seconds} s</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
