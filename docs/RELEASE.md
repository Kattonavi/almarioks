# Cómo se publica una versión

## Una sola vez (dueño del repo)
1. **Hecho (2026-11-11).** Subir la llave del actualizador al entorno `release` de GitHub. La llave está en la carpeta
   `.almarioks-release` de tu usuario, junto con un `LEEME.txt`. Desde la `v0.0.1` esa llave ya no se puede cambiar
   sin el procedimiento de rotación (ADR-0003). Si hay que volver a subirla:

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
2. **Puerta de la 1.0.** Guardar la llave y su contraseña en dos lugares fuera del repo (tu bóveda y una USB).
   GitHub no deja leer un secreto de vuelta: mientras no existan esos respaldos, la carpeta `.almarioks-release` es
   la única copia recuperable y **no se borra**. **Si se pierde, ninguna copia instalada podrá actualizarse nunca
   más** (ADR-0003).
3. **Puerta de la 1.0.** Verificación en dos pasos (2FA) en la cuenta que etiqueta y aprueba los releases.

Las versiones 0.x salen como versiones de desarrollo con los puntos 2 y 3 aplazados por decisión del dueño. No se
publica la 1.0 sin ellos.

## Cada versión
1. PR que sube la versión en `Cargo.toml` (`[workspace.package]`) y `package.json` (deben coincidir) y mueve
   `[Unreleased]` del `CHANGELOG.md` a la versión nueva. Fusionar con CI en verde.
2. Con el CI de `main` en verde, etiquetar **el commit de fusión** por su SHA (no la rama en la que estés):
   `git tag -a vX.Y.Z <sha> -m "AlmarioKS X.Y.Z" && git push origin vX.Y.Z`. Las etiquetas `v*` están protegidas: al
   empujar, GitHub escribe "creations being restricted" y aun así la crea si eres administrador; se confirma con
   `git ls-remote --tags origin`.
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
