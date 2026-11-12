import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";

import { assertNever } from "../assertNever";
import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { EntrySummary, GroupNode, SearchResult } from "../ipc/schemas";
import { ListIcon, PlusIcon, SearchIcon, TrashIcon } from "../ui/icons";
import { EntryForm } from "./EntryForm";
import { EntryPanel } from "./EntryPanel";
import { findGroup, formatDate, formatDay, liveEntryCount, plural, shown, trailTo } from "./format";
import { useGroupActions, type GroupChange } from "./GroupActions";
import { GroupMenuButton, GroupRail } from "./GroupRail";
import { assignLines, initialOf, lineOfPath, type GroupLine } from "./lines";
import { NO_SAVE_SIGNAL, savingText, type SaveSignal } from "./saveSignal";

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
  /** The groups it was asked for (see `listKey`). */
  readonly key: string;
  readonly attempt: number;
  readonly result: Result<EntrySummary[]>;
  /** The group each entry came from: in "Todas las entradas" every row keeps the colour of its own group. */
  readonly groupOf: ReadonlyMap<string, string>;
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

/** The notice of the detail zone: a plain text, or a new entry waiting for its save to reach the disk (RF-21). */
type Notice = { readonly kind: "text"; readonly text: string } | { readonly kind: "created"; readonly since: number };

const NO_NOTICE: Notice = { kind: "text", text: "" };

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

/** Between the group ids of a list key. An id never has a line break in it. */
const KEY_SEPARATOR = "\n";

/** RF-102: every group outside the recycle bin, the root first. */
function liveGroupIds(node: GroupNode): string[] {
  if (node.kind === "recycle_bin") return [];
  return [node.id, ...node.children.flatMap(liveGroupIds)];
}

/**
 * A title the way the core compares it (`text::fold` of the core): lower case, NFKD, without combining marks, and
 * twice, because a lower case can bring a mark of its own.
 */
function foldTitle(title: string | null): string {
  const once = (text: string) => text.toLowerCase().normalize("NFKD").replace(/\p{M}/gu, "");
  return once(once(title ?? ""));
}

