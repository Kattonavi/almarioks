# Spec 001 — Validación en la ventana real (T28)

> 2026-11-11 · Compilación de depuración `0.0.2` de la rama `feat/boveda`, instalada con el instalador NSIS y manejada
> con UI Automation: clic real de ratón y tecleo real (`SendInput`), como una persona. Solo datos falsos.
> Es la primera vez que la interfaz (React) y el núcleo (Rust) corren juntos.
> Scripts: `docs/lab/boveda-0.1/qa-ventana/` · Capturas (`.png`) y volcados del árbol UIA (`.uia.txt`):
> `.impeccable/review/qa/`, fuera de git. Las capturas muestran la ruta de la bóveda de prueba con el nombre de
> usuario de Windows: no se suben al repo.

## Veredicto
- **Hito "crear → abrir → guardar una entrada en la ventana real": cumplido.** El flujo principal de `docs/UX.md`
  corrió de corrido sin errores y la bóveda resultante abre en KeePass 2.62 con todo su contenido.
- **Matriz de completitud: 12 de 12 celdas pasan**, hechas con el ratón y verificadas releyendo el archivo.
- **La fase no puede cerrarse todavía: hay 1 defecto Crítico y 1 Alto**, los dos de integración y los dos en el mismo
  sitio: WebView2 guarda en disco, fuera de la bóveda, lo que se teclea en los formularios (D-1 y D-2). Ninguna
  prueba automática podía verlo: solo aparece con la app instalada.

## Entorno
- Windows 11, pantalla 1920 × 1080 a escala 125 %. WebView2 157. Ventana por defecto: interior de 800 × 600 px lógicos.
- Carpeta de trabajo: `%LOCALAPPDATA%\Temp\almarioks-qa-20261111` (borrada al terminar).
- Instalado: `%LOCALAPPDATA%\AlmarioKS\almarioks.exe`. Es la compilación de `target\debug`: mismo tamaño
  (32 870 400 bytes) y solo 3 bytes distintos, los de la marca `__TAURI_BUNDLE_TYPE_VAR_UNK` → `…_NSS` que el
  empaquetador de Tauri escribe en el binario. La versión anterior instalada medía 11 503 616 bytes.
- KeePass 2.62 instalado (se usó KeePassLib, solo lectura). KeePassXC no está en el PC: no se instaló.

## Resultado por paso

### Preparación
| Paso | Resultado | Evidencia |
|---|---|---|
| Cerrar la instancia que estaba abierta (cierre normal de ventana, sin leerla) | pasa (inferido) | `01-instalar.ps1` le mandó el cierre normal, pero su comprobación reventó por un error del script; que cerró se deduce de que el instalador reemplazó el exe |
| Instalador `/S` termina con código 0 | pasa | 7 s; `almarioks.exe` y `uninstall.exe` en `%LOCALAPPDATA%\AlmarioKS` |
| Clave de desinstalación solo en `HKCU` | pasa | `HKCU: AlmarioKS \| 0.0.2` |
| Nada en `HKLM` | pasa | 0 claves de desinstalación; `reg query HKLM\SOFTWARE /s /k /f almarioks` → 0 claves |
| La app abre en la bienvenida | pasa | `A01-bienvenida-primer-arranque.png` · primer arranque tras instalar: 7 993 ms hasta leer "Bienvenida" por UIA; los cuatro siguientes: ventana a los 130–190 ms y bienvenida legible ~500 ms después |

### A. Flujo principal (de corrido)
| Paso | Resultado | Evidencia |
|---|---|---|
| Bienvenida → "Crear bóveda nueva" | pasa | `A02-crear-vacia.png`, `A02-crear-vacia.uia.txt` |
| Elegir la ruta con el diálogo nativo "Crear una bóveda" | pasa | `A03-dialogo-nativo-guardar.png`; la ruta aparece en el formulario y el diálogo no crea el archivo |
| Medidor de fortaleza (RF-7) | pasa | `123456` → "muy débil (0 de 4)" y "Es una contraseña débil. Puedes continuar…"; la maestra de prueba → "muy buena (4 de 4)" · `A04`, `A05` |
| Crear la bóveda, con progreso (RF-1, RF-18) | pasa | 8 979 ms; se vio "Creando la bóveda: derivando la llave…" · `A06-crear-derivando.png`, `A07-boveda-recien-creada.png`; `qa-uno.kdbx` de 795 bytes en disco |
| Bóveda vacía con acción (UX P4) | pasa | "Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardas la primera cuenta." |
| Nueva entrada con contraseña generada (RF-38, RF-65, RF-70) | pasa | Generador: longitud 20, "muy buena (4 de 4)" · `A08-formulario-con-generador.png`, `A09` |
| Guardar la entrada | pasa, con D-4 | "Entrada guardada." a los 1 609 ms · `A10-entrada-guardada.png`; después en disco: `qa-uno.kdbx` 1 054 bytes, `qa-uno.kdbx.bak` 795 bytes, sin `.tmp` |
| Buscar por parte del título (RF-59, RF-60) | pasa | `titulo-corr` encuentra "CANARIO-QA-Titulo-Correo" · `A11-busqueda-con-resultado.png` |
| Copiar la contraseña: el portapapeles real tiene el valor (RF-71, RF-74) | pasa | 20 caracteres con los cuatro tipos; la ventana dice "Copiado: se borra en 20 s" · `A12` |
| Lo copiado va marcado para no entrar al historial (RF-72, parte automática) | pasa | Formatos: `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`, `CanUploadToCloudClipboard` |
| El portapapeles se limpia solo (RF-73, ejemplo 1a) | pasa | Vacío a los 20,8 s de copiar · `A13-portapapeles-limpio.png` |
| Otra app copia antes: se respeta (RF-73, ejemplo 1b) | pasa | A los 6,7 s otro proceso copia un texto; a los 26 s sigue ahí |
| Mostrar y ocultar la contraseña (RF-44) | pasa | El valor mostrado es el copiado; al ocultar sale del árbol UIA |
| Escape limpia la búsqueda (RF-92) | pasa | Vuelve "Entradas de qa-uno" |
| Bloquear (RF-28, RF-30) | pasa | Pantalla de desbloqueo a los ~300 ms · `A14-bloqueada.png` |
| Contraseña mala (RF-11) | pasa | "La contraseña o el archivo llave no abren esta bóveda. Revisa los dos y vuelve a intentarlo." a los 1,3 s · `A15`. (El script lo marcó FALLA porque buscaba la palabra "incorrecta"; el texto es correcto) |
| Desbloquear con la contraseña con `ñ` y `€` (RF-10) | pasa | 1 181 ms, con progreso visible · `A16-desbloqueada.png` |
| La entrada sigue ahí | pasa | "CANARIO-QA-Titulo-Correo" en la lista tras desbloquear |

