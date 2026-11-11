# ADR-0005: Canal extensión → anfitrión nativo (stdio) → app por named pipe con DACL del usuario
Estado: **propuesto** (MV3 + anfitrión en Rust vienen de la visión como propuesta; se acepta al aprobar el plan) · 2026-11-11 · El detalle va en la spec de la 0.2.

**Contexto:** pilar 3.
- Mensajería nativa (https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging):
  `stdio` con JSON UTF-8 y prefijo de 32 bits; anfitrión → navegador ≤ **1 MB**; `O_BINARY` en Windows; `allowed_origins` con IDs
  exactos, sin comodines; el primer argumento es el origen que llama; clave en `HKCU` o `HKLM`; content script → service worker, que valida `sender`.
- Edge busca primero su propia clave y después la de Chromium y la de Chrome, y **lee solo el primer manifiesto que encuentra**: hay que
  listar los IDs de las dos tiendas, y el ID sin publicar cambia
  (https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging). Brave: sin verificar.
- El DACL por defecto de un named pipe da **lectura a Everyone y a anónimo**
  (https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights). En tokio, los clientes remotos se rechazan
  por defecto y `first_pipe_instance` falla si el nombre ya existe
  (https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/struct.ServerOptions.html).

**Decisión:**
- **Registro:** el NSIS por usuario (ADR-0003) instala el manifiesto `io.github.kattonavi.almarioks.json` junto al anfitrión y crea las
  claves `HKCU` de Chrome y Edge. Al desinstalar, las borra.
- **Canal:** `\\.\pipe\almarioks-<SID>`, creado por la app con `first_pipe_instance(true)`.
  - El DACL solo admite el SID del usuario actual (o el logon SID). Usa `create_with_security_attributes_raw`, que es `unsafe`, así que va
    en `ffi` (R-7). Si el nombre ya existe, la app no escucha y avisa. Sin TCP.
- **Emparejamiento:** la app y la extensión muestran el mismo código y el usuario aprueba en la app. Se guarda una llave de asociación por
  bóveda en sus datos propios, como hace KeePassXC (formato sin verificar), y cada mensaje se autentica con ella. Mientras no hay
  asociación, solo responden `pair` y `status`.
- **El anfitrión verifica al servidor del pipe** (que lo creó un proceso del mismo usuario) antes de enviar nada: con la app cerrada,
  otro usuario local podría crear el nombre primero (AMENAZAS N3). Detalle en la spec 002.
- **App cerrada o bloqueada:** responde `app_not_running` o `locked`; la extensión ofrece abrir o desbloquear. Nada queda en cola.
- **Versión:** todo mensaje lleva `"v": 1` (`almarioks-protocol`). Ante una versión desconocida se responde `unsupported_version`.
- **Origen exacto, en el núcleo:** la extensión envía el origen (esquema, host y puerto) y `almarioks-core::matching` lo compara con la
  entrada. `http` no equivale a `https`, ni un subdominio a su padre. La extensión no decide.

**Por qué es de una vía:** extensión y app se actualizan por separado, y el registro y el nombre del pipe quedan en el instalador.

**Descartadas:** HTTP o WebSocket en localhost (cualquier página lo alcanza) · que el anfitrión abra la bóveda (ADR-0002) ·
`sendNativeMessage` (un proceso por mensaje) · decidir la coincidencia en la extensión.

**Consecuencias:** + nada pasa por la red y el acceso queda limitado al usuario de Windows · − FFI de Win32 que Miri no ejecuta, así que se
prueba con integración · ~ al publicar la extensión cambia su ID y hay que actualizar el manifiesto.

**Cómo lo verificamos:** el primer día de la 0.2 (no en el H0: el anfitrión y sus claves llegan con la actualización 0.1 → 0.2, que
ejecuta el instalador nuevo), que el NSIS registra y borra las claves `HKCU`. En la 0.2, que otro usuario local no abre el pipe, y una
tabla de `matching` (`http`, subdominios, puertos, dominios parecidos).
**Revisar si:** entra Firefox, o un antivirus bloquea el pipe.
