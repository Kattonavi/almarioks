import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { createClient, type Transport, type VaultClient } from "./client";

/** The real channel. Needs `core:event:allow-listen` in the window capability for the core's events. */
export const tauriTransport: Transport = {
  invoke: (command, payload) => invoke<unknown>(command, payload),
  listen: (event, handler) =>
    listen<unknown>(event, (message) => {
      handler(message.payload);
    }),
};

export function createTauriClient(): VaultClient {
  return createClient(tauriTransport);
}
