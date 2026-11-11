# H0 — evidencia (2026-11-11)

Estado: **H0 a medias.** Todo lo que se puede probar sin publicar está probado. Falta el primer release público, que
depende de tres pasos del dueño del repo (ver `docs/RELEASE.md`).

## Criterios del H0 (`docs/PROYECTO.md`)
| # | Criterio | Estado | Evidencia |
|---|---|---|---|
| 1 | Ida y vuelta de KDBX con KeePass 2 y KeePassXC | ✅ | CI en `main`: `cargo nextest run --workspace --locked`, 6 pruebas (`roundtrip_keepass2_*` ×4, `roundtrip_keepassxc_both`, `wrong_key_is_rejected`) |
| 2 | `release.yml` publica por etiqueta sin pasos manuales | ⏳ | Escrito y con YAML válido; no ha corrido: faltan los secretos de firma en el entorno `release` |
| 3 | Instala por usuario, sin administrador, en máquina limpia | ✅ | Job `installer` de CI en un runner limpio de GitHub: instala en `%LOCALAPPDATA%\AlmarioKS`, clave solo en `HKCU`, ninguna en `HKLM`, la app arranca y sigue viva 10 s. También el ensayo local. Falta solo repetirlo con un usuario estándar sin privilegios (el runner y la consola del ensayo son administradores) |
| 4 | Actualiza N → N+1 con permiso; un paquete alterado se rechaza | 🟡 | Ensayo local completo (abajo) + 5 pruebas automáticas `rf_h0_*` contra el plugin real. Falta repetirlo contra GitHub Releases |
| 5 | Desinstalar no deja programa y no toca las bóvedas | ✅ | Job `installer` de CI (sin carpeta ni clave tras desinstalar) + ensayo local (bóveda del usuario con el mismo SHA-256). Queda la carpeta de datos de WebView2: ver hallazgos |
| 6 | Requisitos [B] de `AMENAZAS.md` §4 | 🟡 | Ver tabla siguiente |

| Requisito [B] del H0 | Estado | Evidencia |
|---|---|---|
| Llave del actualizador con contraseña, solo en el entorno `release`, respaldo en 2 lugares | ⏳ | Llave generada con contraseña, fuera del repo. Subirla a GitHub y respaldarla es del dueño |
| `.sig` alterado, otra llave y `http://` rechazados; sin clic no hay descarga | ✅ | `rf_h0_tampered_artifact_is_rejected`, `rf_h0_signature_from_another_key_is_rejected`, `rf_h0_*http*` (en release) y Vitest "sin clic no hay descarga"; además el ensayo local |
| CSP sin `unsafe-*`, sin devtools en release, capacidades explícitas | ✅ | `src-tauri/tauri.conf.json`, `src-tauri/capabilities/main-window.json`, `AppManifest::commands(&[])` en `build.rs` |
| `release.yml` solo por etiqueta protegida con aprobación; Actions por SHA | ✅ | Ruleset `release-tags` (`v*`), entorno `release` con revisor y solo etiquetas `v*` |
| Lockfiles con `--locked`; `cargo deny check` y `pnpm audit --prod` en verde | ✅ | Job `verify` de CI |
| `SHA256SUMS` y atestación en el release | ⏳ | En `release.yml`; se comprueba con el primer release |

## Ensayo local: instalar → rechazar → actualizar → desinstalar
Instaladores de depuración 0.0.1 y 0.0.2 firmados con una **llave de prueba** desechable y servidos desde
`http://127.0.0.1` (solo las compilaciones de depuración aceptan `http`). La ventana se manejó con UI Automation.
No se publicó nada.

```
antes: carpeta existe = False · claves de desinstalacion = 0
instalador (/S): codigo de salida = 0
  %LOCALAPPDATA%\AlmarioKS\almarioks.exe, uninstall.exe
HKCU clave: AlmarioKS | 0.0.1 | Kattonavi        HKLM claves = 0

-- ventana al abrir --
Versión 0.0.1 · "AlmarioKS solo busca actualizaciones cuando tú lo pides." · [Buscar actualizaciones]
-- clic "Buscar actualizaciones" --
"Hay una versión nueva: 0.0.2." · [Actualizar] [Ahora no]

-- clic "Actualizar" con el instalador servido alterado en 1 byte --
"La firma de la actualización no es válida: no se instaló. Tu versión actual sigue intacta. ..."
version instalada: 0.0.1 · proceso vivo: 1

-- clic "Actualizar" con el instalador bueno --
version en el registro: 0.0.2 · reabierta sola: True · ventana: "Versión 0.0.2"

-- desinstalador (/S) --
carpeta del programa existe = False · clave HKCU existe = False · accesos directos = False
boveda del usuario intacta = True (mismo SHA-256)
queda: %LOCALAPPDATA%\io.github.kattonavi.almarioks (datos de WebView2, 210 archivos, 17.3 MB)
```

## Hallazgos
- **El desinstalador silencioso deja la carpeta de datos de WebView2** (17 MB). En una app que maneja secretos hay que
  decidir si se ofrece borrarla (amenaza W2). Va a la 0.1.
- `basicUi` no reabre la app tras actualizar; se usa `passive` (ADR-0003).
- Las pruebas de integración de Tauri en Windows necesitan incrustar el manifiesto de Common-Controls v6 o el binario
  de prueba aborta con `0xc0000139` (resuelto en `src-tauri/build.rs`, solo para pruebas).
- `keepassxc-cli` no crea KDBX 4 (`db-create` e `import` dan 3.1): ver `docs/lab/kdbx-roundtrip/EXPERIMENTO.md`.

## Tiempo
H0: ~1 h de reloj de las 8 h (más la planeación previa, ~0,5 h). No se disparó el corte de las 4 h.
