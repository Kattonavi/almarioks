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

## Reverificación final con el instalador de release 0.1.0

> 2026-11-11, noche · Instalador de **release** sin firmar `AlmarioKS_0.1.0_x64-setup.exe` (5 086 399 bytes, compilado
> a las 23:00:57; el último commit que toca el producto es `9efd85a`, de las 22:56:46, y su fusión `e87da5c`, de las
> 22:58:02; sin cambios de producto sin commit; los commits posteriores solo tocan documentos). Es la única
> reverificación del delta que permite la puerta de la fase.
> Scripts nuevos: `rf-lib.ps1`, `rf1-instalar.ps1`, `rf2-recorrido.ps1`, `rf3-inspeccion.ps1`, `rf4-humo.ps1`,
> `rf5-defectos.ps1`, `rf6-navegador.ps1`, `rf6b-menu.ps1`, `rf7-ventana.ps1`, `rf8-limpieza.ps1` y
> `rf-plantar-control.py` (reutilizan `qa-lib.ps1`, `b-lib.ps1`, `rv-lib.ps1` y el inspector `rv-inspeccionar.py` sin
> tocarlos). Capturas y volcados UIA: `.impeccable/review/qa2/`, fuera de git (muestran la ruta de la bóveda de
> prueba con el nombre de usuario de Windows: no se suben). Carpeta de trabajo: `%TEMP%\almarioks-qa2-20261111`.
> Solo datos falsos, con canarios nuevos (`TESTIGO-RF-…`). Pantalla 1920 × 1080 a escala 125 %.

### Veredicto
- **Rastro en disco en release (RF-77, condición dura de seguridad): 0 apariciones**, con control positivo, en dos
  pasadas: tras el recorrido mínimo y otra vez al final, después de todo lo demás.
- **D-3 a D-8: los seis corregidos.** "Volver a la lista" existe y funciona en ventana estrecha.
- **Flujo principal en release: pasa** de corrido, con teclado para buscar y copiar; la bóveda abre en KeePass 2.62.
- **Cuatro defectos nuevos, ninguno Crítico ni Alto según mi lectura:** D-9 (Medio, al borde de Alto: el menú
  contextual y las teclas de navegador de WebView2 siguen activos, y "Guardar como" e "Imprimir" sacan en claro lo
  que hay en pantalla; es el punto I-7 de la revisión de seguridad, ahora comprobado), D-10, D-11 y D-12 (Bajos).
  **La severidad de D-9 la debe confirmar quien firma seguridad**: si la sube a Alto, bloquea.
- Sigue sin hacerse la otra condición dura de seguridad: mirar Win+V a mano (no se me asignó).

### 1. Rastro en disco en release (RF-77) — **0 apariciones**