### B. Matriz de completitud
Cada celda se hizo con el ratón en la ventana y se verificó después: en el árbol de la ventana, tras bloquear y
desbloquear (relee el archivo) y, al final, abriendo el archivo con KeePass 2 (sección D).

| Entidad | Crear | Ver | Editar | Mover | Borrar | Borrado definitivo |
|---|---|---|---|---|---|---|
| **Entradas** | pasa | pasa | pasa | pasa | pasa | pasa |
| **Grupos** | pasa | pasa | pasa (renombrar) | pasa | pasa | pasa (vaciar papelera) |

| Celda | Qué se hizo | Evidencia |
|---|---|---|
| Entrada · crear | 5 entradas en la raíz y en subgrupos, con contraseña generada y escrita (con `ñ` y `€`) | `A10`, `b1`, `b3`: "Entrada guardada." y el contador del grupo sube |
| Entrada · ver | Detalle con usuario, URL, etiquetas, notas y contraseña oculta; el valor no está en el árbol UIA (RF-43) | `B03-entrada-detalle.png` |
| Entrada · editar | Solo el título: la contraseña sigue igual (portapapeles) y, visto con KeePass 2, el historial gana una versión con el título viejo. Confirmar sin cambios: "No había cambios que guardar." y el archivo no se reescribe (ejemplo 8) | `B04-formulario-editar.png`, `z-historial-y-limpieza.ps1` |
| Entrada · mover | "Mover a otro grupo…" → destino "qa-uno › Personal QA" elegido en la lista con el ratón → "«…» se movió a Personal QA."; raíz 1 → 0, Personal QA 0 → 1 (RF-46) | `B05-dialogo-mover-entrada.png` |
| Entrada · borrar | "Borrar entrada" → "«…» se movió a la papelera. Para recuperarla, ábrela en la papelera y muévela a otro grupo."; raíz 1 → 0, papelera 0 → 1 (RF-47). Corrió en `b2`; verificado en el árbol de la ventana, no releyendo el archivo | `B10-entrada-a-la-papelera.png` |
| Entrada · borrado definitivo | En la papelera: "Borrar definitivamente…" → aviso "…se elimina para siempre y no se puede deshacer", foco inicial en "Cancelar" → "Se borró «…» de forma definitiva."; papelera 1 → 0 (RF-48). Corrió en `b2`, donde el script lo marcó FALLA por un comparador mal escrito. En disco cuadra: el `.bak` anota 6 objetos borrados (esta entrada + 3 entradas y 2 grupos del vaciado de `b2`) y el archivo final 10 (+ 2 entradas y 2 grupos de `b3`) | `B11-dialogo-borrar-definitivo.png`, sección D |
| Grupo · crear y ver | "Trabajo", "Trabajo › Clientes", "Personal", "Archivo": aparecen en el árbol con su contador (RF-50, RF-51) | `B01-grupo-creado.png`, `B01-arbol.uia.txt` |
| Grupo · renombrar | "Personal" → "Personal QA": "Grupo renombrado." (RF-53). Nombre vacío: "El grupo necesita un nombre. Escribe uno." (RF-52) | `B02-dialogo-renombrar.png`, `F03-grupo-nombre-vacio.png` |
| Grupo · mover | "Personal QA" dentro de "Archivo": "«Personal QA» se movió a «Archivo»." (RF-54) | `B08-grupo-movido.png`, `B08-arbol-tras-mover.uia.txt` |
| Grupo · mover a su subgrupo | La ventana no lo permite: para "Trabajo" solo ofrece "qa-uno › Personal QA" y "qa-uno › Archivo"; ni él ni "Clientes" salen como destino. No hay mensaje (ver D-7) | `B06-dialogo-mover-grupo.png` |
| Grupo · borrar | Aviso "«Trabajo» contiene 2 entradas y 1 subgrupo. Al borrarlo, todo pasa a la papelera y desde ahí se puede recuperar.", foco inicial en "Cancelar" → "«Trabajo» se movió a la papelera." (RF-56, ejemplo 4) | `B09-dialogo-borrar-grupo.png` |
| Papelera · vaciar | Aviso "Se borrarán de forma definitiva 2 entradas y 2 subgrupos. No se puede deshacer." → "La papelera quedó vacía." (RF-58) | `B12-dialogo-vaciar-papelera.png` |
| Releer | Tras bloquear y desbloquear: raíz 0 · Archivo 0 · Personal QA 1 · Papelera 0; la búsqueda `canario` da solo la entrada que quedó; su contraseña con `ñ` y `€` sale igual (RF-41) | `B14-releido.uia.txt`, `B14-releido-busqueda.png` |

