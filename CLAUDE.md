# AlmarioKS — Guía para Claude Code

Hereda el `CLAUDE.md` y los estándares (`estandares/`) del Workspace del autor, que es una carpeta privada fuera de
este repo; lo que aplica a este proyecto está copiado en `docs/CHECKLIST.md`.
Este archivo solo agrega o endurece; nunca relaja. Corto: lo que se deduce del código no se escribe aquí.

- **Nivel / talla / riesgo:** E3 + anexo Rust (R, T) · L (4 fases, 64 h) · alto · **Contexto:** `docs/PROYECTO.md` · **Checklist:** `docs/CHECKLIST.md`
- **Specs:** `docs/specs/NNN-*/` (skill `especificar`) · **Decisiones:** `docs/adr/` · **Amenazas:** `docs/AMENAZAS.md`
- **Diseño:** `DESIGN.md` (lo escribe impeccable-documenter) · **Dueño técnico:** `rust-engineer`
- **Repo público (GPL-3.0):** nada de secretos, datos personales ni rutas del PC del usuario en commits, issues o logs.

## Constitución (cada línea con el comando que la verifica)
1. **La spec manda:** nada se implementa si no está en la spec activa; si falta una decisión, se para y se pregunta.
2. **Pruebas como puerta:** cada tarea termina con su prueba en verde — `cargo nextest run --workspace --locked` · `pnpm test`.
3. **Cada RF tiene una prueba que lo cita** (`rf_NN_…` en Rust, `it("RF-NN: …")` en TS) — `/especificar validar`.
4. **Cero criptografía propia:** cifrado, KDF y formato salen de `keepass` y sus dependencias; ningún crate de
   cifrado en código nuestro fuera de `almarioks-core::kdbx` — `cargo deny check` + revisión del `Cargo.toml`.
   (Las pruebas del actualizador usan `minisign` solo como `dev-dependency`, para fabricar llaves de prueba.)
5. **Ida y vuelta sin pérdida:** lo que la app no edita, lo conserva — pruebas `roundtrip_*` de `almarioks-core`
   contra `tests/fixtures/` (KeePass 2 y KeePassXC).
6. **Los secretos no cruzan a la interfaz:** ningún comando de Tauri devuelve una contraseña salvo `reveal_*`;
   copiar y rellenar salen del núcleo — prueba de contrato sobre los comandos.
7. **El bloqueo no falla abierto:** bloquear siempre limpia memoria y portapapeles aunque el guardado falle —
   pruebas con y sin cambios pendientes (E3-3).
8. **El núcleo no conoce Tauri:** `almarioks-core` no depende de `tauri` — `cargo tree -p almarioks-core -i tauri` no devuelve nada.
9. **Rust sin atajos:** lints del workspace de `estandares/rust.md`, sin `unsafe` fuera de `ffi` —
   `cargo clippy --all-targets --all-features --locked -- -D warnings` · `cargo fmt --check`.
10. **TypeScript strict, sin `any`** — `pnpm typecheck` · `pnpm lint`.
11. **Sin red, salvo el actualizador** (y solo con permiso del usuario): ni telemetría ni llamadas a terceros —
    capacidades y CSP de Tauri (T-2, T-3) + revisión de `capabilities/*.json`.
12. **El navegador pide, la app decide:** la extensión no guarda secretos y la coincidencia por origen exacto se
    resuelve en el núcleo — pruebas de origen (dominios parecidos, `http`, subdominios).
13. Idioma: código, identificadores y commits en inglés; textos al usuario y documentos en español.
14. **Primero completo, luego pulido:** ninguna revisión de seguridad, diseño o QA antes de la matriz de completitud en verde.

## Comandos (confirmados en el scaffold del H0, 2026-11-11)
| Tarea | Comando |
|---|---|
| Dev | `pnpm tauri dev` |
| Rust: formato / lints / pruebas | `cargo fmt --check` · `cargo clippy --all-targets --all-features --locked -- -D warnings` · `cargo nextest run --workspace --locked` (las de `clipboard_win32` pisan el portapapeles del PC) |
| Dependencias | `cargo deny check` |
| Interfaz: tipos / lint / pruebas | `pnpm typecheck` · `pnpm lint` · `pnpm test` |
| Instalador | `pnpm tauri build` (firma el paquete de actualización: pide `TAURI_SIGNING_PRIVATE_KEY`, solo en CI) · sin llave: `pnpm tauri build --no-bundle` |
| Auditoría npm | `pnpm audit --prod` |

