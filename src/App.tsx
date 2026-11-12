import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type ReactNode } from "react";

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
 * The app shell: name and version. The update panel (P9) lives in "Ajustes y Acerca de" (P8), which the vault root
 * offers on every screen. The vault screens come in as `children` (composed in main.tsx), so the shell itself never
 * talks to the vault.
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
    <main>
      <h1>AlmarioKS</h1>
      <p>Gestor de contraseñas local.</p>
      <p aria-live="polite">
        <VersionText state={state} />
      </p>
      <p>Versión de desarrollo: no la uses con tus contraseñas reales.</p>
      {children}
    </main>
  );
}
