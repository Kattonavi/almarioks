# ADR-0002: Fronteras de crates y procesos, secretos fuera de la webview y bloqueo que no falla abierto
Estado: **aceptado** (plan aprobado por el usuario el 2026-11-11)

**Contexto:** E3-3, R-3 y T-3. Todo lo que llega al JS de la webview lo puede leer cualquier script que corra en ella. Los comandos de
`invoke_handler` quedan permitidos "a todas las ventanas" salvo que se limiten con `AppManifest::commands` en `build.rs`
(https://v2.tauri.app/security/capabilities/).

**Decisión:**
- **Crates:** `almarioks-core` (KDBX, dominio, generador; sin Tauri ni red) · `almarioks-protocol` (mensajes versionados, serde puro) ·
  `almarioks-host` (mensajería nativa; depende de `protocol`, **nunca de `core`**) · `src-tauri` (comandos, pipe, portapapeles, `ffi`).
- **La webview recibe:** el árbol de grupos y los metadatos (título, usuario, URL, etiquetas, fechas, nombres de campos y de adjuntos).
  **Nunca recibe:** la contraseña, los campos y adjuntos protegidos ni las llaves. Hay dos excepciones: `reveal_field` (con un gesto del
  usuario) y la salida del generador mientras se muestra. Lo que el usuario escribe viaja al núcleo en un solo `invoke` y el campo se vacía.
- **Copiar y rellenar salen del núcleo.** `copy_field` escribe el portapapeles desde Rust y lo limpia a los N s; el formato para excluirlo
  del historial de Windows está sin verificar. Rellenar va de la app al anfitrión y de ahí al navegador.
- **Instancia única:** `tauri-plugin-single-instance` se registra primero y enfoca la ventana existente. Pide Rust 1.90 o más y no documenta
  su mecanismo en Windows (https://v2.tauri.app/plugin/single-instance/). El pipe (ADR-0005) vive solo ahí.
- **Estados:** `enum VaultState { Closed, Locked { path }, Open { db, .. } }`, con `match` exhaustivo. Al pasar a `Locked` se sueltan `db` y
  las llaves (`ZeroizeOnDrop`), se limpia el portapapeles si lo escribimos nosotros y la webview desmonta la vista con el evento `locked`.
  En `Closed`/`Locked`, todo comando de la bóveda devuelve `Error::Locked`.
- **Bloquear siempre gana:**
  - Cada operación confirmada se guarda al momento (archivo temporal + reemplazo atómico); el formulario a medio editar se descarta.
  - Orden fijo (AMENAZAS W3): (1) el estado pasa a bloqueando y todo comando con secretos responde `Locked`; (2) la vista se desmonta y
    el portapapeles se limpia; (3) si el último guardado falló, se intenta escribir la bóveda cifrada (mismo `save_kdbx4`, misma llave)
    en `<bóveda>.recuperacion.kdbx` **con tiempo límite**; (4) se sueltan `db` y llaves pase lo que pase. Si (3) falla o se cuelga, lo
    pendiente se pierde y se avisa al desbloquear. Nunca se queda abierta esperando un guardado ni una respuesta.

**Por qué es de una vía:** la forma del IPC y de los crates la heredan todas las pantallas, el anfitrión y la 0.2.

**Descartadas:** la webview con el modelo completo (lo hallaría E3-9 en un volcado) · el anfitrión con `core` (no debe poder
abrir la bóveda) · preguntar "¿guardar?" al bloquear (si nadie contesta, queda abierta).

**Consecuencias:** + el JS no puede filtrar lo que nunca tuvo · − más comandos, y cada uno valida sus argumentos (T-4) · ~ si se bloquea
solo, se pierde el borrador del formulario.

**Cómo lo verificamos (CI):**
- `cargo tree -p almarioks-core -i tauri` → vacío (Constitución 8).
- `cargo tree -p almarioks-host -e normal | grep -E "almarioks-core|keepass"` → vacío.
- Canario: 0 apariciones de `CANARIO-…` en las respuestas de los comandos (salvo `reveal_field`) y en el volcado de WebView2 (E3-9).
- E3-3, con y sin cambios pendientes: con el guardado forzado a fallar, el estado queda `Locked`, existe `.recuperacion.kdbx` y el
  portapapeles está limpio.
**Revisar si:** una función de la spec necesita un dato protegido en la webview, o Tauri cambia el modelo de capacidades.
