import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type ReactNode } from "react";

import { BrandMark } from "./ui/BrandMark";
import { WarningIcon } from "./ui/icons";

type VersionState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly version: string }
  | { readonly kind: "error" };

function assertNever(value: never): never {
  throw new Error(`Unexpected state: ${JSON.stringify(value)}`);
}

function VersionText({ state }: { readonly state: VersionState }) {
  switch (state.kind) {
    case "loading":
      return <>Leyendo la versión…</>;
    case "ready":
      return <>Versión {state.version}</>;
    case "error":
      return <>No se pudo leer la versión.</>;
    default:
      return assertNever(state);
  }
}

/**
 * The app shell: the signal bar with the name, the screen, and a status strip with the version. The update panel
 * (P9) lives in "Ajustes y Acerca de" (P8), which the vault root offers on every screen. The vault screens come in
 * as `children` (composed in main.tsx), so the shell itself never talks to the vault.
 */
export function App({ children }: { readonly children?: ReactNode }) {
  const [state, setState] = useState<VersionState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    getVersion().then(
      (version) => {
        if (active) setState({ kind: "ready", version });
      },
      () => {
        if (active) setState({ kind: "error" });
      },
    );
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="app" data-density="comfortable">
      <header className="app-bar">
        <BrandMark />
        <h1 className="app-name">AlmarioKS</h1>
        {/* Shown only beside "Crear bóveda", where the totem is the brand panel (`gate.css`). */}
        <p className="app-tagline">Tus contraseñas en un archivo tuyo. Sin cuenta y sin nube.</p>
      </header>
      <main className="app-main">{children}</main>
      <footer className="app-status">
        <p>Gestor de contraseñas local.</p>
        <p aria-live="polite" className="num">
          <VersionText state={state} />
        </p>
        <p className="app-status-note">
          <WarningIcon />
          Versión de desarrollo: no la uses con tus contraseñas reales.
        </p>
      </footer>
    </div>
  );
}