La primera corrida de esta parte (`b2-mover-borrar.ps1`) eligió mal el destino: el `<select>` no expone sus opciones
por UIA hasta desplegarse y el script dejó la primera. La app hizo lo que se le pidió (movió la entrada a "Trabajo",
avisó "3 entradas y 1 subgrupo" y, al vaciar, "3 entradas y 2 subgrupos": cuentas correctas). Se rehízo el escenario
y se repitió con `b3-mover-borrar-bis.ps1`, que es la evidencia de la tabla.

### C. Archivo llave y archivos malos
| Paso | Resultado | Evidencia |
|---|---|---|
| Crear una segunda bóveda con "Generar un archivo llave nuevo" (RF-5) | pasa | 6 638 ms hasta "Bóveda creada"; `qa-dos.keyx` de 213 bytes, XML versión 2.0 con `Hash` · `C01`, `C02` |
| Aviso de recuperación (RF-9) | pasa | "Sin una copia de ese archivo y sin la contraseña no hay forma de recuperarla. Guarda ahora una copia del archivo llave en otro lugar seguro, por ejemplo una memoria USB." · `C03-boveda-creada-aviso.png` |
| Cerrar y abrir con contraseña + archivo llave (RF-10) | pasa | "Elegir archivo llave…" → diálogo nativo → abre y la entrada está · `C04`, `C06` |
| Contraseña buena sin archivo llave; contraseña mala con archivo llave (RF-11, ejemplo 7) | pasa | El mismo texto en los dos casos, un solo mensaje en pantalla, y el archivo llave elegido sigue puesto tras el fallo · `C05-llave-incorrecta-con-archivo.png` |
| Archivo de texto como bóveda (RF-13) | pasa | "El archivo de la bóveda está dañado o no es una bóveda. Prueba con una copia, por ejemplo el archivo .bak que está a su lado." La app sigue viva · `C08-archivo-danado.png` |
| Archivo de 0 bytes (RF-13) | pasa | Mismo mensaje; la app sigue |

### D. Persistencia y compatibilidad
| Paso | Resultado | Evidencia |
|---|---|---|
| Tras cerrar la app: `.kdbx`, `.kdbx.bak` y ningún `.tmp` ni recuperación | pasa | `e-bloqueo.ps1` (D1) y `g2-minimo.ps1` (cierre con la bóveda abierta) |
| Cabecera KDBX 4 | pasa | `03 D9 A2 9A 67 FB 4B B5 01 00 04 00` |
| KeePass 2.62 (KeePassLib) abre `qa-uno.kdbx` con la contraseña con `ñ` y `€` | pasa | Ve `qa-uno › Archivo › Personal QA` y la entrada con título, usuario, URL, notas, etiqueta y la contraseña exacta; papelera activa; 10 objetos borrados anotados |
| KDF de la bóveda nueva (RF-2) | pasa | Argon2id, 64 MiB, 49 iteraciones, 2 hilos; cifrado ChaCha20 |
| El `.bak` abre con la misma llave y trae el estado anterior (RF-22) | pasa | Contiene todavía "Papelera › Trabajo › Clientes" con sus 2 entradas: es el estado de antes de vaciar |
| KeePass 2.62 abre `qa-dos.kdbx` con contraseña + `.keyx`, y la rechaza sin el archivo | pasa | Ve "CANARIO-QA-ConLlave"; sin archivo llave: `InvalidCompositeKeyException` |
| Historial tras editar (RF-26, ejemplo 8) | pasa | KeePass 2: título nuevo, historial = 1 con el título viejo, contraseña igual en ambos |
| Tras reiniciar la app, la bóveda reciente y su contenido siguen (RF-19) | pasa | "Abrir qa-uno.kdbx" en la bienvenida · `G01-bienvenida-con-reciente-800x600.png` |
| KeePassXC 2.8 | **no probado** | `keepassxc-cli.exe` no está en `%ProgramFiles%\KeePassXC`, `%LOCALAPPDATA%\Programs`, la carpeta de laboratorio del Workspace, `docs\lab`, Descargas ni `%TEMP%`. No se instaló |

