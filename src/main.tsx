import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { createTauriClient } from "./ipc/tauri";
import "@fontsource-variable/overpass/index.css";
import "@fontsource-variable/overpass-mono/index.css";
import "./styles.css";
import { VaultRoot } from "./vault/VaultRoot";

const client = createTauriClient();

const container = document.getElementById("root");
if (container === null) {
  throw new Error("Missing #root element in index.html");
}

createRoot(container).render(
  <StrictMode>
    <App>
      <VaultRoot client={client} />
    </App>
  </StrictMode>,
);