| Paso | Resultado | Evidencia |
|---|---|---|
| Borrar solo `EBWebView` e instalar `/S` | pasa | Había 153 archivos, 7,2 MB. Código 0 en 6 s. Clave de desinstalación en `HKCU`: `AlmarioKS 0.1.0` |
| Lo instalado es la compilación de release | pasa | `almarioks.exe` de 18 851 840 bytes (la de depuración medía 32 870 400), idéntico al de `target\release` salvo la marca del empaquetado NSIS; versión del archivo 0.1.0; subsistema de ventanas (no abre consola) |
| Recorrido con canarios nuevos | hecho | Crear bóveda tecleando la maestra, una repetición distinta (rechazada: "no coinciden") y la repetición correcta; entrada con título, usuario, URL, notas, etiqueta y contraseña; grupo; dos búsquedas con Enter; ajustes cambiados y devueltos; bloquear, desbloquear y cierre normal |
| **Control positivo** del inspector | pasa | Archivo plantado: ve un canario en UTF-8, otro en UTF-16LE y el SHA-1 de 8 bytes de la maestra en decimal. Base SQLite plantada con la forma de `Web Data`: cuenta sus filas `autofill_edge_field_values` (1 en claro, 1 enmascarada) y reconoce la enmascarada como el SHA-1 de la maestra. Archivo plantado en la raíz de `%TEMP%`: lo encuentra (el filtro por fecha funciona) |
| Perfil de WebView2 completo | **0 apariciones** | 135 archivos, 5,9 MB, 13 bases SQLite, todas legibles. Ningún canario en claro (UTF-8 ni UTF-16LE) y ninguna forma del SHA-1 de la maestra ni de su repetición (8 bytes en decimal, con signo, hex, crudos y al revés; SHA-1 completo). `Web Data` existe (227 328 bytes): sus 12 tablas `autofill*` tienen **0 filas** |
| Carpeta de ajustes | 0 apariciones | Solo `settings.json` con `auto_lock_minutes`, `clipboard_seconds` y `recent_vaults` |
| Carpeta de trabajo | 0 apariciones | `rf-uno.kdbx`, `rf-uno.kdbx.bak` y la salida estándar y de error del proceso (0 bytes las dos) |
| `%TEMP%`, archivos escritos durante la prueba | 0 apariciones | Sin `%TEMP%\claude` (registros de esta QA) ni la carpeta de trabajo. La única carpeta `EBWebView` tocada es la del perfil de la app |
| **Segunda pasada**, al final de toda la corrida | **0 apariciones** | Después del humo, de D-3 a D-8, de la vista de impresión, de "Guardar como", de cuatro recargas y de buscar actualizaciones. Perfil: 250 archivos, 65,3 MB, 13 bases SQLite, 0 filas `autofill*`. Aquí se buscó además la contraseña generada y copiada. Ajustes, carpeta de trabajo y `%TEMP%`: 0. Los mismos tres controles positivos, en verde |

Límites: los de la reverificación anterior (la búsqueda no ve dentro de bloques comprimidos de la caché ni la memoria
del proceso). Lo que "Guardar como" escribe a pedido del usuario no es rastro silencioso: va aparte, en D-9.

### 2. Humo del flujo principal en release
| Paso | Resultado | Evidencia |
|---|---|---|
| Primer arranque con perfil nuevo → bienvenida | pasa | `RF01-bienvenida-primer-arranque.png`, `RF01-bienvenida.uia.txt`. Sin conexiones TCP no locales en ese momento |
| Crear bóveda (repetición distinta rechazada, luego la correcta) | pasa | "La contraseña maestra y su confirmación no coinciden. Escríbelas otra vez." · bóveda de 797 bytes · `RF02` |
| Cerrar, reabrir la app y abrir la reciente (RF-19) | pasa | "Abrir rf-uno.kdbx" en la bienvenida · `RF05-bienvenida-con-reciente.png` |
| Entrada con contraseña generada (longitud 20) → guardar | pasa | `RF06-formulario-con-generador.png`, `RF07-entrada-guardada.png` |
| Buscar con teclado: Ctrl+K, escribir, flecha abajo, Enter | pasa | Foco leído por UIA en cada tecla: campo `vault-search` → botón "TESTIGO-RF-Generada" → botón "Copiar contraseña" · `RF08`, `RF09` |
| Enter copia: portapapeles real | pasa | 20 caracteres con los cuatro tipos; "Copiado: se borra en 20 s"; formatos `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`, `CanUploadToCloudClipboard` · `RF10-copiado.png` |
| El valor mostrado es el copiado y al ocultar sale del árbol UIA | pasa | "Mostrar contraseña" / "Ocultar contraseña" |
| Portapapeles vacío solo | pasa | A los 20,7 s de copiar |
| Bloquear vacía el portapapeles (RF-31); desbloquear | pasa | Portapapeles de longitud 0 tras bloquear; las dos entradas siguen tras desbloquear |
| Bloqueada, la ventana no expone canarios por UIA (RF-32) | pasa | `RF04-bloqueada.uia.txt`, `RF18-bloqueo-automatico.uia.txt` |
| Cierre normal con una contraseña recién copiada (RF-75) | pasa | Proceso terminado, portapapeles vacío, sin `.tmp` ni archivos de recuperación |