### E. Bloqueo
| Paso | Resultado | Evidencia |
|---|---|---|
| "Bloquear" deja la pantalla de desbloqueo (RF-28) | pasa | ~300 ms · `A14-bloqueada.png` |
| Bloqueada, la ventana no expone datos por UIA (RF-32) | pasa | Abierta, el barrido ve los 5 canarios (`E00-abierta.uia.txt`, control positivo). Bloqueada: 37 nodos, 0 canarios y sin la contraseña (`E01-bloqueada.uia.txt`). Solo queda el nombre y la ruta del archivo |
| Al bloquear se vacía el portapapeles (RF-31) | pasa | Con la contraseña recién copiada: longitud 0 tras bloquear |
| Bloqueo automático a 1 minuto desde Ajustes (RF-29) | pasa | Se bloqueó sola a los **54,7 s** de la última entrada real, con una contraseña mostrada en pantalla; después, 0 canarios en el árbol · `E02-bloqueo-automatico.png`, `E02-bloqueo-automatico.uia.txt`. Los ~5 s de adelanto salen de que la ventana avisa la actividad como mucho cada 10 s |
| La cuenta "Se bloquea en" | **FALLA** | D-3 |
| Cerrar la app con una contraseña recién copiada (RF-75) | pasa | Portapapeles vacío tras el cierre normal, con la bóveda bloqueada y con la bóveda abierta |
| "Cerrar bóveda" vuelve a la bienvenida (RF-36) | pasa | `c-f-llave-y-errores.ps1` (F3) |
| Suspensión (ejemplo 10) y bloqueo con la sesión de Windows (RF-37) | no probado | Suspender o bloquear la sesión corta la automatización |

### F. Estados y errores visibles
Todos en español, dicen qué pasó y qué hacer, sin códigos.

| Estado | Resultado | Texto que se ve · evidencia |
|---|---|---|
| Bóveda vacía | pasa | "Esta bóveda todavía no tiene entradas. Con «Nueva entrada» guardas la primera cuenta." · `A07` |
| Grupo vacío y papelera vacía | pasa | "Este grupo no tiene entradas." · "La papelera está vacía." |
| Búsqueda sin resultados (RF-64) | pasa | "Sin resultados." + "No hay entradas que coincidan con «…»" + botón "Limpiar búsqueda" · `F02-busqueda-sin-resultados.png` |
| Título vacío al guardar (RF-39) | pasa | "La entrada necesita un título. Escribe uno." y el foco vuelve al campo · `F01-titulo-vacio.png` |
| Contraseñas que no coinciden (RF-8) | pasa | "La contraseña maestra y su confirmación no coinciden. Escríbelas otra vez." y el foco va a la contraseña · `F04` |
| Crear donde ya existe un archivo (RF-4) | pasa, con D-8 | "Ya hay un archivo con ese nombre en esa carpeta y no se tocó. Elige otro nombre u otra carpeta." El archivo queda idéntico (SHA-256) · `F05-archivo-ya-existe.png` |
| Bóveda reciente que ya no existe (RF-20) | pasa | "qa-dos.kdbx ya no está en …. Puede que se haya movido o borrado." + "Buscar qa-dos.kdbx…" + "Quitar qa-dos.kdbx de la lista"; quitar deja la lista vacía · `F06-reciente-que-no-existe.png`, `Z02-recientes-quitadas.png` |
| No se pudo guardar · el archivo cambió en disco · KDBX 3.x en solo lectura · KDF débil | no probado | Piden provocar fallos de disco o traer bóvedas de otra app; quedan cubiertos solo por las pruebas automáticas |
| Abrir la URL de una entrada (RF-49) | no probado | Abriría el navegador en un sitio externo |

### G. Ventana y diseño
| Paso | Resultado | Evidencia |
|---|---|---|
| Capturas a 800 × 600 | hecho | Bienvenida `A01`, `G01` · desbloquear `G02` · bóveda con detalle `G04` · formulario con generador `A08`, `G05` · diálogo de borrado `G07` · ajustes `G06` |
| Capturas a 480 × 360 | hecho | `G01`, `G02`, `G03`, `G04`, `G04b`, `G05`, `G06`, `G07` con sufijo `-480x360`. La barra de grupos pasa a iconos y la lista cede el sitio al detalle; sin desborde horizontal. A ese tamaño se sigue pudiendo copiar una contraseña con el ratón |
| Maximizada | hecho | `G04-boveda-detalle-maximizada.png` (interior 1536 × 793 lógicos): ningún control pide desplazarse |
| Mínimo de la ventana | pasa | Arrastrando la esquina con el ratón se detiene en 480 × 360 (`G11`). Con `SetWindowPos` desde otro proceso sí acepta 300 × 200 (no es una vía de usuario; hay capturas `-300x200`) |
| Fuente | pasa | Se ve Overpass y Overpass Mono en las 17 capturas revisadas a ojo (de 88), en los tres tamaños; los `.woff2` van dentro del paquete (`'self'`) |
| Teclado: desbloquear | pasa | Bienvenida: Tab → "Crear bóveda nueva" → "Abrir una bóveda…" → recientes. En Desbloquear el foco cae solo en la contraseña; Tab → "Elegir archivo llave…" → "Desbloquear" → "Volver a la bienvenida" → "Ajustes y Acerca de". Enter en el campo desbloquea. El foco se ve (doble anillo) · `G08`, `G09-foco-desbloquear.png` |
| Teclado: copiar | pasa, con D-5 | Ctrl+K lleva a la búsqueda; al resultado se llega con 9 Tab; Enter lo abre; 3 Tab → "Copiar contraseña"; Enter copia. El foco se ve · `G10-foco-copiar.png` |
| Controles tapados | D-6 | A 800 × 600 con 4 grupos o más, las acciones del grupo quedan cortadas al pie de la barra y "Guardado." se pinta encima · `B06-dialogo-mover-grupo.png`, `fallo-B9-Grupo-mover-valido-.png` |
| Abrir la app dos veces (RF-81) | pasa | Un solo proceso y una sola ventana de Tauri; la segunda instancia termina sola y la ventana existente queda al frente |

