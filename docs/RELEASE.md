# Cómo se publica una versión

## Una sola vez (dueño del repo) — **pendiente**
1. Subir la llave del actualizador al entorno `release` de GitHub. La llave se generó el 2026-11-11 (con contraseña
   aleatoria) y está en la carpeta `.almarioks-release` de tu usuario, junto con un `LEEME.txt`. Hay que hacerlo
   desde el PC donde está la llave; no se puede desde el teléfono. Si prefieres usar una llave tuya, cámbiala
   **antes** del primer release (`pnpm tauri signer generate`, y la pública en `src-tauri/tauri.conf.json`): después
   ya no se puede.

   En PowerShell (no uses `<` ni una tubería: PowerShell 5.1 le agrega un salto de línea a la contraseña):
   ```powershell
   $d = "$env:USERPROFILE/.almarioks-release"
   gh secret set TAURI_SIGNING_PRIVATE_KEY --env release --repo Kattonavi/almarioks --body (Get-Content "$d/almarioks-updater.key" -Raw)
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --env release --repo Kattonavi/almarioks --body (Get-Content "$d/almarioks-updater.password.txt" -Raw)
   ```
   En Git Bash:
   ```bash
   gh secret set TAURI_SIGNING_PRIVATE_KEY --env release --repo Kattonavi/almarioks < ~/.almarioks-release/almarioks-updater.key
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --env release --repo Kattonavi/almarioks < ~/.almarioks-release/almarioks-updater.password.txt
   ```
2. Guardar la llave y su contraseña en dos lugares fuera del repo (tu bóveda y una USB) y borrar esa carpeta.
   **Si se pierde, ninguna copia instalada podrá actualizarse nunca más** (ADR-0003).
3. Activar la verificación en dos pasos (2FA) en GitHub.

## Cada versión
1. PR que sube la versión en `Cargo.toml` (`[workspace.package]`) y `package.json` (deben coincidir) y mueve
   `[Unreleased]` del `CHANGELOG.md` a la versión nueva. Fusionar con CI en verde.
2. Etiquetar el commit de `main`: `git tag v0.0.1 && git push origin v0.0.1`. Las etiquetas `v*` están protegidas.
3. El workflow corre en dos trabajos:
   - `build` (sin secretos): exige que el commit esté en `main` y que etiqueta y versiones coincidan (solo `X.Y.Z`),
     corre en modo release las pruebas que exigen HTTPS y firma válida, y compila el instalador **sin firmar**.
   - `publish` (entorno `release`: **hay que aprobarlo en Actions**): firma el instalador atando la firma a la
     versión, comprueba con `almarioks-release-verify` que esa firma valida contra la llave pública que va dentro de
     la app, genera `latest.json` y `SHA256SUMS`, atesta la procedencia y publica el release como "latest" (**sin** la
     marca de prerelease: el actualizador no vería un prerelease).
4. Comprobar: `gh attestation verify <instalador> -R Kattonavi/almarioks` y la prueba de humo (instalar, abrir,
   actualizar desde la versión anterior, desinstalar).

## Reversa (E3-8; se ensaya en la 1.0)
- Quien no ha actualizado: marcar como "latest" el release anterior.
- Quien ya actualizó: publicar el código de N−1 como N+1. Nunca se permite bajar de versión.

## Lo que no se hace
- No se publican instaladores compilados en un PC: solo los que construye `release.yml`.
- No se modifica ni se borra un release publicado: se publica otro.