### 3. Defectos de la primera prueba, uno por uno
| Defecto | Estado | Evidencia |
|---|---|---|
| **D-3** cuenta "Se bloquea en" | **corregido** | Con el ajuste recién guardado en 1 minuto la cuenta pasa de 4:47 a 0:52. Tres lecturas sin tocar la ventana: 0:52 (0,7 s) → 0:46 (6,8 s) → 0:40 (12,8 s). El bloqueo llegó a los 52,3 s de la última entrada real, con la cuenta en 0:01 leída 0,3 s antes. Al volver a 5 minutos: 4:56 · `RF17-cuenta-regresiva.png`, `RF18-bloqueo-automatico.png` |
| **D-4** "Entrada guardada." antes del disco | **corregido** | Dos medidas. Orden de avisos visto: "Guardando la entrada…" → "Entrada creada. Guardándola en el archivo…" → "Entrada guardada." y "Guardado.". Al leer "Entrada guardada." se releyó la fecha del archivo y ya había cambiado: archivo a los 2 065 y 2 367 ms, aviso a los 2 222 y 2 396 ms |
| **D-5** buscar y copiar con teclado | **corregido** | Flecha abajo en la búsqueda lleva al primer resultado; Enter lo abre con el foco en "Copiar contraseña"; Enter copia. Enter directo en la búsqueda también abre el primer resultado. Son 5 teclas, sin tabular |
| **D-6** acciones del grupo cortadas | **corregido** | A 800 × 600 con 6 grupos y con "Archivo" elegido: los 4 botones ("Nuevo grupo…", "Renombrar grupo…", "Mover grupo…", "Enviar grupo a la papelera…") a la vista, alcanzables en su centro y en su borde inferior, dentro de la ventana y sin "Guardado." encima. A 480 × 360, igual · `RF11-acciones-grupo-800x600.png`, `RF16-acciones-grupo-480x360.png`. Queda un detalle: a 480 × 360 la cuenta de bloqueo sigue sin rótulo ("4:52") |
| **D-7** mover grupo sin explicación | **corregido** | El diálogo dice "No aparecen el propio grupo ni sus subgrupos: un grupo no puede ir dentro de sí mismo."; para "Trabajo" ofrece TESTIGO-RF-Grupo, Personal y Archivo, y no "Trabajo" ni "Clientes" · `RF12-dialogo-mover-grupo-800x600.png` |
| **D-8** crear sobre un archivo existente | **corregido** | Antes de elegir: "AlmarioKS no reemplaza archivos: si eliges uno que ya existe, aunque Windows pregunte si quieres reemplazarlo, te pedirá otro nombre." Tras responder "Sí" a Windows: "Ya existe un archivo con ese nombre en esa carpeta. AlmarioKS nunca reemplaza un archivo, aunque Windows haya preguntado si querías reemplazarlo: no se tocó. Elige otro nombre u otra carpeta." El archivo queda idéntico (SHA-256) · `RF19-archivo-ya-existe.png` |
| **"Volver a la lista"** | **existe** | A 480 × 360 con una entrada abierta el botón está a la vista; al pulsarlo vuelve la lista con el foco en la fila de esa entrada. A 800 × 600 no aparece (lista y detalle caben juntos) · `RF14-boveda-detalle-480x360.png`, `RF15-lista-tras-volver-480x360.png` |