### H. Seguridad observable
Buscador: cada canario y la contraseña maestra, en UTF-8 y en UTF-16LE, dentro de cada archivo. Control positivo:
encontró un canario plantado en cada codificación.

| Dónde | Resultado | Evidencia |
|---|---|---|
| `%LOCALAPPDATA%\io.github.kattonavi.almarioks` (datos de WebView2, 298 archivos, 70,7 MB) | **FALLA** | `EBWebView\Default\Web Data` contiene en claro título, usuario, URL, notas y etiqueta, y un derivado de la contraseña maestra: D-1 y D-2 |
| `%APPDATA%\io.github.kattonavi.almarioks` (ajustes) | pasa | Solo `settings.json`, sin canarios |
| `settings.json` | pasa | Solo `auto_lock_minutes`, `clipboard_seconds` y `recent_vaults` (rutas) |
| Carpeta de trabajo: bóvedas, `.bak`, `.keyx`, salida del proceso | pasa | 0 apariciones. La salida estándar y de error de la app: 0 bytes en todas las corridas |
| `%TEMP%`, archivos escritos durante la prueba | pasa | 0 apariciones (se excluye `%TEMP%\claude`, donde están los registros de esta misma QA) |
| Contraseña maestra y contraseñas de entradas **en claro** | pasa | 0 apariciones en todas las carpetas |

## Tiempos medidos
| Qué | Tiempo | Nota |
|---|---|---|
| Arranque: ventana visible | 130–190 ms | Cuatro arranques en caliente; la bienvenida es legible por UIA ~500 ms después. El primero tras instalar tardó 7 993 ms hasta leer "Bienvenida", contaminado por el primer enlace de UIA y por el perfil nuevo de WebView2. **La meta de ≤ 2 s en frío queda sin medir** |
| Crear bóveda (clic → bóveda abierta) | 8 979 ms | Incluye calibrar Argon2 en este equipo (RF-2). La segunda, en otro arranque de la app y con archivo llave: 6 638 ms hasta "Bóveda creada" |
| Desbloquear (clic → bóveda abierta) | 1 181 ms | Meta ≤ 1,5 s |
| Contraseña mala (clic → mensaje) | 1 260 ms | |
| Bloquear (clic → pantalla de desbloqueo) | ~300 ms | Meta ≤ 1 s |
| Guardar entrada: clic → "Entrada guardada." | 1 609 · 1 629 · 1 628 · 1 676 · 1 702 ms | |
| Guardar entrada: clic → archivo reemplazado y "Guardado." | 2 422 · 2 402 · 2 460 · 2 450 ms | El archivo se reemplaza ~0,8 s después del mensaje: D-4 |
| Limpieza del portapapeles | 20,8 s | N = 20 |
| Bloqueo automático | 54,7 s | Ajuste: 1 minuto |

Los tiempos incluyen la espera fija del clic del script (~200 ms) y el sondeo por UIA (~50 ms).

## Defectos

### D-1 · Crítico · WebView2 guarda en disco un derivado rápido de la contraseña maestra
- **Qué hice:** crear una bóveda escribiendo la contraseña maestra y su confirmación; cerrar la app; buscar en la
  carpeta de datos de WebView2.
- **Qué esperaba:** RF-77 (ningún secreto en disco, tampoco en la carpeta de WebView2) y RF-2: la única forma de
  probar una contraseña maestra es pagar la derivación Argon2id.
- **Qué pasó:** `%LOCALAPPDATA%\io.github.kattonavi.almarioks\EBWebView\Default\Web Data` (SQLite), tabla
  `autofill_edge_field_values`, guarda para el campo "Repite la contraseña maestra" (`is_masked = 1`) un número que
  es **los primeros 8 bytes de SHA-1 de lo tecleado (UTF-8), sin sal**. Comprobado con los dos valores que escribí
  ahí (la maestra de prueba y la confirmación distinta del caso "no coinciden"): los dos coinciden. Quien copie esa
  carpeta puede descartar candidatas a contraseña maestra a velocidad de SHA-1, sin tocar Argon2id.
- **Cómo repetirlo:** instalar, crear una bóveda, cerrar la app, abrir `Web Data` con SQLite:
  `select value, is_masked from autofill_edge_field_values` y comparar con SHA-1 de la contraseña.
- **Nota:** el campo "Contraseña maestra" de crear y el de desbloquear no aparecen; solo el de confirmación.
  Visto en la compilación de depuración; la de release no se comprobó (lleva el mismo motor y los mismos formularios).
