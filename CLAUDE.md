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
| Rust: formato / lints / pruebas | `cargo fmt --check` · `cargo clippy --all-targets --all-features --locked -- -D warnings` · `cargo nextest run --workspace --locked` |
| Dependencias | `cargo deny check` |
| Interfaz: tipos / lint / pruebas | `pnpm typecheck` · `pnpm lint` · `pnpm test` |
| Instalador | `pnpm tauri build` |

## Mapa del código
- `Cargo.toml` — workspace: versión única (`[workspace.package]`, de ahí la toma Tauri), lints, perfil release; `rust-toolchain.toml` fija 1.99.1.
- `crates/almarioks-core/` — núcleo sin Tauri ni red; `tests/roundtrip.rs` + `tests/fixtures/keepass2/` (datos falsos).
- `src-tauri/` — app Tauri 2.12: `tauri.conf.json` (CSP, NSIS `currentUser`), `capabilities/main-window.json` (solo `core:app:allow-version`), `build.rs` (`AppManifest::commands`).
- `src/` — interfaz React + TS strict (Vite); pruebas `*.test.tsx` con Vitest.
- `deny.toml` — avisos, licencias compatibles con GPL-3.0, solo crates.io.
- Restricción del núcleo: `cargo tree -p almarioks-core -e normal --prefix none` sin `tauri`.

## Reglas específicas del proyecto
- Versiones fijadas a mano: Tauri **2.x**. `cargo info tauri` y `cargo add tauri` ya devuelven la 3.0 alfa.
- El crate `keepass` se envuelve en el núcleo (lab `docs/lab/kdbx-roundtrip`): historial ordenado de
  viejo a nuevo antes de guardar; KDF propio (Argon2id, nunca `DatabaseConfig::default()`); archivo llave `.keyx`
  v2 generado y validado por nosotros; KDBX 3.1 no se guarda sin convertir.
- Sin stop-hook por ahora: no hay plantilla de `settings` para Rust y no se improvisa.
- Tiempo fijo por fase (`docs/PROYECTO.md`). Lo que no cabe va a `docs/BACKLOG.md`, no a la fase.
- Revisiones una vez por fase cerrada, nunca por tarea. Bajo/Info a `docs/BACKLOG.md`.

## Definición de terminado
- **Tarea:** su prueba en verde + pruebas afectadas en verde (salida real) + commit.
- **Fase:** puerta de la Fase 6 de `/nuevo-proyecto` (completitud RF → verificación → revisiones proporcionales →
  publicar un release instalable y actualizable desde el anterior).
