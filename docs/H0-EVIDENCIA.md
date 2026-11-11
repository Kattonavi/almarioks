# H0 — evidencia (2026-11-11)

Estado: **H0 cumplido en lo técnico.** `v0.0.1` y `v0.0.2` están publicadas por CI y la actualización de una a otra
se probó contra GitHub Releases. Quedan abiertos: la prueba con un usuario estándar de Windows, y el respaldo de la
llave y el endurecimiento de la cuenta, que el dueño aplazó a la 1.0 (`docs/RELEASE.md`).

## Criterios del H0 (`docs/PROYECTO.md`)
| # | Criterio | Estado | Evidencia |
|---|---|---|---|
| 1 | Ida y vuelta de KDBX con KeePass 2 y KeePassXC | ✅ | CI en `main`: `cargo nextest run --workspace --locked`, 6 pruebas (`roundtrip_keepass2_*` ×4, `roundtrip_keepassxc_both`, `wrong_key_is_rejected`) |
| 2 | `release.yml` publica por etiqueta sin pasos manuales | ✅ | Dos corridas completas: `v0.0.1` y `v0.0.2` (abajo). El único paso humano es la aprobación del entorno `release`, que es a propósito |
| 3 | Instala por usuario, sin administrador, en máquina limpia | ✅ | Job `installer` de CI en un runner limpio de GitHub: instala en `%LOCALAPPDATA%\AlmarioKS`, clave solo en `HKCU`, ninguna en `HKLM`, la app arranca y sigue viva 10 s. También el ensayo local. Falta solo repetirlo con un usuario estándar sin privilegios (el runner y la consola del ensayo son administradores) |
| 4 | Actualiza N → N+1 con permiso; un paquete alterado se rechaza | ✅ | Actualización real 0.0.1 → 0.0.2 desde GitHub Releases (abajo). El rechazo del paquete alterado está probado en el ensayo local y en las pruebas `rf_h0_*` contra el plugin real, no contra GitHub (no se publica un release malo a propósito) |
| 5 | Desinstalar no deja programa y no toca las bóvedas | ✅ | Job `installer` de CI (sin carpeta ni clave tras desinstalar) + ensayo local (bóveda del usuario con el mismo SHA-256). Queda la carpeta de datos de WebView2: ver hallazgos |
| 6 | Requisitos [B] de `AMENAZAS.md` §4 | 🟡 | Ver tabla siguiente: todo cumplido salvo el respaldo de la llave, aplazado a la 1.0 por el dueño |

| Requisito [B] del H0 | Estado | Evidencia |
|---|---|---|
| Llave del actualizador con contraseña, solo en el entorno `release`, respaldo en 2 lugares | 🟡 | Llave con contraseña, fuera del repo y subida solo al entorno `release` (2026-11-11); los dos releases se firmaron con ella en CI y la app instalada aceptó la firma. **Respaldo en 2 lugares: aplazado a la 1.0 por decisión del dueño** |
| `.sig` alterado, otra llave y `http://` rechazados; sin clic no hay descarga | ✅ | `rf_h0_tampered_artifact_is_rejected`, `rf_h0_signature_from_another_key_is_rejected`, la de `http://` (solo falla en modo release: corrió así en local y está como paso de `release.yml`; el CI de PR la corre en depuración) y Vitest "sin clic no hay descarga"; además el ensayo local |
| CSP sin `unsafe-*`, sin devtools en release, capacidades explícitas | ✅ | `src-tauri/tauri.conf.json`, `src-tauri/capabilities/main-window.json`, `AppManifest::commands(&[])` en `build.rs` |
| `release.yml` solo por etiqueta protegida con aprobación; Actions por SHA | ✅ | Ruleset `release-tags` (`v*`), entorno `release` con revisor y solo etiquetas `v*` |
| Lockfiles con `--locked`; `cargo deny check` y `pnpm audit --prod` en verde | ✅ | Job `verify` de CI |
| `SHA256SUMS` y atestación en el release | ✅ | En los dos releases: la suma del instalador coincide y `gh attestation verify` sale con 0 |

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

## Ensayo del trabajo de publicación (sin publicar)
Los pasos de `publish` de `release.yml`, corridos en local sobre el instalador de release sin firmar:

```
tauri signer sign --app-version 0.0.1 <instalador>     -> .sig con "file:AlmarioKS_0.0.1_x64-setup.exe  version:0.0.1"
almarioks-release-verify tauri.conf.json <inst> <sig> 0.0.1 -> OK ... signed for version 0.0.1 with the pubkey of src-tauri/tauri.conf.json (exit 0)
almarioks-release-verify ... 0.0.2                      -> FAIL: the signature was made for version 0.0.1, expected 0.0.2 (exit 1)
node scripts/release-manifest.mjs ...                   -> latest.json (windows-x86_64, URL del release v0.0.1) y SHA256SUMS
```
Lo que sigue sin ejecutarse nunca: el workflow en GitHub (subida y bajada de artefactos entre trabajos, atestación y
`gh release create`).

