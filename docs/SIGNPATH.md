# Solicitud a SignPath Foundation (firma de Windows)

Estado: **preparada, sin enviar.** Enviarla es del dueño del repo (crea una cuenta en SignPath). Decisión de fondo:
`docs/adr/0004-firma-de-codigo.md`.

Condiciones leídas en la fuente el 2026-11-11: <https://signpath.org/terms> (la página sigue marcada como borrador).
Formulario: <https://signpath.org/apply> (no se pudo leer sin navegador: sus campos se confirman al abrirlo).

## Condiciones y cómo estamos
| Condición de SignPath | Estado | Dónde se ve |
|---|---|---|
| Licencia OSI sin doble licencia comercial, en todos los componentes | ✅ | `LICENSE` (GPL-3.0); `cargo deny check` solo admite licencias compatibles |
| Sin componentes propietarios (se permiten bibliotecas del sistema) | ✅ | WebView2 es componente del sistema; no se empaqueta |
| Ya publicado en la forma que se va a firmar | ✅ | Releases `v0.0.1` y `v0.0.2` (instalador NSIS) |
| Mantenido activamente | ✅ | Historial del repo |
| Función descrita en la página de descarga | ✅ | `README.md` y notas de cada release |
| "Code signing policy" en la página de inicio y en la de descargas | ✅ | Sección en `README.md` y enlace en las notas de cada release (`docs/RELEASE_NOTES.md`, desde la próxima versión) |
| Roles con sus miembros (autores, revisores, quien aprueba) | ✅ | `README.md` → "Firma de código" |
| Política de privacidad | ✅ | `README.md` → "Privacidad". Falta revisar si WebView2 obliga a mencionar la política de Microsoft |
| Build verificable y aprobación manual de cada release | ✅ | `.github/workflows/release.yml`, entorno `release`, atestación de procedencia |
| **MFA de todo el equipo, en SignPath y en el repositorio (GitHub)** | ⏳ | Condición previa: se comprueba antes de enviar |
| "Cierta reputación verificable" (no dan un umbral) | ❓ | Proyecto nuevo y sin usuarios: es el motivo más probable de un rechazo |

**Consecuencia:** sin MFA en GitHub la solicitud no se puede enviar. Y aun con todo en regla pueden negarla por
reputación, sin apelación; la salida está en el ADR-0004 (Microsoft Store o certificado OV, que son gasto o cuenta).

## Texto para el formulario
- **Proyecto:** AlmarioKS
- **Repositorio:** https://github.com/Kattonavi/almarioks
- **Licencia:** GPL-3.0-only
- **Descripción (en inglés, como la piden):** *AlmarioKS is a local, open-source password manager for Windows. The
  vault is a standard KDBX 4 file, compatible with KeePass 2 and KeePassXC. No account, no cloud, no telemetry. Built
  with Rust and Tauri 2.*
- **Qué se firma:** el instalador NSIS (`AlmarioKS_<versión>_x64-setup.exe`), `almarioks.exe` y, desde la 0.2, el
  anfitrión de mensajería nativa.
- **Cómo se construye:** GitHub Actions, workflow `release.yml`, solo desde etiquetas `v*` protegidas sobre `main`;
  cada publicación se aprueba a mano en el entorno `release`.
- **Página de descargas:** https://github.com/Kattonavi/almarioks/releases

## Al ser aceptados
1. Agregar a `README.md` y a `docs/RELEASE_NOTES.md` el texto que exigen: *Free code signing provided by
   [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).* No se escribe
   antes: hoy sería falso.
2. Meter la firma en `release.yml` **antes** de `tauri signer sign`: el `.sig` del actualizador se calcula sobre el
   instalador ya firmado (ADR-0004).
3. Comprobar con `signtool verify /pa` y repetir la actualización N → N+1 con un instalador firmado.
