# ADR-0003: Instalador NSIS por usuario y actualizador de Tauri con consentimiento, desde GitHub Releases
Estado: **aceptado** (plan aprobado por el usuario el 2026-11-11)

**Contexto:** pilar 4, T-1, T-6 y E3-8. Almario (su ADR-0005) no descargaba actualizaciones; aquí el usuario lo cambia, porque instalar a
mano deja versiones viejas de un programa que guarda secretos.
- El NSIS instala por defecto **solo para el usuario actual**, sin administrador, en `%LOCALAPPDATA%`, y admite hooks. El MSI pide WiX,
  VBSCRIPT y un instalador por idioma (https://v2.tauri.app/distribute/windows-installer/).
- La firma del actualizador "no se puede desactivar". Si se pierde la llave, no hay más actualizaciones para quien ya instaló, y `.env` no
  sirve para pasarla. `installMode` es `passive` por defecto, y la app se cierra antes de instalar (https://v2.tauri.app/plugin/updater/).
- En GitHub, "latest" es el más reciente que no sea prerelease ni borrador
  (https://docs.github.com/en/rest/releases/releases#get-the-latest-release).

**Decisión:**
- **NSIS `currentUser`** y WebView2 con `downloadBootstrapper`: ni el registro `HKCU` de la mensajería nativa (ADR-0005) ni la actualización
  piden administrador.
- **Canal:** el endpoint es `https://github.com/Kattonavi/almarioks/releases/latest/download/latest.json`, con `pubkey` en `tauri.conf.json` y
  `createUpdaterArtifacts: true`. Releases **0.x sin la marca "prerelease"**: con esa marca no serían "latest" y el actualizador no los
  vería. Que `/latest/download/` siga a "latest" es inferencia; se prueba en el H0.
- **Consentimiento:** la consulta se apaga en Ajustes. Se muestran la versión y las notas, y solo con "Actualizar" se descarga, verifica e
  instala. El instalador corre en modo `passive` (barra de progreso, sin clics, reabre la app): el permiso ya se dio en la
  app, y `basicUi` no reabre la app (fuente de `tauri-plugin-updater` 2.14.1). La capacidad da `allow-check` y `allow-download`, `allow-install`, no `updater:default` (T-3).
- **Custodia:** la llave se genera con `tauri signer generate -w`, protegida con contraseña. (La del proyecto la generó Sara el
  2026-11-11 en el PC del dueño, con contraseña aleatoria, estando él ausente; puede reemplazarla antes del primer release.) La privada y su contraseña viven solo
  en los secretos de GitHub Actions (`TAURI_SIGNING_PRIVATE_KEY`, `…_PASSWORD`) y en un respaldo suyo fuera del repo (su bóveda + una USB).
  Nunca en el disco del runner ni en los logs.
- **Reversa (E3-8):** `version_comparator` no se toca, porque permitir bajar de versión habilita un downgrade malicioso.
  - Quien no ha actualizado: se vuelve a marcar como "latest" el release anterior.
  - Quien ya actualizó: se publica el código de N−1 como N+1.
  - El procedimiento va en `docs/RELEASE.md` y se ensaya en la 1.0.

**Por qué es de una vía:** la `pubkey` y el identificador quedan en cada copia instalada, y una instalación por usuario y otra por máquina
no se actualizan entre sí.

**Descartadas:** MSI (VBSCRIPT, un instalador por idioma) · `perMachine`/`both` (piden administrador) · solo avisar y enlazar (la gente se
queda en versiones viejas) · servidor propio de actualizaciones (un servicio más que mantener).

**Consecuencias:** + sin administrador y con una sola ruta de entrega · − si se pierde la llave, toca reinstalar a mano; GitHub queda como
punto único · ~ los releases 0.x salen sin la marca de prerelease.

**Cómo lo verificamos (H0; el PC es Windows 11 Home y no trae Windows Sandbox: job de GitHub Actions + segundo usuario local):** instalar la 0.0.1 → publicar la 0.0.2 → la app la ofrece → aceptar → abre la
0.0.2. Con un `.sig` alterado se rechaza y la versión no cambia. Desinstalar no borra la bóveda.
**Revisar si:** se pasa a la Microsoft Store (MSIX) o a varios canales (beta y estable).