/** RF-102: several groups put together go by title, like the entries of one group; the id settles equal titles. */
function byTitle(a: EntrySummary, b: EntrySummary): number {
  const ta = foldTitle(a.title);
  const tb = foldTitle(b.title);
  if (ta !== tb) return ta < tb ? -1 : 1;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/** One list out of the lists of `ids`: the first failure, or every entry with the group it came from. */
function mergeLists(
  ids: readonly string[],
  results: readonly Result<EntrySummary[]>[],
): Pick<ListState, "result" | "groupOf"> {
  const groupOf = new Map<string, string>();
  const entries: EntrySummary[] = [];
  for (const [index, result] of results.entries()) {
    if (!result.ok) return { result, groupOf };
    for (const entry of result.value) {
      groupOf.set(entry.id, ids[index] ?? "");
      entries.push(entry);
    }
  }
  // One group arrives in the order of the core; only several put together are ordered here.
  if (ids.length > 1) entries.sort(byTitle);
  return { result: { ok: true, value: entries }, groupOf };
}

interface VaultScreenProps {
  readonly client: VaultClient;
  /** RF-14: nothing that changes the vault is offered; the notice above the screen says why. */
  readonly readOnly?: boolean;
  /** RF-21: saves that reached the disk, so "guardada" is said only when it is true. */
  readonly saveSignal?: SaveSignal;
  /** Frame slots, filled by whoever owns the open vault: the end of the top bar (lock, settings)… */
  readonly barEnd?: ReactNode;
  /** …the head of the group rail (the name of the vault)… */
  readonly railHead?: ReactNode;
  /** …and the notices above the work area (read-only, save problems). */
  readonly banner?: ReactNode;
}

/**
 * P4: the open vault (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92, RF-102), with P5 in the detail zone. A signal frame
 * (top bar with the search, rail with the groups) around two work zones: entries of what the rail has open or search
 * results, and the detail. Every vault datum is rendered as a React text child, never as HTML or as a link (RF-78).
 * All its data is local state, so a lock, which unmounts this screen, drops it at once (RF-32).
 */
export function VaultScreen({
  client,
  readOnly = false,
  saveSignal = NO_SAVE_SIGNAL,
  barEnd,
  railHead,
  banner,
}: VaultScreenProps) {
  const [tree, setTree] = useState<TreeState>({ kind: "loading" });
  const [treeAttempt, setTreeAttempt] = useState(0);
  /** The group the rail has open; `null` is "Todas las entradas", which is how the vault opens (RF-102). */
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [list, setList] = useState<ListState | null>(null);
  const [listAttempt, setListAttempt] = useState(0);
  /** Asks the list again after a change without showing "Cargando" in between. */
  const [listRefresh, setListRefresh] = useState(0);
  const [detail, setDetail] = useState<DetailState>({ kind: "none" });
  const [notice, setNotice] = useState<Notice>(NO_NOTICE);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<SearchState>({ kind: "idle" });
  /** RF-74 on the list: which row holds the clipboard and until when. It outlives the detail of that entry. */
  const [stamp, setStamp] = useState<{ readonly entryId: string; readonly deadline: number } | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const searchRef = useRef<HTMLInputElement>(null);
  const listPaneRef = useRef<HTMLElement>(null);
  /** RF-59: Enter pressed while the search was still on its way: open the first result when it lands. */
  const openFirst = useRef(false);
  /** The entry opened from the keyboard: its detail takes the focus once loaded (`seq` asks again for the same one). */
  const [keyboardOpen, setKeyboardOpen] = useState<{ readonly entryId: string; readonly seq: number } | null>(null);
  const detailTitleRef = useRef<HTMLHeadingElement>(null);
  const groupsTitleRef = useRef<HTMLHeadingElement>(null);
  /** After a group action removed what had focus, focus goes back to the tree once it is painted. */
  const focusTree = useRef(false);
  const focusDetail = useRef(false);
  /** After "Volver a la lista": the row of the entry that was open takes the focus once the list is shown again. */
  const focusRow = useRef<string | null>(null);
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
    const entryId = focusRow.current;
    if (entryId === null) return;
    focusRow.current = null;
    const index = shownItems().findIndex((item) => item.id === entryId);
    const row = rowButtons()[index];
    if (row === undefined) listPaneRef.current?.querySelector<HTMLElement>("#entries-title")?.focus();
    else row.focus();
    // shownItems and rowButtons read the state of this same render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detail]);

  useEffect(() => {
    if (!focusTree.current) return;
    focusTree.current = false;
    groupsTitleRef.current?.focus();
  }, [tree]);

  const root = tree.kind === "ready" ? tree.root : null;
  const selected = root !== null && selectedGroupId !== null ? findGroup(root, selectedGroupId) : undefined;
  /**
   * The group the list shows; `null` is "Todas las entradas" (RF-102). The root is never a group of its own, and a
   * chosen group that is gone falls back here too.
   */
  const groupId = selected !== undefined && selected.kind !== "root" ? selected.id : null;
  /** The groups whose entries the list asks for, as one text: the list is asked again only when they change. */
  const listKey = root === null ? null : (groupId ?? liveGroupIds(root).join(KEY_SEPARATOR));
  const groupActions = useGroupActions(client, root, onGroupChange);

  useEffect(() => {
    if (listKey === null) return;
    let active = true;
    // RF-102: the core has no "every entry" command, so "Todas las entradas" asks for each group and puts them together.
    const ids = listKey.split(KEY_SEPARATOR);
    void Promise.all(ids.map((id) => client.entryList(id))).then((results) => {
      if (active) setList({ key: listKey, attempt: listAttempt, ...mergeLists(ids, results) });
    });
    return () => {
      active = false;
    };
  }, [client, listKey, listAttempt, listRefresh]);

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
    openFirst.current = false;
    // RF-92: an empty query (after trimming) does not filter: back to what the rail has open.
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
        const first = result.ok ? result.value[0] : undefined;
        if (openFirst.current && first !== undefined) openFromKeyboard({ id: first.id, title: first.title });
        openFirst.current = false;
      });
    }, SEARCH_DELAY_MS);
  }

  function clearSearch() {
    onQueryChange("");
    searchRef.current?.focus();
  }

  /** A group, or `null` for "Todas las entradas". Choosing in the rail ends a search: the list shows what was chosen. */
  function selectGroup(id: string | null) {
    setSelectedGroupId(id);
    setDetail({ kind: "none" });
    if (query !== "") onQueryChange("");
  }

  /** After a change: the tree (counts), the list and an active search, without unmounting the detail. */
  function refresh() {
    setTreeAttempt((value) => value + 1);
    setListRefresh((value) => value + 1);
    if (query.trim() !== "") onQueryChange(query);
  }

  /** P7: the core answered with the new tree. */
  function onGroupChange(change: GroupChange) {
    // The list was showing the group that left its place, or one inside it: back to "Todas las entradas".
    const lost =
      change.gone !== null &&
      root !== null &&
      selectedGroupId !== null &&
      (trailTo(root, selectedGroupId) ?? []).some((group) => group.id === change.gone);
    setTree({ kind: "ready", root: change.root });
    if (lost) setSelectedGroupId(null);
    setListRefresh((value) => value + 1);
    if (query.trim() !== "") onQueryChange(query);
    if (change.removed) {
      setDetail({ kind: "none" });
      focusTree.current = true;
    }
  }

  function selectEntry(entry: SelectedEntry) {
    setNotice(NO_NOTICE);
    setKeyboardOpen(null);
    setDetail({ kind: "entry", entry });
  }

  /** RF-59, UX "buscar y copiar en 3 pasos": open from the keyboard and hand the focus to the detail. */
  function openFromKeyboard(entry: SelectedEntry) {
    setNotice(NO_NOTICE);
    setKeyboardOpen((current) => ({ entryId: entry.id, seq: (current?.seq ?? 0) + 1 }));
    setDetail({ kind: "entry", entry });
  }

  function rowButtons(): HTMLButtonElement[] {
    return [...(listPaneRef.current?.querySelectorAll<HTMLButtonElement>("table.entries tbody th button") ?? [])];
  }

  /** The list that answers what is asked right now, or `null` while it is on its way. */
  function currentList(): ListState | null {
    return list !== null && list.key === listKey && list.attempt === listAttempt ? list : null;
  }

  /** The entries the list shows right now, in order: the search results or what the rail has open. */
  function shownItems(): readonly EntrySummary[] {
    if (searching) return search.kind === "done" ? search.items : [];
    const loaded = currentList()?.result ?? null;
    return loaded?.ok === true ? loaded.value : [];
  }

  function onSearchKeyDown(event: ReactKeyboardEvent<HTMLInputElement>) {
    switch (event.key) {
      case "Escape":
        if (query === "") return;
        event.preventDefault();
        clearSearch();
        return;
      case "ArrowDown":
        event.preventDefault();
        rowButtons()[0]?.focus();
        return;
      case "Enter": {
        event.preventDefault();
        if (searching && search.kind === "searching") {
          openFirst.current = true;
          return;
        }
        const first = shownItems()[0];
        if (first !== undefined) openFromKeyboard({ id: first.id, title: first.title });
        return;
      }
      default:
        return;
    }
  }

  function onRowKeyDown(event: ReactKeyboardEvent<HTMLButtonElement>, entry: SelectedEntry) {
    const buttons = rowButtons();
    const index = buttons.indexOf(event.currentTarget);
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const next = buttons[index + (event.key === "ArrowDown" ? 1 : -1)];
        next?.focus();
        return;
      }
      case "Enter":
        event.preventDefault();
        openFromKeyboard(entry);
        return;
      case "Escape":
        event.preventDefault();
        searchRef.current?.focus();
        return;
      default:
        return;
    }
  }

  const searching = query.trim() !== "";
  const noticeText =
    notice.kind === "text"
      ? notice.text
      : savingText(saveSignal, notice.since, "Entrada creada. Guardándola en el archivo…", "Entrada guardada.");
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
            onKeyDown={onSearchKeyDown}
          />
          <kbd aria-hidden="true">Ctrl K</kbd>
          <p id="vault-search-help" className="sr-only">
            Atajo: Ctrl+F o Ctrl+K. Enter abre el primer resultado y flecha abajo lleva a la lista, donde las
            flechas recorren las entradas, Enter abre una y Escape vuelve aquí. Escape limpia la búsqueda.
          </p>
        </div>
        {/* The lock countdown, the lock button and the settings come from whoever owns the open vault. */}
        <div className="vault-bar-end">{barEnd}</div>
      </div>

      <div className="rail">
        <div className="rail-head">
          {railHead !== undefined && <div className="rail-vault">{railHead}</div>}
          {/* The root is not a row of the rail (RF-102): what can still be asked of it (its name) is asked here. */}
          {root !== null && !readOnly && <GroupMenuButton root={root} group={root} onOrder={groupActions.start} />}
        </div>
        <nav aria-labelledby="groups-title" className="rail-nav">
          {renderTree()}
        </nav>
      </div>

      <div className="work" data-detail={detail.kind}>
        <div className="work-banner">{banner}</div>
        <div className="vault-zones">
          <section ref={listPaneRef} aria-labelledby="entries-title" className="list-pane">
            {searching ? renderSearch() : renderGroup()}
          </section>

          <section aria-labelledby="detail-title" className="detail-pane">
            {/* The focus target after an entry leaves; the entry title is the visible heading of this zone. */}
            <h3 id="detail-title" ref={detailTitleRef} tabIndex={-1} className="sr-only">
              Detalle
            </h3>
            <div aria-live="polite">{noticeText !== "" && <p className="notice">{noticeText}</p>}</div>
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
              setNotice({ kind: "created", since: saveSignal.saved });
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
            saveSignal={saveSignal}
            focusRequest={keyboardOpen?.entryId === detail.entry.id ? keyboardOpen.seq : 0}
            onBack={() => {
              focusRow.current = detail.entry.id;
              setKeyboardOpen(null);
              setDetail({ kind: "none" });
            }}
            onChanged={refresh}
            onCopied={(deadline) => {
              setNow(Date.now());
              setStamp(deadline === null ? null : { entryId: detail.entry.id, deadline });
            }}
            onDeleted={(message) => {
              setDetail({ kind: "none" });
              setNotice({ kind: "text", text: message });
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
          <>
            <h3 id="groups-title">Grupos</h3>
            <p role="status" className="pane-state">
              Cargando los grupos…
            </p>
          </>
        );
      case "failed":
        return (
          <>
            <h3 id="groups-title">Grupos</h3>
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
          </>
        );
      case "ready":
        return (
          <GroupRail
            root={tree.root}
            lines={lines}
            selectedId={groupId}
            marked={!searching}
            readOnly={readOnly}
            titleRef={groupsTitleRef}
            onSelect={selectGroup}
            onOrder={groupActions.start}
          >
            {readOnly && (
              <p className="path rail-note">
                Solo lectura: los grupos no se pueden crear, renombrar, mover ni borrar.
              </p>
            )}
            {groupActions.view}
          </GroupRail>
        );
      default:
        return assertNever(tree);
    }
  }

  function renderGroup() {
    if (root === null) {
      return (
        <header className="list-head">
          <div className="list-title">
            <h3 id="entries-title">Entradas</h3>
          </div>
        </header>
      );
    }
    /** `null`: "Todas las entradas" (RF-102). */
    const group = groupId === null ? null : (findGroup(root, groupId) ?? null);
    const inBin = group?.kind === "recycle_bin";
    const badgeOfGroup = (id: string): Badge => {
      const node = findGroup(root, id);
      return {
        line: lines.get(id) ?? "neutral",
        initial: initialOf(shown(node?.name ?? "", "(sin nombre)")),
        bin: node?.kind === "recycle_bin",
      };
    };
    const current = currentList();
    const loaded = current?.result ?? null;
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
    return (
      <>
        <header className="list-head">
          {group === null ? (
            <span className="bullet" data-line="station" aria-hidden="true">
              <ListIcon />
            </span>
          ) : (
            <Bullet badge={badgeOfGroup(group.id)} />
          )}
          <div className="list-title">
            <h3 id="entries-title">
              {group === null ? (
                "Todas las entradas"
              ) : (
                <>
                  {/* The space is its own text node: inside the hidden span it would be trimmed out of the name. */}
                  <span className="sr-only">Entradas de</span> {shown(group.name, "(sin nombre)")}
                </>
              )}
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
                setNotice(NO_NOTICE);
                // RF-102: from "Todas las entradas" the new entry goes to the root.
                setDetail({ kind: "new", groupId: group?.id ?? root.id });
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
          liveEntryCount(root) === 0 && !inBin ? (
            <p className="pane-state">
              Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardas la primera cuenta.
            </p>
          ) : (
            <p className="pane-state">{inBin ? "La papelera está vacía." : "Este grupo no tiene entradas."}</p>
          )
        ) : (
          <EntryTable
            items={loaded.value}
            groupPaths={null}
            badgeOf={(index) =>
              badgeOfGroup(current?.groupOf.get(loaded.value[index]?.id ?? "") ?? group?.id ?? root.id)
            }
            stamped={stamped}
            selectedId={selectedId}
            onSelect={selectEntry}
            onRowKeyDown={onRowKeyDown}
          />
        )}
      </>
    );
  }

  function renderSearch() {
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
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
            onRowKeyDown={onRowKeyDown}
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
function Bullet({ badge }: { readonly badge: Badge }) {
  return (
    <span
      className="bullet"
      data-line={badge.line}
      data-initial={badge.bin ? undefined : badge.initial}
      aria-hidden="true"
    >
      {badge.bin && <TrashIcon />}
    </span>
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
  /** RF-59: arrows move between rows, Enter opens one, Escape goes back to the search. */
  readonly onRowKeyDown: (event: ReactKeyboardEvent<HTMLButtonElement>, entry: SelectedEntry) => void;
}

/**
 * RF-42: entries in the order the core sends (by title). The URL is text, never a link (RF-78, RF-49). It is a real
 * table drawn as a list of rows: the title button stretches over its row, and the header row stays for screen readers.
 */
function EntryTable({ items, groupPaths, badgeOf, stamped, selectedId, onSelect, onRowKeyDown }: EntryTableProps) {
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
                  onKeyDown={(event) => {
                    onRowKeyDown(event, { id: entry.id, title: entry.title });
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
                {stamped?.entryId === entry.id && <span className="stamp"><span className="stamp-word">Copiado</span> · {stamped.seconds} s</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
