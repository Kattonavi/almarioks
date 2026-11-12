/// <reference types="vitest/config" />
import process from "node:process";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  // Keep Rust errors visible in the terminal.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port (devUrl in src-tauri/tauri.conf.json).
    port: 1420,
    strictPort: true,
    host: host ?? false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // Rust sources and build output are not frontend files.
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  build: {
    // WebView2 on Windows 10/11 is evergreen Chromium.
    target: "chrome120",
    sourcemap: false,
    // The CSP is `font-src 'self'`: a font inlined as a `data:` URL would be blocked, so every asset stays a file.
    assetsInlineLimit: 0,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test/setup.ts"],
  },
});