### 4. Ventana
| Paso | Resultado | Evidencia |
|---|---|---|
| Tamaño inicial | pasa | 1120 × 720 lógicos (1400 × 900 físicos) en los tres arranques en que se leyó (primero, tercero y cuarto), antes de cualquier cambio de tamaño |
| Posición inicial | **D-12** | Abre con el borde visible de (168, 133) a (1570, 1072) físicos; el área útil acaba en y = 1020: 52 px quedan bajo la barra de tareas · `RF40-bienvenida-posicion-inicial.png`, `RF03-boveda-detalle-1120x720.png` |
| Bóveda con detalle a 1120 × 720 | pasa | Usuario (46 caracteres) y URL (55) se leen completos en el detalle, partidos en líneas y sin "…"; ningún control fuera de la vista según UIA · `RF42-boveda-detalle-1120x720.png`. En la fila de la lista el usuario sí se recorta con "…" (el detalle lo da entero). "Enviar a la papelera" queda justo en el pliegue del detalle: asoma y pide un poco de desplazamiento |
| Bóveda con detalle a 800 × 600 | pasa | Usuario y URL completos, partidos en más líneas; "Editar", "Mover a otro grupo…" y "Enviar a la papelera" piden desplazar el detalle · `RF42-boveda-detalle-800x600.png` |
| Bóveda con detalle a 480 × 360 | pasa | La barra de grupos pasa a iconos, el detalle ocupa el sitio de la lista y aparece "Volver a la lista"; sin desborde horizontal · `RF42-boveda-detalle-480x360.png` |
| Mínimo arrastrando la esquina | pasa | Se detiene en 480 × 360 · `RF52-minimo-arrastrando.png` (el primer intento del script no agarró el borde; el segundo, 3 px fuera del borde visible, sí) |
| Instancia única (RF-81) | pasa | Al lanzar el exe otra vez: un solo proceso, una sola ventana, la segunda instancia termina y la bóveda sigue abierta como estaba |

### 5. Superficie de navegador en release (I-7)
Con la bóveda abierta y una entrada a la vista. Cada tecla se envió con la ventana en primer plano y se anotó el foco,
la ventana en primer plano y lo que cambió en el árbol UIA.

| Gesto | Qué pasa | Nota |
|---|---|---|
| Clic derecho sobre texto o fondo | **Menú de WebView2:** Atrás · Actualizar · Guardar como · Imprimir · Más herramientas (→ Compartir) | D-9, D-10 · `RF20`, `RF23`, `RF31-menu-contextual.png`, `RF34-mas-herramientas.png` |
| Clic derecho sobre texto seleccionado | Copiar · Copiar vínculo al texto resaltado · Imprimir · Más herramientas | `RF21` |
| Clic derecho sobre un campo | Emoji · **Importar contraseñas** · Deshacer · Cortar · Copiar · Pegar · Pegar como texto sin formato · Seleccionar todo · Dirección de escritura · Más herramientas | D-11 · `RF22`. No pulsé "Importar contraseñas" ni "Compartir" |
| "Inspeccionar" en el menú | no está | Coincide con la compilación sin `devtools` |
| F12 · Ctrl+Shift+I · Ctrl+Shift+J · Ctrl+Shift+C | nada | No se abren herramientas de desarrollo |
| "Guardar como" del menú | **abre el diálogo nativo y guarda la página** | D-9 · `RF32-dialogo-guardar-como.png`, `RF33-tras-guardar-como.png` |
| Ctrl+S | nada | El guardado solo se alcanza por el menú |
| Ctrl+P | **abre la vista de impresión** de WebView2 con la página de la bóveda y "Microsoft Print to PDF" | D-9 · `RF26-ctrl-p.png`. Escape la cierra. No se imprimió nada |
| Ctrl+Shift+P | **abre el diálogo de impresión de Windows** | D-9 · `RF35-ctrl-shift-p.png`. Escape lo cierra |
| F5 · Ctrl+R · Ctrl+Shift+R · Ctrl+F5 | **recargan la página** | D-10 |
| Ctrl+F con la bóveda abierta | va a la búsqueda de la app | Foco en `vault-search`; no aparece la búsqueda del navegador · `RF24-ctrl-f.png` |
| Ctrl+F con la bóveda bloqueada · F3 · Ctrl+G | **abren "Buscar en la página"** del navegador | D-11 · `RF25-F3.png`, `RF29-ctrl-f-bloqueada.png` |
| Ctrl+J | abre el panel "Descargas" del navegador | D-11 · `RF25-Ctrl-J.png` |
| Ctrl+U · Ctrl+H · Ctrl+N · Ctrl+O · F7 · Ctrl++ (zoom) | nada visible | Foco y primer plano sin cambios; el ancho de "Bloquear" no cambia con Ctrl++ |