## Primer release público: `v0.0.1` (2026-11-11)
Etiqueta sobre el commit de fusión del PR #1 con el CI de `main` en verde. Corrida de `release.yml`:
`https://github.com/Kattonavi/almarioks/actions/runs/43093696078`.

- **Intento 1:** `build` pasó pruebas en modo release, instalador y verificador, y falló al subir el artefacto
  (`Failed to CreateArtifact … is not valid JSON`) durante una caída de GitHub Actions. No se firmó ni se publicó nada.
- **Intento 2** (`gh run rerun --failed`): `build` completo; `publish` esperó la aprobación del dueño en el entorno
  `release` y luego firmó, verificó la firma contra la `pubkey` de la app, atestó y publicó.

Comprobado después, desde otro PC que el runner:
```
gh release view v0.0.1          -> draft=false prerelease=false · setup.exe, setup.exe.sig, latest.json, SHA256SUMS
releases/latest/download/latest.json -> version 0.0.1 · windows-x86_64 · url .../download/v0.0.1/AlmarioKS_0.0.1_x64-setup.exe
sha256sum del instalador        -> e129abb9…5f11, igual que SHA256SUMS
gh attestation verify <instalador> -R Kattonavi/almarioks -> exit 0
almarioks-release-verify tauri.conf.json <instalador> <sig> 0.0.1 -> OK ... signed for version 0.0.1 (exit 0)

instalador (/S): codigo de salida = 0
HKCU: AlmarioKS | 0.0.1 | Kattonavi      HKLM claves: 0
-- ventana al abrir --            Versión 0.0.1 · [Buscar actualizaciones]
-- clic "Buscar actualizaciones" -- "Tienes la versión más reciente."   (consulta real a GitHub por HTTPS)
```

## Segundo release y actualización real: `v0.0.2` (2026-11-11)
PR #4 (solo sube la versión) fusionado con CI en verde; etiqueta sobre su commit de fusión. Corrida:
`https://github.com/Kattonavi/almarioks/actions/runs/43106208709` (un solo intento, `build` y `publish` completos).

```
gh release list                 -> v0.0.2 Latest · v0.0.1
releases/latest/download/latest.json -> version 0.0.2 · url .../download/v0.0.2/AlmarioKS_0.0.2_x64-setup.exe
sha256sum del instalador        -> dd031c26…63c1, igual que SHA256SUMS
gh attestation verify <instalador> -R Kattonavi/almarioks -> exit 0
```
Con la 0.0.1 de GitHub instalada y abierta (ventana manejada con UI Automation):
```
antes: registro = 0.0.1
-- clic "Buscar actualizaciones" --
"Hay una versión nueva: 0.0.2." · "Novedades de la versión 0.0.2" · [Actualizar] [Ahora no]
-- clic "Actualizar" --
registro tras actualizar = 0.0.2 · reabierta sola: True (proceso nuevo) · ventana: "Versión 0.0.2"
HKLM claves: 0
```

## Lo que sigue sin probar
- **Usuario estándar de Windows.** Todas las instalaciones (runner de GitHub, consola del ensayo, este PC) corrieron
  con una cuenta administradora. El instalador es por usuario y no escribe en `HKLM`, pero falta verlo con una cuenta
  sin privilegios. Crear esa cuenta en el PC es del dueño.
- **Paquete alterado servido desde GitHub.** Solo en local y en pruebas automáticas.

## Revisiones del lote
`docs/revisiones/h0.md`: una ronda de `code-reviewer` y una de `security-engineer`, sin Críticos ni Altos; los Medios
se corrigieron en el mismo PR.

## Hallazgos
- **El desinstalador silencioso deja la carpeta de datos de WebView2** (17 MB). En una app que maneja secretos hay que
  decidir si se ofrece borrarla (amenaza W2). Va a la 0.1.
- `basicUi` no reabre la app tras actualizar; se usa `passive` (ADR-0003).
- Las pruebas de integración de Tauri en Windows necesitan incrustar el manifiesto de Common-Controls v6 o el binario
  de prueba aborta con `0xc0000139` (resuelto en `src-tauri/build.rs`, solo para pruebas).
- `keepassxc-cli` no crea KDBX 4 (`db-create` e `import` dan 3.1): ver `docs/lab/kdbx-roundtrip/EXPERIMENTO.md`.

## Tiempo
H0: ~2,5 h de trabajo de las 8 h (más la planeación previa, ~0,5 h), sin contar las esperas por las aprobaciones del dueño ni la caída de GitHub Actions. No se disparó el corte de las 4 h.