## Mapa del código
- `Cargo.toml` — workspace: versión única (`[workspace.package]`, de ahí la toma Tauri), lints (`unsafe_code = deny`; el núcleo lo prohíbe con `#![forbid]`), perfil release; `rust-toolchain.toml` fija 1.99.1.
- `crates/almarioks-core/` — núcleo sin Tauri ni red. `kdbx/` (único módulo que nombra tipos de `keepass`: lector propio de cabecera, llave, abrir, crear, entradas, grupos, búsqueda, conversión), `vault/` (tipos propios y validación), `storage/` (guardado atómico, `.bak`, cola, recuperación), `session.rs` (cerrada / bloqueada / abierta; el bloqueo no espera al disco), `generator`, `strength`, `settings`, `text`. Pruebas: `tests/rf_*.rs` (del implementador), `tests/spec_*.rs` (escritas desde la spec sin ver la implementación), `tests/roundtrip*.rs` y `tests/fixtures/` (datos falsos).
- `src-tauri/` — app Tauri 2.12. `src/commands/` (un parámetro `args` estricto por comando; lista en `list.rs`, que lee `build.rs`), `src/lock.rs` (inactividad, salida, recarga de la ventana), `src/clipboard/` y `src/webview/` (sus `ffi.rs` son los DOS únicos archivos con `unsafe`; el segundo apaga el menú contextual y las teclas de navegador de WebView2), `src/state.rs`. `tauri.conf.json`: CSP sin `unsafe-*`, ventana en modo privado con autocompletado apagado (WebView2 guardaba lo tecleado: no quitar), centrada, 1120 × 720, NSIS `currentUser`, actualizador. `capabilities/main-window.json`: un `allow-<comando>` por comando, eventos, y nada de `fs`/`shell`/`http`/`dialog`. Pruebas: `tests/commands_contract.rs` (lista, permisos, códigos), `tests/commands_vault.rs` (canario: ningún comando devuelve un valor protegido), `tests/lock_app.rs`, `tests/webview_config.rs`, `tests/webview_surface.rs` y `tests/webview_real.rs` (menú y teclas de navegador apagados).
- `src/` — interfaz React + TS strict (Vite). `ipc/` (cliente tipado, validación con zod sin `eval`, núcleo falso para pruebas), `vault/` (pantallas y máquina de estados), `settings/`, `update/`, `ui/icons.tsx`, `ui/browserSurface.ts` (segunda barrera contra el menú y las teclas de navegador), `styles/` (tokens por rol, tema oscuro). `preview.html` + `src/preview.tsx`: vista previa solo de desarrollo, fuera del bundle.
- Diseño: `DESIGN.md`, `PRODUCT.md`, `.impeccable/surfaces/` (contrato de dirección), `docs/marca/`, `docs/diseno/`.
- `deny.toml` — avisos, licencias compatibles con GPL-3.0, solo crates.io.
- Registrar un comando nuevo toca 3 sitios (`src-tauri/src/commands/mod.rs` lo explica) y las pruebas de contrato fallan si no coinciden.

## Reglas específicas del proyecto
- Versiones fijadas a mano: Tauri **2.x**. `cargo info tauri` y `cargo add tauri` ya devuelven la 3.0 alfa.
- El crate `keepass` se envuelve en el núcleo (lab `docs/lab/kdbx-roundtrip`): historial ordenado de
  viejo a nuevo antes de guardar; KDF propio (Argon2id, nunca `DatabaseConfig::default()`); archivo llave `.keyx`
  v2 generado y validado por nosotros; KDBX 3.1 no se guarda sin convertir.
- Sin stop-hook por ahora: no hay plantilla de `settings` para Rust y no se improvisa.
- La puerta local corre SIEMPRE las dos mitades (Rust e interfaz): hay pruebas de Rust que leen archivos de `src/`.
- La app instalada se prueba con los scripts de `docs/lab/boveda-0.1/qa-ventana/` (UI Automation); el rastro en disco y lo que la persona puede hacer con el ratón y el teclado sobre WebView2 no lo ve ninguna prueba automática (`tests/webview_real.rs` solo comprueba los ajustes de un WebView2 oculto).
- Tiempo fijo por fase (`docs/PROYECTO.md`). Lo que no cabe va a `docs/BACKLOG.md`, no a la fase.
- Revisiones una vez por fase cerrada, nunca por tarea. Bajo/Info a `docs/BACKLOG.md`.

## Definición de terminado
- **Tarea:** su prueba en verde + pruebas afectadas en verde (salida real) + commit.
- **Fase:** puerta de la Fase 6 de `/nuevo-proyecto` (completitud RF → verificación → revisiones proporcionales →
  publicar un release instalable y actualizable desde el anterior).
