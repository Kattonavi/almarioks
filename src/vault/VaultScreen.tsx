import {
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";

import { assertNever } from "../assertNever";
import type { FieldName, VaultClient } from "../ipc/client";
import { errorMessage, type AppError, type Result } from "../ipc/errors";
import type { EntrySummary, GroupNode, SearchResult } from "../ipc/schemas";
import { CheckIcon, CopyIcon, MouseIcon, PlusIcon, SearchIcon, WindowIcon } from "../ui/icons";
import { Dialog } from "./Dialog";
import { EntryDialog } from "./EntryDialog";
import { copyNoticeOf, EntryPanel, type CopyNotice } from "./EntryPanel";
import { findGroup, formatShortDay, plural, shortSite, shown, trailTo } from "./format";
import { useGroupActions, type GroupChange } from "./GroupActions";
import { GroupMenuButton, GroupRail } from "./GroupRail";
import { assignLines, entryInitial, entryLine, lineOfPath, type GroupLine } from "./lines";
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

/** The entry the list has chosen, and whether its window is open over the list (RF-43). */
type DetailState =
  | { readonly kind: "none" }
  | { readonly kind: "entry"; readonly entry: SelectedEntry; readonly open: boolean };

/** The notice of the list: a plain text, or a new entry waiting for its save to reach the disk (RF-21). */
type Notice = { readonly kind: "text"; readonly text: string } | { readonly kind: "created"; readonly since: number };

const NO_NOTICE: Notice = { kind: "text", text: "" };

/** RF-73: what the copy notice says once the time the core gave is over. */
const CLEARED = "Se cumplió el tiempo: AlmarioKS limpió el portapapeles si nadie copió otra cosa.";

/**
 * RF-74: the words of the copy notice. A protected value counts down the seconds the core gave until it clears the
 * clipboard; a value that is not protected is only named, because nothing clears it.
 */
function copyNoticeText(copy: CopyNotice | null, now: number): string {
  if (copy === null) return "";
  if (copy.deadline === null) return copy.what;
  const seconds = Math.ceil((copy.deadline - now) / 1000);
  return seconds > 0 ? `${copy.what}. Se borra del portapapeles en ${String(seconds)} s` : CLEARED;
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
 * P4: the open vault (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92, RF-102), with P5 in a window over it (RF-43). A signal
 * frame (top bar with the search, rail with the groups) around the work zone: the entries of what the rail has open,
 * or the search results, as a table. Every vault datum is rendered as a React text child, never as HTML or as a link (RF-78).
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
  /** RF-100: the group a new entry is being written for, in its dialog over the vault; `null`: no dialog. */
  const [newEntryIn, setNewEntryIn] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(NO_NOTICE);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<SearchState>({ kind: "idle" });
  /** RF-74: the last copy, for the notice at the foot of the list or of the window. It outlives that window. */
  const [copyNotice, setCopyNotice] = useState<CopyNotice | null>(null);
  /** What an order given from the list (copy, open the site) could not do. */
  const [listError, setListError] = useState<AppError | null>(null);
  /** The title of the window of the entry: the dialog takes its name from it. */
  const windowTitleId = useId();
  const [now, setNow] = useState(() => Date.now());
  const searchRef = useRef<HTMLInputElement>(null);
  const listPaneRef = useRef<HTMLElement>(null);
  /** RF-59: Enter pressed while the search was still on its way: open the first result when it lands. */
  const openFirst = useRef(false);
  /** The entry opened from the keyboard: its window takes the focus once loaded (`seq` asks again for the same one). */
  const [keyboardOpen, setKeyboardOpen] = useState<{ readonly entryId: string; readonly seq: number } | null>(null);
  const groupsTitleRef = useRef<HTMLHeadingElement>(null);
  /** After a group action removed what had focus, focus goes back to the tree once it is painted. */
  const focusTree = useRef(false);
  /** After an entry left (to the bin or for good): the title of the list takes the focus. */
  const focusList = useRef(false);
  /**
   * The entry whose "Copiar contraseña" takes the focus once its row is chosen and in the list: the first result
   * after Enter in the search (RF-59), or an entry just created, whose row arrives with the next list.
   */
  const focusCopy = useRef<string | null>(null);
  /** RF-108: after the window closes, the row of its entry takes the focus. */
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
    if (!focusList.current) return;
    focusList.current = false;
    listPaneRef.current?.querySelector<HTMLElement>("#entries-title")?.focus();
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

  // After every render: the row may arrive later than the choice (a new entry waits for the list to be asked again).
  useEffect(() => {
    const entryId = focusCopy.current;
    if (entryId === null || detail.kind !== "entry" || detail.entry.id !== entryId) return;
    // While a modal dialog is open everything behind it is inert: the focus waits for it to close.
    if (document.querySelector("dialog[open]") !== null) return;
    const button = listPaneRef.current?.querySelector<HTMLElement>("table.entries tbody tr[data-selected] .row-copy");
    if (button === null || button === undefined) return;
    focusCopy.current = null;
    // The focus brings the row into view when it is out of sight.
    button.focus();
  });

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

  // RF-74: the copy notice counts down with the clipboard and stops at zero.
  useEffect(() => {
    const deadline = copyNotice?.deadline ?? null;
    if (deadline === null) return;
    const timer = setInterval(() => {
      const at = Date.now();
      setNow(at);
      if (at >= deadline) clearInterval(timer);
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  }, [copyNotice]);

  // Ctrl+F or Ctrl+K go to the search field from anywhere in the vault.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key !== "f" && key !== "k") return;
      event.preventDefault();
      // With a modal dialog open, what is behind it is out of reach: the key is still taken from the browser.
      if (document.querySelector("dialog[open]") !== null) return;
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
        if (openFirst.current && first !== undefined) chooseFromKeyboard({ id: first.id, title: first.title });
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
    focusCopy.current = null;
    settleCopyNotice();
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

  /**
   * Another entry is chosen: the copy notice goes with the one it spoke of, unless the clipboard still holds a
   * protected value that is counting down. That one stays until the core clears it.
   */
  function settleCopyNotice() {
    const at = Date.now();
    setCopyNotice((current) => (current !== null && current.deadline !== null && current.deadline > at ? current : null));
  }

  function selectEntry(entry: SelectedEntry) {
    focusCopy.current = null;
    settleCopyNotice();
    setNotice(NO_NOTICE);
    setListError(null);
    setKeyboardOpen(null);
    setDetail({ kind: "entry", entry, open: true });
  }

  /** RF-43: the window of the entry, over the list. */
  function openWindow(entry: SelectedEntry) {
    focusCopy.current = null;
    setNotice(NO_NOTICE);
    setListError(null);
    setKeyboardOpen(null);
    setDetail({ kind: "entry", entry, open: true });
  }

  /** RF-108: back to the list. The entry stays chosen and its row takes the focus. */
  function closeWindow(entry: SelectedEntry) {
    focusRow.current = entry.id;
    setKeyboardOpen(null);
    setDetail({ kind: "entry", entry, open: false });
  }

  /** RF-71, RF-106: a copy asked from the list. Always through the core: the window never holds what is copied. */
  async function copyFromList(entry: SelectedEntry, field: FieldName) {
    setListError(null);
    const result = await client.entryCopy(entry.id, field);
    if (!mounted.current) return;
    if (!result.ok) {
      setListError(result.error);
      return;
    }
    setNow(Date.now());
    setCopyNotice(copyNoticeOf(field, result.value.clearsInSeconds));
  }

  /**
   * RF-59, "buscar y copiar en 3 pasos": Enter in the search chooses the first result and leaves the focus on its
   * "Copiar contraseña", so the next Enter copies. The window does not open by itself.
   */
  function chooseFromKeyboard(entry: SelectedEntry) {
    settleCopyNotice();
    setNotice(NO_NOTICE);
    setListError(null);
    setKeyboardOpen(null);
    focusCopy.current = entry.id;
    setDetail({ kind: "entry", entry, open: false });
  }

  /** RF-43, RF-59: Enter on the title of a row opens its window and hands it the focus ("Copiar contraseña"). */
  function openFromKeyboard(entry: SelectedEntry) {
    focusCopy.current = null;
    settleCopyNotice();
    setNotice(NO_NOTICE);
    setListError(null);
    setKeyboardOpen((current) => ({ entryId: entry.id, seq: (current?.seq ?? 0) + 1 }));
    setDetail({ kind: "entry", entry, open: true });
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
        if (first !== undefined) chooseFromKeyboard({ id: first.id, title: first.title });
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
  const copyText = copyNoticeText(copyNotice, now);
  /** RF-74: the one notice of a copy. It is said where the person is looking: in the window while it is open. */
  const toast = copyText !== "" && (
    <p className="toast">
      <CheckIcon />
      {copyText}
    </p>
  );
  const windowOpen = detail.kind === "entry" && detail.open;
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
              focusCopy.current = null;
              onQueryChange(event.target.value);
            }}
            onKeyDown={onSearchKeyDown}
          />
          <kbd aria-hidden="true">Ctrl K</kbd>
          <p id="vault-search-help" className="sr-only">
            Atajo: Ctrl+F o Ctrl+K. Enter elige el primer resultado y lleva a su botón de copiar contraseña. Flecha
            abajo lleva a la lista, donde las flechas recorren las entradas, Enter abre una y Escape vuelve aquí.
            Escape limpia la búsqueda.
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

      <div className="work">
        <div className="work-banner">{banner}</div>
        <div className="vault-zones">
          <section ref={listPaneRef} aria-labelledby="entries-title" className="list-pane">
            {searching ? renderSearch() : renderGroup()}
            <div className="list-foot">
              {/* What the last order left to say (a new entry, one sent to the bin) and what it could not do. */}
              <div aria-live="polite" className="list-notice">
                {noticeText !== "" && <p>{noticeText}</p>}
              </div>
              <p role="alert" className="error list-error">
                {listError === null ? "" : errorMessage(listError)}
              </p>
              {/* RF-74: a copy is said here while no window is open; `role="status"` announces it. */}
              <div role="status" className="toast-zone">
                {!windowOpen && toast}
              </div>
              <p className="list-hint">
                <span className="list-hint-key">
                  <MouseIcon />
                  Doble clic
                </span>
                <span>en el título: abre la entrada</span>
                <span>en usuario o contraseña: copia</span>
                <span>en el sitio: lo abre en el navegador</span>
              </p>
            </div>
          </section>
        </div>
      </div>
      {renderWindow()}
      {renderNewEntry()}
    </div>
  );

  /** RF-100: "Nueva entrada" is a dialog over the vault; the list stays behind it as it was. */
  function renderNewEntry() {
    if (newEntryIn === null || tree.kind !== "ready") return null;
    const group = findGroup(tree.root, newEntryIn) ?? tree.root;
    return (
      <EntryDialog
        client={client}
        mode={{
          kind: "new",
          group: { id: group.id, name: shown(group.name, "(sin nombre)"), line: entryLine(lines.get(group.id)) },
        }}
        onClose={() => {
          setNewEntryIn(null);
        }}
        onSaved={(entry) => {
          // The new entry is the chosen one, and its window does not open by itself. The focus goes to its "Copiar
          // contraseña" once its row is in the list (the dialog gives it back to "Nueva entrada" first).
          settleCopyNotice();
          setNotice({ kind: "created", since: saveSignal.saved });
          setListError(null);
          setKeyboardOpen(null);
          focusCopy.current = entry.id;
          setDetail({ kind: "entry", entry: { id: entry.id, title: entry.title }, open: false });
          refresh();
        }}
      />
    );
  }

  /**
   * RF-43, RF-108: the entry in a modal window over the list. The panel is mounted only while the window is open, so
   * closing it (or a lock, which unmounts the whole screen) drops every value it was showing.
   */
  function renderWindow() {
    switch (detail.kind) {
      case "none":
        return null;
      case "entry": {
        if (!detail.open || tree.kind !== "ready") return null;
        const { entry } = detail;
        return (
          <Dialog
            layout="entry"
            labelledBy={windowTitleId}
            onClose={() => {
              closeWindow(entry);
            }}
          >
            <EntryPanel
              key={entry.id}
              client={client}
              entryId={entry.id}
              listTitle={entry.title}
              root={tree.root}
              readOnly={readOnly}
              saveSignal={saveSignal}
              focusRequest={keyboardOpen?.entryId === entry.id ? keyboardOpen.seq : 0}
              titleId={windowTitleId}
              onClose={() => {
                closeWindow(entry);
              }}
              onChanged={refresh}
              onCopied={(copy) => {
                setNow(Date.now());
                setCopyNotice(copy);
              }}
              onDeleted={(message) => {
                settleCopyNotice();
                setDetail({ kind: "none" });
                setNotice({ kind: "text", text: message });
                refresh();
                // After the commit: while a modal dialog is open, everything outside it is inert.
                focusList.current = true;
              }}
            />
            {/* RF-74: while the window is open the copy notice is said inside it. */}
            <div role="status" className="toast-zone">
              {toast}
            </div>
          </Dialog>
        );
      }
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

  /** RF-38: the main action of the list. From "Todas las entradas", and from a search over it, the entry goes to the root. */
  function newEntryButton(target: string) {
    if (readOnly) return null;
    return (
      <button
        type="button"
        className="btn-primary"
        onClick={() => {
          setNewEntryIn(target);
        }}
      >
        <PlusIcon />
        Nueva entrada
      </button>
    );
  }

  function renderGroup() {
    if (root === null) {
      return (
        <header className="list-head">
          <div className="list-title">
            <h3 id="entries-title" tabIndex={-1}>
              Entradas
            </h3>
          </div>
        </header>
      );
    }
    /** `null`: "Todas las entradas" (RF-102). */
    const group = groupId === null ? null : (findGroup(root, groupId) ?? null);
    const inBin = group?.kind === "recycle_bin";
    const groupName = group === null ? "Todas las entradas" : shown(group.name, "(sin nombre)");
    const current = currentList();
    const loaded = current?.result ?? null;
    const selectedId = detail.kind === "entry" ? detail.entry.id : null;
    return (
      <>
        <header className="list-head">
          <div className="list-title">
            {/* One line, cut with an ellipsis: the whole name is in the `title`. */}
            <h3 id="entries-title" tabIndex={-1} title={groupName}>
              {group === null ? (
                groupName
              ) : (
                <>
                  {/* The space is its own text node: inside the hidden span it would be trimmed out of the name. */}
                  <span className="sr-only">Entradas de</span> {groupName}
                </>
              )}
            </h3>
            {loaded !== null && loaded.ok && <p role="status">{plural(loaded.value.length, "entrada", "entradas")}</p>}
          </div>
          {newEntryButton(group?.id ?? root.id)}
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
          <p className="pane-state">
            {inBin
              ? "La papelera está vacía."
              : readOnly
                ? "No hay entradas aquí."
                : "No hay entradas aquí. Crea la primera con «Nueva entrada»."}
          </p>
        ) : (
          <EntryTable
            items={loaded.value}
            groupPaths={null}
            lineOf={(index) =>
              // In "Todas las entradas" each row keeps the colour of the group it was asked from.
              entryLine(lines.get(current?.groupOf.get(loaded.value[index]?.id ?? "") ?? group?.id ?? root.id))
            }
            selectedId={selectedId}
            onSelect={selectEntry}
            onOpen={openWindow}
            onCopy={(entry, field) => void copyFromList(entry, field)}
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
          <div className="list-title">
            {/* It reads "Resultados"; its name stays "Resultados de la búsqueda". */}
            <h3 id="entries-title" tabIndex={-1}>
              {/* The space is its own text node: inside the hidden span it would be trimmed out of the name. */}
              Resultados <span className="sr-only">de la búsqueda</span>
            </h3>
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
          {/* The new entry goes where it would go without the search: the group the rail had open, or the root. */}
          {root !== null && newEntryButton(groupId ?? root.id)}
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
            lineOf={(index) =>
              // A result brings the names of its groups, not an id: the path is walked down the tree.
              root === null ? "neutral" : entryLine(lineOfPath(root, lines, search.items[index]?.groupPath ?? []))
            }
            selectedId={selectedId}
            onSelect={selectEntry}
            onOpen={openWindow}
            onCopy={(entry, field) => void copyFromList(entry, field)}
            onRowKeyDown={onRowKeyDown}
          />
        )}
      </>
    );
  }
}

interface EntryTableProps {
  readonly items: readonly EntrySummary[];
  /** Search results: the group path of each item, from the root down. */
  readonly groupPaths: readonly (readonly string[])[] | null;
  /** The line colour of the group each row sits in. */
  readonly lineOf: (index: number) => GroupLine;
  readonly selectedId: string | null;
  readonly onSelect: (entry: SelectedEntry) => void;
  /** RF-43: the window of the entry. */
  readonly onOpen: (entry: SelectedEntry) => void;
  /** RF-71: a copy, which the core makes. */
  readonly onCopy: (entry: SelectedEntry, field: FieldName) => void;
  /** RF-59: arrows move between rows, Enter opens one, Escape goes back to the search. */
  readonly onRowKeyDown: (event: ReactKeyboardEvent<HTMLButtonElement>, entry: SelectedEntry) => void;
}

/** RF-105: what the password cell draws. Always the same marks: it says nothing of the value, not even its length. */
const PASSWORD_MASK = "••••••••";

/** RF-105: the password cell of a row. The list never gets the value; this is the one place that decides what it draws. */
// eslint-disable-next-line @typescript-eslint/no-unused-vars -- the entry is what RF-105 will read (`hasPassword`)
function passwordCell(_entry: EntrySummary): ReactNode {
  // RF-105: hasPassword arrives with the core change; until then the mask is always shown
  return (
    <>
      <span className="mask" aria-hidden="true">
        {PASSWORD_MASK}
      </span>
      <span className="sr-only">Oculta</span>
    </>
  );
}

/**
 * RF-42: the entries as a table, in the order the core sends (by title): title, user, a fixed mask for the password,
 * site and the day it was modified. The URL is text, never a link (RF-78, RF-49). The tags and, in search results,
 * the group stay in the table for screen readers. Each row opens with a circle: the initial of the entry's own title
 * on the colour of its group, drawn by CSS from `data-initial`, so it adds no text; a protected title never reaches
 * it. Only the chosen row shows its two buttons, in a cell every row has: choosing a row moves nothing.
 */
function EntryTable({ items, groupPaths, lineOf, selectedId, onSelect, onOpen, onCopy, onRowKeyDown }: EntryTableProps) {
  const today = new Date();
  return (
    <div className="table-scroll">
      <table className="entries" data-search={groupPaths === null ? undefined : ""}>
        <thead>
          <tr>
            <th scope="col" className="cell-title">
              Título
            </th>
            <th scope="col" className="cell-user">
              Usuario
            </th>
            <th scope="col" className="cell-password">
              Contraseña
            </th>
            <th scope="col" className="cell-url">
              Sitio
            </th>
            <th scope="col" className="cell-tags">
              Etiquetas
            </th>
            {groupPaths !== null && (
              <th scope="col" className="cell-group">
                Grupo
              </th>
            )}
            <th scope="col" className="cell-date">
              Modificada
            </th>
            <th scope="col" className="cell-actions">
              <span className="sr-only">Acciones</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((entry, index) => {
            const chosen = entry.id === selectedId;
            const picked: SelectedEntry = { id: entry.id, title: entry.title };
            // A protected title arrives as `null` and reads "(protegido)": its value never reaches a name.
            const title = shown(entry.title, "(sin título)");
            const noUser = entry.username === "";
            const noSite = entry.url === "";
            const select = () => {
              onSelect(picked);
            };
            return (
              <tr key={entry.id} data-selected={chosen ? "" : undefined}>
                <th scope="row" className="cell-title">
                  <button
                    type="button"
                    title="Doble clic para abrir la entrada"
                    aria-current={chosen ? "true" : undefined}
                    onClick={select}
                    onKeyDown={(event) => {
                      onRowKeyDown(event, picked);
                    }}
                  >
                    <span
                      className="bullet"
                      data-line={lineOf(index)}
                      data-initial={entryInitial(entry.title)}
                      aria-hidden="true"
                    />
                    <span className="cell-text">{title}</span>
                  </button>
                </th>
                <td
                  className="cell-user"
                  data-empty={noUser ? "" : undefined}
                  title={noUser ? undefined : "Doble clic para copiar el usuario"}
                  onClick={select}
                >
                  <span className="cell-text">{shown(entry.username, "—")}</span>
                </td>
                <td className="cell-password" title="Doble clic para copiar la contraseña" onClick={select}>
                  {passwordCell(entry)}
                </td>
                {/* The row shows the site without `https://`; the whole address is in the window of the entry. */}
                <td
                  className="cell-url"
                  data-empty={noSite ? "" : undefined}
                  title={noSite ? undefined : "Doble clic para abrir el sitio en el navegador"}
                  onClick={select}
                >
                  <span className="cell-text">
                    {entry.url === null ? shown(entry.url) : noSite ? "—" : shortSite(entry.url)}
                  </span>
                </td>
                <td className="cell-tags">{entry.tags.join(", ")}</td>
                {groupPaths !== null && <td className="cell-group">{(groupPaths[index] ?? []).join(" › ")}</td>}
                <td className="cell-date" title="Doble clic para abrir la entrada" onClick={select}>
                  <time dateTime={entry.modified}>{formatShortDay(entry.modified, today)}</time>
                </td>
                <td className="cell-actions">
                  {chosen && (
                    <>
                      <button
                        type="button"
                        className="row-copy"
                        aria-label={`Copiar contraseña de ${title}`}
                        title="Copiar contraseña"
                        onClick={() => {
                          onCopy(picked, "Password");
                        }}
                      >
                        <CopyIcon />
                      </button>
                      <button
                        type="button"
                        className="row-open"
                        aria-label={`Abrir ${title}`}
                        title="Abrir la entrada"
                        onClick={() => {
                          onOpen(picked);
                        }}
                      >
                        <WindowIcon />
                      </button>
                    </>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