- **Arreglo sugerido (sin verificar):** apagar el autorrelleno de WebView2 (`IsGeneralAutofillEnabled` y
  `IsPasswordAutosaveEnabled` en falso) o abrir la webview en modo InPrivate (`incognito` de Tauri), y borrar el
  `Web Data` que ya exista al actualizar. Agregar la búsqueda de canarios sobre la app instalada a la puerta de release.

### D-2 · Alto · WebView2 guarda en claro el contenido de los formularios de la bóveda
- **Qué hice:** el recorrido normal: crear entradas y grupos, buscar.
- **Qué esperaba:** RF-77 y su prueba ("búsqueda del canario… en la carpeta de WebView2, sobre la app instalada").
- **Qué pasó:** el mismo archivo y tabla guardan en claro (`is_masked = 0`), asociados a su etiqueta ("Título",
  "Usuario", "URL", "Notas", "Etiquetas", "Nombre del grupo", "Buscar en la bóveda"): títulos de entradas,
  usuarios, la URL, las notas, las etiquetas, los nombres de los grupos y un texto de búsqueda. No quedó todo lo
  tecleado (falta, por ejemplo, el título "CANARIO-QA-Desechable" y casi todas las búsquedas), pero lo que queda sobrevive a bloquear, cerrar la app y
  borrar la entrada: los títulos de dos entradas ya eliminadas siguen ahí. Las contraseñas de las entradas no
  aparecen.
- **Cómo repetirlo:** crear una entrada con todos los campos, cerrar la app y buscar el título dentro de `Web Data`.
- **Por qué Alto y no Crítico:** no son campos protegidos, pero las notas suelen llevar códigos de recuperación y
  todo esto debía vivir solo dentro del archivo cifrado. Mismo arreglo que D-1.

### D-3 · Medio · La cuenta "Se bloquea en" no cuenta ni sigue el ajuste
- **Qué hice:** dejar la ventana sin tocar y leer la cuenta; cambiar el tiempo de bloqueo en Ajustes y volver a leerla.
- **Qué esperaba:** una cuenta que baja y que usa el tiempo vigente (RF-29; `docs/UX.md`: "se bloquea sola").
- **Qué pasó:** muestra siempre el valor de la última consulta de estado. 24 s sin tocar: "5:00 → 5:00 → 5:00". Con
  el ajuste recién guardado en 1 minuto siguió diciendo "5:00" y la bóveda se bloqueó a los 55 s; al volver a 5
  minutos siguió diciendo "1:00". El bloqueo real sí respeta el ajuste.
- **Cómo repetirlo:** abrir una bóveda, no tocar nada 30 s y mirar la barra; o cambiar el tiempo en Ajustes.
- **Riesgo:** la persona cree que le quedan 5 minutos y pierde un formulario a medio llenar.

### D-4 · Medio · "Entrada guardada." aparece antes de que el archivo esté guardado
- **Qué hice:** guardar una entrada cronometrando el mensaje y la fecha del archivo.
- **Qué esperaba:** RF-21 y D7: cada operación confirmada se guarda; `docs/UX.md` P5 "Guardada".
- **Qué pasó:** "Entrada guardada." sale a los ~1,65 s y el archivo se reemplaza a los ~2,45 s (5 medidas). Durante
  esos ~0,8 s la ventana dice a la vez "Entrada guardada." y "Guardando…", y en disco solo existe el `.tmp`.
- **Cómo repetirlo:** `b1-crear-editar.ps1`, paso B4.
- **Arreglo sugerido:** que el mensaje de la entrada espere al guardado, o que diga "Entrada creada" y deje
  "Guardando… / Guardado." como única verdad sobre el disco.

### D-5 · Medio · Con teclado, la tarea principal no son 3 pasos
- **Qué hice:** Ctrl+K, escribir, y probar Enter y flecha abajo en el campo de búsqueda.
- **Qué esperaba:** `docs/UX.md`: "buscar y copiar una contraseña: 3 pasos (atajo de búsqueda, escribir, copiar)".
- **Qué pasó:** Enter y flecha abajo no hacen nada. Para llegar al primer resultado hay que pasar con Tab por
  "Bloquear", "Ajustes", todos los grupos y las acciones del grupo (9 Tab con 4 grupos; crece con cada grupo),
  luego Enter, 3 Tab y Enter.
- **Cómo repetirlo:** `g-ventana.ps1` paso G2 y `g2-minimo.ps1` paso G8.

### D-6 · Bajo · Las acciones del grupo se cortan y "Guardado." se pinta encima
- A 800 × 600 con 4 grupos o más, "Mover grupo…" y "Borrar grupo…" quedan cortadas al pie de la barra izquierda y el
  texto "Guardado." se dibuja sobre la última. Un clic en el centro del botón medio visible no lo acciona; hay que
  desplazar la barra. `B06-dialogo-mover-grupo.png`, `fallo-B9-Grupo-mover-valido-.png`.
- A 480 × 360 pasa lo mismo con el último icono de la barra (`G04-boveda-detalle-480x360.png`), y la cuenta de
  bloqueo queda como "5:00" sin su rótulo.