Panel de actualización (C-1, solo lo que se puede probar sin un release publicado): con la bóveda abierta, Ajustes
muestra "Actualizaciones" y "Buscar actualizaciones". Al pulsarlo se vio "Buscando actualizaciones…" y a los 0,9 s el
texto exacto **"Tienes la versión más reciente."** No se ofreció ninguna versión, no se descargó ni instaló nada y la
bóveda siguió abierta · `RF30-buscar-actualizaciones.png`. El cierre de la bóveda antes de instalar no se probó.

### 6. Compatibilidad
KeePassLib 2.62 (solo lectura) abre `rf-uno.kdbx`, creada y editada por la app de release, con la maestra con `ñ` y
`€`: cabecera KDBX 4 (`03 D9 A2 9A 67 FB 4B B5 01 00 04 00`), Argon2id de 64 MiB, 48 iteraciones y 2 hilos, cifrado
ChaCha20, papelera activa. Ve `rf-uno` con sus 2 entradas (la del canario con título, usuario, URL, notas, etiqueta y
contraseña exactos; la otra con la contraseña generada, igual a la que salió del portapapeles) y los grupos
`TESTIGO-RF-Grupo`, `Trabajo › Clientes`, `Personal` y `Archivo`. El `.bak` abre con la misma llave y trae el estado
anterior (sin "Archivo"). Una contraseña distinta se rechaza. KeePassXC: no probado (no se instala).

### Tiempos medidos (release)
| Qué | Tiempo | Nota |
|---|---|---|
| Arranque hasta leer "Bienvenida", primero con perfil nuevo | 4 944 ms | Ventana a los 4 175 ms. Por encima de la meta de ≤ 2 s; es el arranque que crea el perfil de WebView2 (una vez por instalación) |
| Arranque hasta leer "Bienvenida", siguientes | 635 · 602 · 676 ms | Ventana a los 142, 76 y 147 ms. Con la app recién cerrada: no son arranques tras reiniciar el equipo, que sigue sin medirse |
| Crear bóveda (clic → bóveda abierta) | 9 071 ms | Incluye calibrar Argon2 (RF-2). Igual que en depuración (8 979 ms) |
| Desbloquear (clic → bóveda abierta) | 1 168 · 1 184 · 1 151 · 1 207 · 1 120 ms | Meta ≤ 1,5 s |
| Bloquear (clic → pantalla de desbloqueo) | 678 · 657 · 645 · 666 ms | Meta ≤ 1 s |
| Guardar entrada: clic → archivo cambiado en disco | 2 065 · 2 367 ms | |
| Guardar entrada: clic → "Entrada guardada." | 2 222 · 2 396 ms | Después del disco (D-4 corregido) |
| Limpieza del portapapeles | 20,7 s | N = 20 |
| Bloqueo automático | 52,3 s | Ajuste: 1 minuto. La cuenta lo anunciaba (D-3) |
| "Buscar actualizaciones" → respuesta | 0,9 s | |

Los tiempos incluyen la espera fija del clic del script (~200 ms) y el sondeo por UIA.

### Defectos nuevos

#### D-9 · Medio (al borde de Alto) · "Guardar como" e "Imprimir" de WebView2 sacan en claro lo que hay en pantalla
- **Qué hice:** con una entrada abierta y su contraseña mostrada, clic derecho sobre el texto de las notas →
  "Guardar como" → en el diálogo nativo elegí un archivo dentro de la carpeta de trabajo → Guardar. Aparte, Ctrl+P.
