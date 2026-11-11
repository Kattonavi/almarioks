# ADR-0004: Firma Authenticode con SignPath Foundation, aparte de la firma del actualizador de Tauri
Estado: **aceptado** (plan aprobado por el usuario el 2026-11-11) · El usuario aceptó publicar las 0.x sin Authenticode, marcadas como versiones de desarrollo, y que el editor visible con SignPath sea "SignPath Foundation".

**Contexto:** E1-6 y T-8. Artifact Signing no está abierto a individuos de Colombia (investigación del autor de 2026-11; confirmar en la fuente antes de decidir). EV ya no da reputación
inmediata: se acumula con un firmante constante (https://v2.tauri.app/distribute/sign/windows/). Condiciones de SignPath Foundation
(https://signpath.org/terms, página **en borrador**):
- Licencia OSI sin dual comercial y ningún componente propietario.
- El proyecto ya publicado en la forma que se firmará, mantenido, con "a certain verifiable reputation" (sin umbral).
- Build verificable, aprobación manual de cada release y MFA para todo el equipo.
- En la página de inicio y en la de descargas: "Code signing policy", los roles, el texto "Free code signing provided by SignPath.io,
  certificate by SignPath Foundation" y una política de privacidad con opción para apagar lo que conecta.
- No están obligados a aceptar ni hay apelación. **El certificado sale a nombre de SignPath Foundation.**

**Son dos firmas distintas:**
- **Authenticode** (`signtool`): la que miran SmartScreen y Smart App Control. Va en la app, el anfitrión y el `-setup.exe`.
- **Actualizador** (llave de Tauri, ADR-0003): el `.sig` que la app verifica antes de instalar. Protege aunque falte Authenticode.
- **Orden:** el `.sig` se calcula sobre el instalador **ya firmado**, porque firmar después cambia el archivo. En CI: build → SignPath
  (aprobación manual) → `tauri signer sign <setup.exe>` (existe: https://v2.tauri.app/reference/cli/) → `latest.json` → release.

**Decisión:**
- En el H0 se deja lista la solicitud: README con "Code signing policy", roles, constancia de privacidad y MFA activo. Se envía después del
  primer release público, porque SignPath exige que ya exista.
- Mientras llega: releases 0.x sin Authenticode pero siempre con `.sig`, y aviso de SmartScreen en el README y las notas.
- Si rechazan, o no responden en 60 días: MSIX en la Microsoft Store (la firma Microsoft y no hay SmartScreen; sin verificar aquí) o un certificado OV
  con HSM (150-300 USD/año). Es gasto o cuenta: lo decide el usuario.

**Por qué es de una vía:** la reputación de SmartScreen va con el firmante; cambiarlo la reinicia.

**Descartadas:** Artifact Signing (no aplica desde Colombia) · EV (ya no compra reputación) · certificado autofirmado (Windows no confía en
él).

**Consecuencias:** + gratis y con la llave en HSM · − en las Propiedades del archivo, el editor es **"SignPath Foundation"**, no "Kattonavi"
(el nombre de la app y `publisher` siguen siendo los nuestros) · − cada release se aprueba a mano.

**Cómo lo verificamos:** `signtool verify /pa` sobre la app, el anfitrión y el instalador, y la prueba N → N+1 del ADR-0003 con un
instalador firmado (su `.sig` tiene que validar).
**Revisar si:** SignPath responde o cambia sus condiciones, o Artifact Signing se abre a individuos de Colombia.