### D-7 · Bajo · Mover un grupo a su propio subgrupo: se impide sin explicarlo
- La lista de destinos omite el grupo y sus subgrupos, así que RF-55 se cumple por prevención, pero nada dice por qué
  faltan. El rechazo `MovimientoInvalido` del núcleo no se puede provocar desde la ventana (queda cubierto solo por
  las pruebas automáticas). Si se quiere el mensaje del ejemplo 3, una línea de ayuda en el diálogo basta.

### D-8 · Bajo · Dos respuestas opuestas al elegir un archivo que ya existe
- El diálogo nativo pregunta "qa-uno.kdbx ya existe. ¿Quieres reemplazarlo?"; al responder "Sí", la app contesta
  "Ya hay un archivo con ese nombre… y no se tocó". El resultado es correcto (RF-4), pero la persona acaba de decir
  que sí. Quitar la pregunta de reemplazo del diálogo de guardar dejaría un solo mensaje.

### Observaciones (no son defectos)
- Guardar tarda ~2,4 s por operación: con 49 iteraciones de Argon2 y la relectura de RF-21, cada guardado deriva la
  llave dos veces. No hay meta en la spec; conviene que el dueño lo vea.
- El bloqueo automático llegó ~5 s antes del minuto (la ventana avisa actividad como mucho cada 10 s). Va por el lado seguro.
- En el árbol UIA, el nombre de cada elemento de la lista de grupos junta el de todos sus hijos
  ("qa-uno·1entradaTrabajo·2entradas…"). Para la revisión con lector de pantalla de la 1.0.
- La compilación de depuración es de subsistema de consola (`main.rs`): abierta desde el menú Inicio mostrará una
  ventana de consola. No pasa en release.

## Red
`Get-NetTCPConnection` por PID en 7 momentos del recorrido. **Ninguna conexión no local de `almarioks.exe`** en
ninguno (el script no desglosa por proceso las locales). Sus procesos hijos `msedgewebview2.exe` tuvieron de 0 a 3 conexiones a `:443` (150.171.27.11, 52.96.109.226,
52.96.165.194, 52.96.173.194). No pulsé "Buscar actualizaciones". Son conexiones propias del motor WebView2, que la
spec deja fuera de lo que la app controla (RF-84, "Cómo se prueba"); se anotan. No se inspeccionó su contenido.

## Heurísticas de Nielsen
| Heurística | Pantalla | Severidad | Evidencia |
|---|---|---|---|
| Visibilidad del estado del sistema | P4 Bóveda (cuenta de bloqueo) | Medio | D-3 |
| Visibilidad del estado del sistema | P5 Entrada (guardado) | Medio | D-4 |
| Flexibilidad y eficiencia de uso | P4 Búsqueda con teclado | Medio | D-5 |
| Prevención de errores | P7 Grupo (mover) | Bajo | D-7: previene bien, no explica |
| Consistencia y estándares | P2 Crear (archivo existente) | Bajo | D-8 |
| Estética y diseño minimalista | P4 Barra de grupos a 800 × 600 | Bajo | D-6 |
| Ayuda a reconocer y recuperarse de errores | P1, P2, P3, P5, P7 | sin hallazgos | Todos los mensajes de error leídos (sección F y C) dicen qué pasó y qué hacer, sin códigos |
| Control y libertad del usuario | Diálogos destructivos | sin hallazgos | Foco inicial en "Cancelar"; la papelera permite deshacer |

## Lo que no se probó
KeePassXC (no está en el PC) · Win+V a mano (RF-72, parte manual) · suspensión y bloqueo de sesión de Windows ·
estados de guardado fallido y cambio externo · KDBX 3.x y KDF débil · abrir URL · ejemplos 1c y 1d del portapapeles ·
doble clic en "Desbloquear" (RF-88) · opciones del generador ("evitar parecidos", 0 tipos) · datos extremos
(títulos larguísimos, emoji, miles de entradas) · alto contraste, otras escalas y lector de pantalla · una cuenta
estándar de Windows.

## Límites del método
- Los `<select>` y los diálogos nativos se manejan por teclado cuando UIA no expone el control (queda anotado
  "ALTERNATIVA" en la salida de los scripts).
- La comprobación "lo que hay bajo el puntero es el control" dio falsos avisos en la barra superior; solo se
  reporta como defecto lo que además se ve en una captura (D-6).
- La búsqueda en disco ve texto en UTF-8 y UTF-16LE; no ve nada dentro de archivos comprimidos de la caché de WebView2.

## Estado en que quedó el PC
- Instalada y cerrada la compilación de depuración `0.0.2` de `feat/boveda`. **No escribir en ella ninguna
  contraseña real** (D-1), además del aviso que la propia app ya muestra.
- Carpeta de trabajo borrada. Recientes de prueba quitadas desde la app ("Quitar … de la lista"). Ajustes en 5
  minutos y 20 s, como estaban. `settings.json` ahora existe (antes no) con la lista de recientes vacía.
- **Queda** en `%LOCALAPPDATA%\io.github.kattonavi.almarioks\EBWebView\Default\Web Data` lo descrito en D-1 y D-2,
  solo con datos falsos de esta prueba. No se borró: es la evidencia. Para limpiarlo, borrar esa carpeta `EBWebView`
  con la app cerrada.
- Portapapeles vacío.
- *Actualización:* ese `Web Data` se borró en la reverificación de abajo, que deja instalada otra compilación.

## Reverificación de D-1 y D-2 (modo privado)