- **Qué esperaba:** RF-77 (los datos de la bóveda viven solo en el archivo cifrado) y AMENAZAS W1 ("exportar pide la
  maestra"): ninguna vía de la ventana escribe el contenido en claro. La revisión de seguridad lo dejó sin verificar (I-7).
- **Qué pasó:** el diálogo "Guardar como" se abre en Descargas, propone `AlmarioKS.html` y tipo "Página web,
  completa". El archivo guardado (12 998 bytes) contiene en claro título, usuario, URL, notas, etiqueta, nombres de
  grupos y **la contraseña mostrada** (8 agujas del inspector). Ctrl+P abre la vista de impresión con la misma
  página y "Microsoft Print to PDF" como impresora; Ctrl+Shift+P, el diálogo de impresión de Windows. No confirmé
  ninguna impresión. No hay rastro silencioso: tras usarlas, el perfil y `%TEMP%` siguen en 0 apariciones.
- **Cómo repetirlo:** `rf6b-menu.ps1` (paso RF37) y `rf6-navegador.ps1` (paso RF26). El archivo guardado se borró.
- **Severidad:** 2 × 2 = Medio en la escala de AMENAZAS: hace falta un gesto deliberado del dueño y confirmar un
  diálogo. Sube a Alto si se cuenta impacto 3 (contraseña en claro en Descargas, carpeta que suele sincronizarse con
  la nube). Que lo decida quien firma seguridad.
- **Arreglo sugerido (sin verificar):** apagar en WebView2 el menú contextual por defecto y las teclas de
  navegador (`AreDefaultContextMenusEnabled` y `AreBrowserAcceleratorKeysEnabled` en falso), conservando Ctrl+F y
  Ctrl+K de la app. Ojo: sin menú contextual se pierde "Pegar" con el ratón en los campos; habría que decidir si se
  ofrece uno propio. Cierra también D-10 y D-11. Añadir estos gestos a la puerta de release.

#### D-10 · Bajo · F5, Ctrl+R, Ctrl+Shift+R, Ctrl+F5 y "Actualizar" recargan la ventana
- **Qué hice:** escribir un texto en la búsqueda con una entrada abierta y pulsar cada tecla.
- **Qué pasó:** la página se recarga las cuatro veces: la búsqueda queda vacía y la entrada abierta se cierra. La
  bóveda sigue abierta (vuelve a la lista del grupo raíz); no se bloquea ni se cierra.
- **Riesgo:** perder lo que esté a medio escribir en un formulario (deducido: el formulario vive en la página; no lo
  probé con un formulario abierto). "Atrás" del menú no se pulsó.
- **Cómo repetirlo:** `rf6-navegador.ps1`, paso RF28.

#### D-11 · Bajo · Quedan a la vista piezas del navegador que no son de la app
- F3 y Ctrl+G (y Ctrl+F con la bóveda bloqueada) abren "Buscar en la página"; Ctrl+J abre "Descargas"; el menú de
  los campos ofrece "Importar contraseñas" y "Emoji", y "Más herramientas" ofrece "Compartir". Ninguna expuso datos
  en esta prueba, pero rompen la idea de una app de escritorio y "Importar contraseñas" confunde en un gestor de
  contraseñas. Con la bóveda abierta, Ctrl+F sí es de la app: la misma tecla hace dos cosas según la pantalla.

#### D-12 · Bajo · La ventana abre con el pie bajo la barra de tareas
- **Qué pasó:** en 1920 × 1080 a 125 % la ventana de 1120 × 720 abre donde Windows la coloca, de (168, 133) a
  (1570, 1072) físicos, y el área útil termina en y = 1020. Los últimos 52 px (la barra de estado con la versión, el
  aviso "Versión de desarrollo: no la uses con tus contraseñas reales." y parte de "Guardado.") quedan tapados hasta
  que la persona mueve la ventana. Visto en los arranques primero y cuarto · `RF03`, `RF40`, `RF41`.
- **Deducido, sin probar:** en un portátil de 1366 × 768 a 100 % el área útil mide unos 728 px de alto y la ventana
  con su título unos 751: no cabe entera en ninguna posición.
- **Arreglo sugerido (sin verificar):** `"center": true` en la ventana de `tauri.conf.json` y, si el área útil es
  menor que el tamaño por defecto, abrir ajustada a ella.

### Heurísticas de Nielsen (lo que cambia respecto a la primera tabla)
| Heurística | Pantalla | Severidad | Evidencia |
|---|---|---|---|
| Visibilidad del estado del sistema | P4 cuenta de bloqueo · P5 guardado | resuelto | D-3 y D-4 corregidos |
| Flexibilidad y eficiencia de uso | P4 búsqueda con teclado | resuelto | D-5 corregido |
| Prevención de errores · Consistencia | P7 mover grupo · P2 archivo existente | resuelto | D-7 y D-8 corregidos |
| Estética y diseño minimalista | P4 barra de grupos | resuelto | D-6 corregido |
| Prevención de errores | Toda la ventana (menú y teclas de navegador) | Medio | D-9, D-10 |
| Consistencia y estándares | Toda la ventana (piezas del navegador) | Bajo | D-11 |
| Visibilidad del estado del sistema | Ventana al abrir (barra de estado tapada) | Bajo | D-12 |

### Red
`Get-NetTCPConnection` por PID en tres momentos. En la bienvenida del primer arranque: ninguna conexión. Con la
bóveda abierta y tras buscar actualizaciones: `almarioks.exe` sin conexiones no locales en el momento de la muestra
(la consulta del actualizador ya había terminado); `msedgewebview2.exe` con 2 conexiones a `52.96.173.178:443`,
propias del motor, como en la primera prueba. No se inspeccionó su contenido.

### Lo que no se probó
Win+V a mano (RF-72, la otra condición dura de seguridad) · la actualización de punta a punta y el cierre de la
bóveda antes de instalar (C-1; pide un release publicado) · KeePassXC · suspensión y bloqueo de sesión · confirmar
una impresión real · "Importar contraseñas", "Compartir", "Atrás" y "Copiar vínculo al texto resaltado" del menú ·
recargar con un formulario a medio llenar · otras pantallas y escalas (100 %, 150 %, 1366 × 768) · alto contraste y
lector de pantalla · archivo llave y archivos dañados (sin cambios desde la primera prueba) · un arranque tras
reiniciar el equipo.

### Límites y tropiezos del método
- Cuatro comprobaciones fallaron por el script y no por la app; se corrigieron y se repitieron: el control positivo
  de SQLite (tomaba otra aguja), el texto de ayuda de D-7 (su `id` no se expone por UIA), la lectura del menú
  contextual (solo se alcanza subiendo desde lo que hay bajo el puntero) y el primer arrastre de la esquina.
- El diálogo "Guardar como" no aparece entre las ventanas de primer nivel de UIA: se encontró por clase entre las
  ventanas del proceso. Para las demás teclas la señal fue el foco y la ventana en primer plano.
- **`rf6-navegador.ps1` da avisos falsos y quien lo repita los verá:** anota `HALLAZGO` en cada tecla porque la
  cuenta regresiva cambia entre dos lecturas del árbol; marca `FALLA` en Ctrl+F aunque el foco fue a la búsqueda de la
  app; envió Ctrl+Shift+P con la vista de impresión todavía abierta (no cierra lo que no abre ventana nueva), y su
  manejo de Ctrl+S no vería el diálogo. El veredicto por tecla de la tabla sale del foco y de la ventana en primer
  plano que el script registra, no de sus `PASA`/`FALLA`. La evidencia válida de "Guardar como" y de Ctrl+Shift+P es
  `rf6b-menu.ps1`. No se corrigió el script porque ya no había forma de volver a probarlo en esta ronda.
- La marca "TAPADO O RECORTADO" que el script anota en "Bloquear" y "Ajustes y Acerca de" es el falso aviso de la
  barra superior ya descrito en la primera prueba: en las capturas se ven enteros.

### Estado en que quedó el PC
Instalada y cerrada la compilación de **release 0.1.0** (`almarioks.exe` de 18 851 840 bytes). Carpeta de trabajo
borrada, sin restos en `%TEMP%`. La reciente de prueba quitada desde la app ("Quitar rf-uno.kdbx de la lista").
Ajustes en 5 minutos y 20 s. Portapapeles vacío. Nada nuevo en Descargas. `EBWebView` existe, recreada por esta
compilación, sin datos de formularios.
