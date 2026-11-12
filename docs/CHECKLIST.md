# Checklist — AlmarioKS · nivel E3 + anexo Rust (R, T) + extensión

Base generada con `scripts/generar-checklist.py --nivel E3` (45 ítems) y **traducida a Tauri/Rust a mano**: el
estándar `escritorio.md` está escrito para .NET. Cada cambio de redacción conserva la intención del ítem.
Un ítem sin evidencia = no cumplido. **Fase** = cuándo se exige como tarde.

Responsables: R = `rust-engineer` · F = `react-next-engineer` · T = `test-engineer` · Q = `ui-qa` ·
S = `security-engineer` · C = `code-reviewer` · P = Sara (hilo principal) · U = usuario.
No hay revisor experto de Rust equivalente a `dotnet-reviewer`: lo suyo lo cubren `cargo deny`, clippy y `code-reviewer`.

## Escritorio E0–E1
| ID | Requisito (traducido) | Fase | Resp. | Evidencia |
|---|---|---|---|---|
| E0-1 | `cargo clippy … -D warnings` y `cargo nextest run` sin errores; `pnpm typecheck` y `pnpm lint` | H0 | R | ✅ CI `verify` en el PR #1 (clippy `-D warnings`, 11 pruebas nextest, typecheck, lint, 10 pruebas Vitest) |
| E0-2 | Versión única en `Cargo.toml` del workspace y `tauri.conf.json`; README con cómo compilar y ejecutar | H0 | R | ✅ versión en `[workspace.package]` + `package.json` (release.yml exige que coincidan); README → "Compilar" |
| E0-3 | El instalador de `pnpm tauri build` corre en otra máquina | H0 | P | ✅ job `installer` de CI en runner limpio |
| E0-4 | Logs locales rotados y sin secretos | 0.1 | R | pendiente |
| E1-1 | H0 cumplido (instala, abre, actualiza, desinstala) | H0 | P | ✅ `v0.0.1` y `v0.0.2` publicadas; instalar, abrir y actualizar 0.0.1 → 0.0.2 probados desde GitHub; desinstalar, en el job `installer` de CI y en el ensayo local (`docs/H0-EVIDENCIA.md`) |
| E1-2 | SemVer 2.0; `CHANGELOG.md` y notas del release tomadas de él | H0 | R | 🟡 `CHANGELOG.md` existe; notas del release en `docs/RELEASE_NOTES.md` |
| E1-3 | Instalador NSIS por usuario + actualizador de Tauri (en vez de Velopack; ADR-0003) | H0 | R | ✅ NSIS `currentUser` + actualizador (config); `latest.json` lo genera `scripts/release-manifest.mjs` en `release.yml` (publicado en `v0.0.1` y `v0.0.2`; la app lo leyó al actualizar) |
| E1-4 | `release.yml` por etiqueta `v*` sin pasos manuales | H0 | R | ✅ dos corridas completas (`v0.0.1`, `v0.0.2`); el único paso humano es la aprobación del entorno `release` |
| E1-5 | `SHA256SUMS` en el mismo release | H0 | R | ✅ en los dos releases; la suma del instalador coincide |
| E1-6 | Decisión de firma en un ADR; sin firma, el README avisa de SmartScreen | H0 | P | ✅ ADR-0004 + aviso en README y `docs/RELEASE_NOTES.md` |
| E1-7 | Actualización con consentimiento y recuperación si falla | H0 | R+T | ✅ 0.0.1 → 0.0.2 desde GitHub con permiso; si la firma falla la versión queda intacta (ensayo local y pruebas `rf_h0_*`) |
| E1-8 | Desinstalación limpia; no borra bóvedas ni datos sin preguntar | H0 | Q | ✅ job `installer` + ensayo local (bóveda intacta). Deja datos de WebView2: backlog |
| E1-9 | "Acerca de": versión, licencia, enlace al changelog, al código y a reporte de errores | 0.1 | F | pendiente |
| E1-10 | Prueba de humo en ventana real tras instalar (abre, crea una entrada, cierra, reabre) | 0.1 | Q | pendiente |
| E1-11 | Accesibilidad: nombre accesible en controles sin texto, teclado completo, foco visible, contraste AA, Narrator | 0.1 básico · 1.0 completo | F+Q | pendiente |
| E1-12 | DPI 100/125/150/200 % y segundo monitor | 1.0 | Q | pendiente |