> 2026-11-11, misma noche · Compilación de depuración nueva (instalador de las 21:42) cuyo cambio es
> `"incognito": true` en la ventana de `src-tauri/tauri.conf.json` (WebView2 en modo InPrivate).
> Scripts: `rv-lib.ps1`, `rv1-preparar-y-recorrer.ps1`, `rv2-inspeccion.ps1`, `rv3-comprobar-y-limpiar.ps1` y el
> inspector `rv-inspeccionar.py`. Solo se reverificaron D-1 y D-2; D-3 a D-8 siguen como estaban.

**Veredicto: D-1 corregido · D-2 corregido**, en esta compilación de depuración y con el recorrido mínimo de abajo.

| Paso | Resultado | Evidencia |
|---|---|---|
| Lo que había antes de borrar | anotado | `EBWebView`: 297 archivos, 69,5 MB. Su `Web Data`: 12 tablas `autofill*` con 34 filas (`autofill_edge_field_values` = 24: 22 en claro y 2 enmascaradas; `autofill_edge_field_client_info` = 10) |
| **Control positivo** sobre una copia de ese `Web Data` | pasa | El mismo inspector encuentra el título y las notas de la primera corrida en claro y las dos filas enmascaradas, y las identifica como el SHA-1 de 8 bytes de la maestra anterior y de su repetición distinta |
| **Control positivo** con los canarios nuevos | pasa | Archivo plantado: ve un canario en UTF-8, otro en UTF-16LE y el SHA-1 de 8 bytes de la maestra nueva en decimal |
| Borrar solo `EBWebView` e instalar `/S` | pasa | Código 0; el exe instalado es el de `target\debug` (igual salvo la marca NSIS) y ya no es el de la primera corrida; `tauri.conf.json` trae `"incognito": true` |
| Recorrido con canarios nuevos (`CENTINELA-RV-…`) | hecho | Crear bóveda tecleando la maestra, una repetición distinta (rechazada: "no coinciden") y la repetición correcta; entrada con título, usuario, URL, notas, etiqueta y contraseña; grupo; dos búsquedas con Enter; cambio de ajustes; bloquear, desbloquear y cierre normal |
| `Web Data` tras el recorrido | existe, vacío | 227 328 bytes; sus 12 tablas `autofill*` tienen **0 filas** (antes 34). Ningún campo recordado |
| Carpeta de datos de WebView2 completa | 0 apariciones | 133 archivos, 5,9 MB, 13 bases SQLite (`Web Data`, `History`, `Login Data`, `Cookies`, `Favicons`…): ningún canario en claro (UTF-8 ni UTF-16LE) y ninguna forma del SHA-1 de la maestra o de su repetición (8 bytes en decimal, con signo, hex, crudos y al revés; SHA-1 completo). Repetido tras un segundo arranque y cierre: 152 archivos, 0 apariciones, 0 filas |
| Otros sitios | 0 apariciones | Carpeta de ajustes, carpeta de trabajo (bóveda, `.bak`, salida del proceso) y 163 archivos de `%TEMP%` escritos durante la prueba (sin `%TEMP%\claude`). No apareció ninguna carpeta nueva en `%LOCALAPPDATA%` ni en `%TEMP%`: la única `EBWebView` tocada es la del perfil de la app. **Ningún rastro nuevo en otro lugar** |
| La app abre; fuente | pasa | Bienvenida y bóveda con Overpass y Overpass Mono · `RV01-bienvenida.png`, `RV02-boveda-con-detalle.png` |
| Ajustes tras reiniciar | pasa | Guardados 15 minutos y 30 s; tras cerrar y abrir, Ajustes muestra "15 minutos" y "30" · `RV04-ajustes.png` |
| Recientes tras reiniciar | pasa | "Abrir rv-uno.kdbx" en la bienvenida; la bóveda abre, la entrada y el grupo están y copiar funciona · `RV03`, `RV05` |
| Algo dejó de funcionar | nada visto | Crear, guardar, buscar, bloquear, desbloquear, copiar y cerrar se comportaron como en la primera corrida |

Límites de esta reverificación:
- Una sola corrida. La comparación es contra la primera, donde las mismas acciones (la primera creación con su
  repetición, el primer grupo, las entradas) sí dejaron filas; no se repitió la compilación anterior sobre un perfil limpio.
- La búsqueda ve texto y bases SQLite sin comprimir; no ve dentro de bloques comprimidos (LevelDB, caché). No se
  inspeccionó la memoria del proceso.
- Sigue sin comprobarse la compilación de release. Conviene dejar este inspector como paso de la puerta de release
  (RF-77, "una vez por release").
- Dato suelto: el primer arranque con el perfil recién creado tardó 7 113 ms en mostrar la ventana (los siguientes,
  97–140 ms). Afecta a la meta de arranque en frío de ≤ 2 s, que sigue sin medirse bien.

Estado del PC tras la reverificación: instalada y cerrada la compilación nueva (modo privado); carpeta de trabajo
borrada; la reciente de prueba quitada desde la app; ajustes en 5 minutos y 20 s; portapapeles vacío. La carpeta
`EBWebView` existe, recreada por esta compilación, sin datos de formularios. La copia del `Web Data` anterior usada
como control se borró con la carpeta de trabajo.
