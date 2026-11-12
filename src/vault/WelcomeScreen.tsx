import { useEffect, useState } from "react";

import type { VaultClient } from "../ipc/client";
import { errorMessage, type AppError } from "../ipc/errors";
import type { RecentVault } from "../ipc/schemas";
import { ChevronRightIcon, FolderIcon, PlusIcon, WarningIcon } from "../ui/icons";
import { initialOf } from "./lines";
import { fileNameOf } from "./state";

type RecentState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly items: readonly RecentVault[] }
  | { readonly kind: "failed"; readonly error: AppError };

interface WelcomeScreenProps {
  readonly client: Pick<VaultClient, "recentList" | "recentRemove" | "pickVaultToOpen">;
  /** A vault file was chosen: go to P3 with it. */
  readonly onChoose: (path: string) => void;
  readonly onCreate: () => void;
}

/** P1: create a vault, open one, or pick one of the last five (RF-19), offering to find or forget a lost one (RF-20). */
export function WelcomeScreen({ client, onChoose, onCreate }: WelcomeScreenProps) {
  const [recent, setRecent] = useState<RecentState>({ kind: "loading" });
  const [actionError, setActionError] = useState<AppError | null>(null);

  useEffect(() => {
    let active = true;
    void client.recentList().then((result) => {
      if (!active) return;
      setRecent(result.ok ? { kind: "ready", items: result.value } : { kind: "failed", error: result.error });
    });
    return () => {
      active = false;
    };
  }, [client]);

  async function pickAndChoose() {
    setActionError(null);
    const result = await client.pickVaultToOpen();
    if (!result.ok) {
      setActionError(result.error);
    } else if (result.value !== null) {
      onChoose(result.value);
    }
  }

  async function forget(path: string) {
    setActionError(null);
    const result = await client.recentRemove(path);
    if (result.ok) setRecent({ kind: "ready", items: result.value });
    else setActionError(result.error);
  }

  return (
    <section aria-labelledby="welcome-title" className="gate">
      <h2 id="welcome-title" className="gate-title">
        Bienvenida
      </h2>
      <div className="actions gate-actions">
        <button type="button" className="btn-primary" onClick={onCreate}>
          <PlusIcon />
          Crear bóveda nueva
        </button>
        <button type="button" onClick={() => void pickAndChoose()}>
          <FolderIcon />
          Abrir una bóveda…
        </button>
      </div>

      <p role="alert" className="error">
        {actionError === null ? "" : errorMessage(actionError)}
      </p>

      <h3 id="recent-title" className="gate-subtitle">
        Bóvedas recientes
      </h3>
      <RecentList
        state={recent}
        onChoose={onChoose}
        onFind={() => void pickAndChoose()}
        onForget={(path) => void forget(path)}
      />
    </section>
  );
}

interface RecentListProps {
  readonly state: RecentState;
  readonly onChoose: (path: string) => void;
  readonly onFind: () => void;
  readonly onForget: (path: string) => void;
}

function RecentList({ state, onChoose, onFind, onForget }: RecentListProps) {
  switch (state.kind) {
    case "loading":
      return (
        <p role="status" className="pane-state">
          Buscando tus bóvedas recientes…
        </p>
      );
    case "failed":
      return (
        <p role="alert" className="error">
          {errorMessage(state.error)}
        </p>
      );
    case "ready":
      if (state.items.length === 0) {
        return (
          <p className="pane-state">
            Todavía no has abierto ninguna bóveda. Crea una nueva o abre una que ya tengas.
          </p>
        );
      }
      return (
        <ul aria-labelledby="recent-title" className="recent">
          {state.items.map((item) => {
            const name = fileNameOf(item.path);
            return item.exists ? (
              // A vault is a station: the same white badge the root group wears on the rail.
              <li key={item.path} className="recent-row">
                <span className="bullet" data-line="station" data-initial={initialOf(name)} aria-hidden="true" />
                <button type="button" onClick={() => { onChoose(item.path); }}>
                  Abrir {name}
                </button>{" "}
                <span className="path mono">{item.path}</span>
                <ChevronRightIcon />
              </li>
            ) : (
              <li key={item.path} className="recent-row" data-missing="">
                <span className="bullet" data-line="neutral" aria-hidden="true">
                  <WarningIcon />
                </span>
                <p>
                  {name} ya no está en <span className="path mono">{item.path}</span>. Puede que se haya movido o
                  borrado.
                </p>
                <div className="actions">
                  <button type="button" onClick={onFind}>
                    Buscar {name}…
                  </button>
                  <button type="button" onClick={() => { onForget(item.path); }}>
                    Quitar {name} de la lista
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      );
    default:
      return state satisfies never;
  }
}