## Escritorio E2
| ID | Requisito (traducido) | Fase | Resp. | Evidencia |
|---|---|---|---|---|
| E2-1 | Firma Authenticode del instalador y binarios con sello de tiempo; mismo firmante siempre | 1.0 | U+P | ❌ en las 0.x, aceptado por el usuario el 2026-11-11 (SignPath exige un release publicado; las 0.x van marcadas como desarrollo) |
| E2-2 | Actualizaciones solo por HTTPS desde el canal propio, verificadas antes de aplicar | H0 | R+S | ✅ 5 pruebas `rf_h0_*` contra el plugin real + ensayo local con instalador alterado |
| E2-3 | Configuración en el directorio de datos de la app; la app no guarda la contraseña maestra en ningún lado; nada secreto en texto plano ni en logs | 0.1 | R+S | pendiente |
| E2-4 | Carga segura de DLL: sin DLL sueltas junto al ejecutable, instalación en carpeta del usuario no escribible por otros | 1.0 | R+S | pendiente (Process Monitor) |
| E2-5 | Instala por usuario; no pide administrador en uso normal | H0 | R | 🟡 solo `HKCU` y `%LOCALAPPDATA%`; falta probar con usuario estándar |
| E2-7 | `docs/PRIVACIDAD.md` público: no se recolecta nada (lo pide la tienda de extensiones) | 1.0 | U+P | pendiente |
| E2-8 | OWASP Desktop Top 10, una línea por ítem | H0 | S | ✅ `docs/AMENAZAS.md` §3 |
| E2-9 | `cargo deny check` (avisos, licencias, fuentes) y `pnpm audit --prod` limpios; `Cargo.lock` y `pnpm-lock.yaml` en git | H0 | R | ✅ CI `verify`: `cargo deny check` y `pnpm audit --prod`; lockfiles en git |
| E2-10 | Lector de pantalla (Narrator o NVDA) en los flujos principales | 1.0 | Q | pendiente |
| E2-11 | Arranque en frío medido contra la meta del proyecto | 0.1 | T | pendiente |

## Escritorio E3
| ID | Requisito (traducido) | Fase | Resp. | Evidencia |
|---|---|---|---|---|
| E3-1 | Modelo de amenazas STRIDE corto antes del código de seguridad | H0 | S | ✅ `docs/AMENAZAS.md` |
| E3-2 | Cifrado, KDF y formato con librería probada; cero criptografía propia; parches documentados | 0.1 | R+S | parcial: lab 19 (`docs/lab/kdbx-roundtrip/EXPERIMENTO.md`), sin parches |
| E3-3 | Secretos con tipos protegidos (`secrecy`, `zeroize`); el bloqueo oculta la vista y limpia el portapapeles; no falla abierto | 0.1 | R+S | pendiente |
| E3-4 | Pruebas de formato con fixtures reales de KeePass 2 y KeePassXC y fuzzing básico del parser | H0 fixtures · 1.0 fuzzing | T+S | 🟡 H0 ✅: fixtures de KeePass 2.62 (×4) y KeePassXC 2.8.0 en `crates/almarioks-core/tests/fixtures/`, pruebas `roundtrip_*` en CI `verify`. Fuzzing: 1.0 |
| E3-5 | Build repetible (`--locked`, `rust-toolchain.toml`, versiones fijadas) y atestación (`actions/attest`) | H0 atestación · 1.0 build repetible | R+S | 🟡 H0 ✅: `actions/attest-build-provenance` en `release.yml`; `gh attestation verify` salió con 0 en `v0.0.1` y `v0.0.2` (`docs/H0-EVIDENCIA.md`). Build repetible: 1.0 |
| E3-6 | SBOM CycloneDX en el release | 1.0 | R | pendiente |
| E3-7 | `SECURITY.md` con canal de reporte y plazo; parche de Críticos en ≤ 7 días | 1.0 | S+P | pendiente |
| E3-8 | Plan de reversa: republicar una versión anterior como estable | 1.0 | P | pendiente (procedimiento probado) |
| E3-9 | Prueba de intrusión ligera sobre el RC: portapapeles, volcado de memoria, temporales, DLL planting, canal del navegador | 1.0 | S | pendiente |
| E3-10 | Licencias de terceros compatibles con GPL-3.0 y `THIRD-PARTY-NOTICES` | 1.0 | R+C | pendiente |

