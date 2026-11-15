import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { createTauriClient } from "./ipc/tauri";
import "@fontsource-variable/overpass/index.css";
import "@fontsource-variable/overpass-mono/index.css";
import "./styles.css";
import { closeBrowserSurface } from "./ui/browserSurface";
import { VaultRoot } from "./vault/VaultRoot";

// Before anything is on screen: no context menu and no keys of the browser (RF-77; D-9, D-10, D-11).
closeBrowserSurface();

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
