# Plan — Spec 001 Bóveda

Sin código. Decisiones de una vía: ADR-0001 y ADR-0002. Lo aprendido antes de implementar: `docs/lab/boveda-0.1/`.

## Módulos y quién los escribe
| Parte | Dónde | Qué hace | RF | Dueño |
|---|---|---|---|---|
| Envolturas KDBX | `almarioks-core::kdbx` | Abrir, crear, guardar a bytes, KDF, `.keyx`, política 3.1, orden del historial. Único módulo que nombra tipos de `keepass` | 1–6, 10–17, 25, 26 | rust-engineer |
| Bóveda en memoria | `almarioks-core::vault` | `VaultState { Closed, Locked, Open }`; operaciones de entradas y grupos sobre tipos propios; papelera | 35, 38–58 | rust-engineer |
| Guardado | `almarioks-core::storage` | Temporal → relectura → reemplazo atómico → `.bak`; detección de cambio externo; archivo de recuperación | 21–24, 27, 33 | rust-engineer |
| Búsqueda | `almarioks-core::search` | Normalización (mayúsculas, tildes) y orden | 59–63 | rust-engineer |
| Generador y fortaleza | `almarioks-core::generator`, `::strength` | CSPRNG del sistema, sin sesgo; escala 0–4 | 7, 65–70 | rust-engineer |
| Ajustes | `almarioks-core::settings` | Tipos y validación; leer y escribir un archivo fuera de la bóveda | 19, 20, 82 | rust-engineer |
| Comandos | `src-tauri::commands` | Un comando por operación; valida argumentos; convierte errores a `{ code, message }` | 76, 79, 80, 83 | rust-engineer |
| Bloqueo | `src-tauri::lock` | Orden de 4 pasos, temporizador de inactividad, evento `locked` | 28–37 | rust-engineer |
| Portapapeles | `src-tauri::clipboard` + `ffi` | Copia desde Rust, exclusión de historial y nube, limpieza condicional | 31, 71–75 | rust-engineer |
| Interfaz | `src/` | Pantallas P1–P8 de `docs/UX.md`, sin dirección visual; solo metadatos | 7–9, 18, 20, 28, 32, 64, 74, 78, 83 | react-next-engineer |

Regla de dependencia: `core` no conoce Tauri ni Win32. Lo que necesita el sistema (reloj, portapapeles, disco con
fallos) entra al núcleo como parámetro o `trait`, para probar RF-23, RF-27, RF-30 y RF-33 sin trucos.

## Contrato entre la ventana y el núcleo
Todos los comandos devuelven `Result<T, AppError>` con `AppError = { code: string, message: string }`; `code` es uno
de los códigos de la spec. Los identificadores de grupos y entradas son el UUID en texto. Los argumentos rechazan
campos desconocidos (`deny_unknown_fields`, RF-80). Nombres en `snake_case`.

| Comando | Entra | Sale | RF |
|---|---|---|---|
| `vault_status` | — | `{ state, path?, name?, read_only, weak_kdf, save_pending, recovery }` | 34, 35 |
| `vault_create` | `{ path, name, password?, key_file }` con `key_file` = `none` \| `existing{path}` \| `generate{path}` | estado | 1–6, 9 |
| `vault_open` | `{ path, password?, key_file_path? }` | estado | 10–17 |
| `vault_lock` · `vault_close` | — | — | 28, 30–33, 36 |
| `vault_convert` | `{ new_path }` | estado | 15 |
| `vault_retry_save` · `vault_save_as{path}` · `vault_reload` | | estado | 23, 24 |
| `pick_vault_to_open` · `pick_vault_to_create` · `pick_key_file` · `pick_new_key_file` | — | ruta o nada (el diálogo lo abre Rust) | 1, 10, 20 |
| `recent_list` · `recent_remove{path}` | | `[{ path, exists }]` | 19, 20 |
| `password_strength` | `{ password }` | `{ score }` | 7, 70 |
| `group_tree` | — | árbol `{ id, name, entry_count, kind, children }` con `kind` = `root` \| `normal` \| `recycle_bin` | 50 |
| `group_create{parent_id,name}` · `group_rename{id,name}` · `group_move{id,parent_id}` | | árbol | 51–55, 57 |
| `group_delete_preview{id}` · `group_delete{id}` · `recycle_bin_empty` | | `{ entries, subgroups }` · árbol | 56–58 |
| `entry_list{group_id}` | | `[EntrySummary]` | 42 |
| `entry_get{id}` | | `EntryDetail` (sin valores protegidos) | 43 |
| `entry_create{group_id,…}` · `entry_update{id,…}` | campos; en `update` es un parche: campo ausente = no cambiar, presente = reemplazar | `EntryDetail` | 38–41, 45, 91 |
| `entry_move{id,group_id}` · `entry_delete{id}` | | — | 46–48 |
| `entry_reveal{id,field}` | | `{ value }` | 44 |
| `entry_copy{id,field}` | | `{ clears_in_seconds }` | 71–74 |
| `entry_open_url{id}` | | — | 49 |
| `entry_history{id}` | | versiones (recortable) | 85 |
| `search{query}` | | `[EntrySummary + ruta del grupo]` | 59–64 |
| `generate_password{length,lower,upper,digits,symbols,avoid_lookalikes}` | | `{ password, score }` | 65–70 |
| `settings_get` · `settings_set{auto_lock_minutes,clipboard_seconds}` | | ajustes | 29, 73, 82 |
| `activity` | — | — (reinicia el temporizador de inactividad) | 29 |