## Rust (R) y Tauri (T) — `estandares/rust.md`
| ID | Requisito | Fase | Resp. | Evidencia |
|---|---|---|---|---|
| R-1 | `Cargo.lock` en git; CI corre fmt, clippy y pruebas con `--locked` | H0 | R | ✅ CI `verify` |
| R-2 | Bloque de lints del estándar; cada `#[allow]` dice por qué | H0 | R | ✅ `Cargo.toml` del workspace |
| R-3 | Dominio en `almarioks-core` sin Tauri; fechas en UTC, texto por caracteres | H0 | R | ✅ `cargo tree -p almarioks-core -e normal` sin `tauri` |
| R-4 | Sin `unwrap`/`expect`/índice sobre entrada externa; un tipo de error por crate | 0.1 | R | pendiente |
| R-5 | Async sin bloqueos ni candados vivos en `.await`; canales acotados; apagado ordenado | 0.2 | R | pendiente |
| R-6 | `cargo deny check` en CI | H0 | R | ✅ CI `verify` |
| R-7 | `unsafe` solo en `ffi` con `// SAFETY:`. Miri no ejecuta Win32: el permiso del pipe se prueba con integración (otro usuario no abre) | 0.2 | R+S | pendiente |
| R-8 | Pruebas del dominio con propiedades (generador, coincidencia de origen); ningún `#[ignore]` sin tarea | 0.1 | T | pendiente |
| T-1 | H0: instalador que se actualiza de N a N+1 | H0 | R+U | ✅ 0.0.1 → 0.0.2 por GitHub Releases (`docs/H0-EVIDENCIA.md`) |
| T-2 | CSP definida; `devtools` solo en desarrollo | H0 | R | ✅ `src-tauri/tauri.conf.json` (CSP sin `unsafe-*`; sin feature `devtools`) |
| T-3 | Capacidades mínimas por ventana; sin `remote` con comodín ni `shell` abierto | H0 | R+S | ✅ `src-tauri/capabilities/main-window.json` + `AppManifest::commands(&[])` |
| T-4 | Cada comando valida sus argumentos; rutas desde un diálogo del sistema | 0.1 | R | pendiente |
| T-5 | Ningún secreto en archivos de texto ni logs | 0.1 | R+S | pendiente |
| T-6 | Actualizador con `pubkey`, HTTPS y llave privada solo en secretos de CI, con respaldo fuera del repo | H0 | R+U | 🟡 `pubkey` y HTTPS en config; llave verificada con simulacro y subida al entorno `release`; el respaldo fuera del repo es puerta de la 1.0 |
| T-8 | Firma de código en Windows | 1.0 | U | pendiente (ADR-0004) |
| T-9 | Ningún comando síncrono hace disco pesado (el KDF corre fuera del hilo de la interfaz) | 0.1 | R | pendiente |

## Extensión de navegador (X) — propios del proyecto; se afinan con `docs/AMENAZAS.md`
| ID | Requisito | Fase | Resp. | Evidencia |
|---|---|---|---|---|
| X-1 | Manifest V3 con permisos mínimos; sin código remoto | 0.2 | F+S | pendiente |
| X-2 | La extensión no guarda secretos (ni en `storage` ni en memoria más allá del gesto) | 0.2 | F+S | pendiente |
| X-3 | El anfitrión nativo solo acepta el ID de la extensión (`allowed_origins`) y el canal local solo al usuario actual | 0.2 | R+S | pendiente |
| X-4 | Emparejamiento con aprobación del usuario en la app | 0.2 | R | pendiente |
| X-5 | Rellenar y guardar exigen gesto del usuario y origen exacto, resuelto en el núcleo; pruebas con dominios parecidos, `http` y subdominios | 0.2 | T+S | pendiente |
| X-6 | Con la app bloqueada o cerrada no se entrega nada: se pide desbloquear | 0.2 | T | pendiente |
| X-7 | Lista fija de 20 sitios: 16 o más detectados y guardados bien | 0.2 | T | pendiente |

## Diseño y UX (nativo)
| Requisito | Fase | Resp. | Evidencia |
|---|---|---|---|
| Mock de la pantalla clave aprobado por el usuario antes de construir interfaz | antes de la 0.1 | Impeccable + U | pendiente |
| Matriz pantalla × estado de `docs/UX.md` con cada celda construida | 0.1 | F+Q | pendiente |
| Texto escalable al 150 % y movimiento reducido respetado | 1.0 | Q | pendiente |
| Evaluación heurística de Nielsen sin críticos abiertos | 1.0 | Q | pendiente |

## CI/CD
| Requisito | Fase | Resp. | Evidencia |
|---|---|---|---|
| PR: fmt + clippy + nextest + `cargo deny` + typecheck + lint + test, en ≤ 10 min | H0 | R | ✅ job `verify` de `ci.yml` corre todos esos pasos; las corridas de PR del 2026-11-11 tardaron entre 2,5 y 3,5 min |
| `main` protegida (PR + CI verde) desde que exista CI | H0 | P | ✅ ruleset `main` activo: PR obligatorio, checks `verify`, `secrets` e `installer` requeridos, sin borrado ni `force push` (`gh api repos/Kattonavi/almarioks/rulesets`) |
| Actions fijadas por SHA; `permissions: contents: read` salvo el job de release; gitleaks | H0 | R | ✅ ningún `uses:` sin SHA de 40 caracteres; `contents: read` en los dos workflows y permisos de escritura solo en el job `publish`; job `secrets` con gitleaks, requerido en `main` |
| Conventional Commits; etiqueta `v*` → `release.yml` | H0 | R | ✅ el historial sigue Conventional Commits (por convención: no hay verificación automática); `release.yml` solo corre con etiquetas `v*`, protegidas por el ruleset `release-tags` |

## No aplica
- **E2-6** (telemetría y reportes de fallos opt-in): no hay telemetría ni reportes automáticos (pilar 1). Si algún día se agregan, vuelve a aplicar.
- **T-7** (móvil): no hay móvil antes de la 1.0.
- **Bloques A (API) y F (FFI)** de `rust.md`: no hay servidor; FFI solo aplica si aparece un módulo `ffi` (entonces R-7).
- **Railway, SEO, legal de comercio:** app de escritorio sin servidor ni sitio.