Eventos del núcleo a la ventana: `vault-locked` (desmontar todo, RF-32) y `vault-save-state` (`saved` \| `failed` \|
`external-change`).

`EntrySummary = { id, title, username, url, tags, modified }`.
`EntryDetail = EntrySummary + { group_id, notes, protected_fields: [nombres con valor], custom_fields:
[{ name, protected, value? }], attachments: [{ name, size }], created }`. Un campo no protegido trae su valor; uno
protegido, nunca: en `EntrySummary` y `EntryDetail` un campo estándar protegido viaja como `null` y la ventana
muestra "(protegido)".

## Precisiones del contrato (salieron al construir la interfaz, T17; la capa `src/ipc/` ya las usa)
1. Todo comando con datos recibe **un solo parámetro `args`**: un struct en `snake_case` con `deny_unknown_fields`.
   La ventana llama `invoke(cmd, { args })`. Con parámetros sueltos Tauri espera `camelCase` y descarta en silencio
   los campos que sobran, que es justo lo que RF-80 prohíbe.
2. `state` vale `closed` | `locked` | `deriving` | `open`. `locked` y `deriving` llevan `path`; `open` lleva `name`,
   `read_only`, `weak_kdf`, `save_pending` y `recovery`.
3. `recovery` = `null` | `{ kind: "saved", path }` | `{ kind: "lost" }` (RF-34).
4. `key_file` = `{ kind: "none" }` | `{ kind: "existing", path }` | `{ kind: "generate", path }`.
5. Códigos nuevos: `ArchivoNoEncontrado` y `ArchivoLlaveNoEncontrado` (RF-95).
6. El `field` de `entry_reveal` y `entry_copy` es el nombre que el núcleo lista en `protected_fields` o `custom_fields`.
7. `search` devuelve además `group_path: string[]`.
8. Cada versión de `entry_history`: `{ index, modified, title, username, url, tags, notes }`.
9. La capacidad de la ventana necesita `core:event:allow-listen` (para `vault-locked`) y el permiso de cada comando
   nuevo; los comandos que derivan la llave son `async` (T13, T14).
10. Las respuestas que no cuadran con el esquema dan `RespuestaInvalida` en la ventana; un código que la ventana no
    conoce da `Inesperado`. El texto del núcleo nunca se muestra: el mensaje sale del mapa de la ventana (RF-83).
11. `entry_count` cuenta solo las entradas directas del grupo. `group_path` incluye la raíz. `modified` y `created`
    son ISO 8601 en UTC.
12. Orden de validación de `vault_create`: `ArgumentoInvalido` › `LlaveVacia` › `ArchivoExiste` (bóveda) ›
    `ArchivoExiste` (archivo llave) › escribir.
13. El evento `vault-save-state` vale `saving` | `saved` | `failed` | `external-change`.
14. **Para la revisión de seguridad de la fase:** `password_strength` recibe la contraseña maestra en cada pausa de
    escritura (RF-7), además del `vault_create` final. El ADR-0002 dice "un solo `invoke`". El valor ya está en el
    campo de la ventana y el núcleo lo envuelve y lo borra; la alternativa descartada era un estimador en JavaScript.
    Que `security-engineer` diga si se acepta o se mide solo al salir del campo.

## Concurrencia y bloqueo (RF-30, RF-33, RF-88 a RF-90)
- Un solo dueño del estado: todas las operaciones de la bóveda, la copia al portapapeles y el bloqueo pasan por el
  mismo candado, en orden.
- Guardar se parte en dos: **cifrar a bytes** (con candado, rápido, necesita la llave) y **escribir a disco** (sin
  candado, puede fallar o colgarse, solo ve bytes cifrados). Por eso bloquear nunca espera al disco.
- Mientras un guardado no se confirma, el núcleo conserva esos bytes cifrados: son lo que se escribe en el archivo
  de recuperación después de soltar la bóveda y las llaves (D12).

## Decisiones con la alternativa descartada
- **Los diálogos de archivo los abre Rust**, no la ventana: así la ventana no necesita permisos de `dialog` ni `fs`.
  Descartado: plugin de diálogo llamado desde JS (más permisos en la capacidad).
- **La fortaleza se calcula en el núcleo** (`password_strength`), no con una librería en la ventana. Descartado:
  `zxcvbn` en JS (otra dependencia npm con la maestra en la mano).
- **La inactividad la mide Rust** con un aviso `activity` que la ventana manda como mucho una vez cada 10 s.
  Descartado: temporizador en JS (un script colgado dejaría la bóveda abierta).
- **Bloquear recarga la webview** además de desmontar la vista (AMENAZAS W2).

## Estrategia de pruebas
- Núcleo: pruebas `rf_NN_*` con archivos en carpetas temporales y las fixtures de KeePass 2 y KeePassXC; propiedades
  con semilla fija para el generador (RF-66, RF-69) y la búsqueda (RF-60); un disco de prueba que falla o se cuelga
  para RF-23, RF-27, RF-30 y RF-33.
- Comandos: prueba de contrato con un canario en todos los campos protegidos; ninguna respuesta lo contiene salvo
  `entry_reveal` y `generate_password` (RF-76). `invoke` a un comando no listado falla (RF-79).
- Interfaz: Vitest por pantalla y estado de la matriz de `docs/UX.md`; fixture XSS pintada (RF-78).
- Validación independiente (`/especificar validar`): pruebas escritas desde la spec por un agente que no vio la
  implementación, para los RF de seguridad (bloqueo, portapapeles, guardado, secretos fuera de la ventana).
- Compatibilidad: la bóveda creada aquí se abre con KeePassLib 2.62 y `keepassxc-cli` (script del lab).
