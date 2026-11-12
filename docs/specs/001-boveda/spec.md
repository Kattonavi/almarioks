# Spec 001 — Bóveda (fase 0.1)

> Estado: **borrador** (2026-11-11) · Proyecto: almarioks · Constitución: `CLAUDE.md` → "Constitución"
> Escrita sin entrevista: el dueño no estaba. Las respuestas salen de `docs/PROYECTO.md`, `docs/UX.md`,
> `docs/AMENAZAS.md` §4, `docs/ARQUITECTURA.md`, los ADR 0001 y 0002 y `docs/BACKLOG.md`. Cada decisión que tomó la
> directora por defecto lleva **[por confirmar]** y está reunida en "Decisiones por defecto". Todas son reversibles.

## Contexto y objetivo
El H0 dejó una app que instala, abre y se actualiza, pero no guarda nada. Esta fase la convierte en un gestor de
contraseñas local utilizable: la persona crea su bóveda, la abre, guarda y encuentra sus cuentas, copia una contraseña
sin verla y se va tranquila porque la app se bloquea sola. La bóveda es un archivo KDBX 4 que también abren KeePass 2
y KeePassXC: si AlmarioKS desaparece, las contraseñas siguen siendo de su dueño.

## Usuarios / actores
- **Dueño de la bóveda:** una persona en su PC con Windows, con o sin gente al lado.
- **Otra app KeePass** (KeePass 2, KeePassXC): lee y escribe el mismo archivo cuando AlmarioKS no lo tiene abierto.
- **Contenido no confiable:** campos de una bóveda que escribió otra app o que vino de otra persona.

## Historias de usuario
- H1: Como dueño quiero crear mi bóveda en un minuto y entender qué pasa si pierdo la contraseña o el archivo llave.
- H2: Como dueño quiero abrir una bóveda mía, hecha aquí o en KeePass, con mi contraseña, mi archivo llave o ambos.
- H3: Como dueño quiero que cada cambio quede guardado sin riesgo de dañar el archivo.
- H4: Como dueño quiero que la bóveda se bloquee cuando lo pido y cuando me voy.
- H5: Como dueño quiero crear, ver, editar, mover y borrar entradas.
- H6: Como dueño quiero organizar las entradas en grupos.
- H7: Como dueño quiero encontrar una entrada escribiendo parte de lo que recuerdo.
- H8: Como dueño quiero que la app me proponga contraseñas fuertes.
- H9: Como dueño quiero copiar una contraseña sin mostrarla y que no se quede en el portapapeles.
- H10: Como dueño quiero ver las versiones anteriores de una entrada (recortable).

## Requisitos funcionales (criterios de aceptación en EARS)
Las pruebas citan el número: `rf_NN_…` en Rust, `it("RF-NN: …")` en TypeScript. Los errores tienen un código estable
(en `CódigoAsí`) además del texto en español.

### A. Crear bóveda (H1)
- RF-1: CUANDO el dueño confirma una ruta nueva y una contraseña maestra, un archivo llave o ambos, EL SISTEMA creará
  ahí una bóveda KDBX 4 vacía que se vuelve a abrir con esa misma llave y con ninguna otra.
- RF-2: EL SISTEMA creará toda bóveda nueva con Argon2id, 64 MiB de memoria o más y al menos 2 iteraciones, calibradas
  para que derivar la llave tarde entre 0,5 y 1,5 s en el equipo donde se crea.
- RF-3: SI el dueño no da contraseña maestra ni archivo llave, ENTONCES EL SISTEMA rechazará la creación (`LlaveVacia`).
- RF-4: SI en la ruta elegida ya existe un archivo, ENTONCES EL SISTEMA rechazará la creación sin modificarlo
  (`ArchivoExiste`).
- RF-5: CUANDO el dueño pide un archivo llave nuevo, EL SISTEMA generará un `.keyx` versión 2 con 32 bytes del
  generador aleatorio del sistema operativo y solo dará la creación por hecha después de reabrir la bóveda con él.
- RF-6: SI la ruta del archivo llave nuevo ya existe, ENTONCES EL SISTEMA rechazará generarlo sin modificar el
  existente (`ArchivoExiste`).
- RF-7: MIENTRAS el dueño escribe la contraseña maestra, EL SISTEMA mostrará su fortaleza en una escala de 0 a 4 y,
  si es menor que 3, avisará que es débil sin impedir continuar.
- RF-8: SI la contraseña maestra y su confirmación no coinciden, ENTONCES EL SISTEMA no creará la bóveda y dirá que
  no coinciden.
- RF-9: CUANDO la bóveda queda creada con archivo llave, EL SISTEMA avisará que sin una copia de ese archivo y sin la
  contraseña no hay forma de recuperarla.

### B. Abrir y desbloquear (H2)
- RF-10: CUANDO el dueño da la llave correcta de una bóveda KDBX 4 escrita por AlmarioKS, KeePass 2 o KeePassXC,
  EL SISTEMA la abrirá y mostrará sus grupos y entradas.
- RF-11: SI la llave no abre la bóveda, ENTONCES EL SISTEMA responderá `LlaveIncorrecta` con el mismo mensaje sea
  cual sea la parte equivocada (contraseña, archivo llave o ambas) y la bóveda seguirá bloqueada.
- RF-12: SI el archivo llave es un `.keyx` versión 2 cuyo `Hash` no corresponde a su contenido, ENTONCES EL SISTEMA
  responderá `ArchivoLlaveDanado` sin intentar abrir la bóveda.
- RF-13: SI el archivo no es KDBX, está truncado o su contenido cifrado no verifica, ENTONCES EL SISTEMA responderá
  `ArchivoDanado`; SI es una versión de KDBX que no soporta, responderá `FormatoNoSoportado`.
- RF-14: CUANDO el dueño abre una bóveda KDBX 3.1, EL SISTEMA la abrirá en solo lectura y toda operación que la
  modifique responderá `SoloLectura`.
- RF-15: CUANDO el dueño pide convertir una bóveda KDBX 3.1, EL SISTEMA escribirá una bóveda KDBX 4 en una ruta nueva
  con el KDF de RF-2 y dejará el archivo original idéntico byte a byte.
- RF-16: SI la cabecera pide más de 1 GiB de memoria, más de 100 iteraciones de Argon2, más de 16 hilos o más de
  100 000 000 de rondas de AES-KDF, ENTONCES EL SISTEMA rechazará el archivo con `ParametrosKdfExcesivos` antes de
  derivar la llave. *(No bloquea la fase: AMENAZAS §4.)*
- RF-17: CUANDO abre una bóveda cuyo KDF no es Argon2id o usa menos de 64 MiB, EL SISTEMA la abrirá y avisará que su
  protección contra fuerza bruta es débil.
- RF-18: MIENTRAS se deriva la llave, EL SISTEMA mostrará progreso y la ventana seguirá respondiendo.
- RF-19: EL SISTEMA recordará las rutas de las últimas 5 bóvedas abiertas, sin guardar ninguna llave ni contraseña.
- RF-20: SI una bóveda reciente ya no está en su ruta, ENTONCES EL SISTEMA lo dirá y ofrecerá buscarla o quitarla de
  la lista.

### C. Guardar (H3)
- RF-21: CUANDO el dueño confirma una operación que cambia la bóveda, EL SISTEMA la guardará de inmediato escribiendo
  un archivo temporal en la misma carpeta, releyéndolo y descifrándolo con la llave, y solo entonces reemplazando el
  archivo de forma atómica.
- RF-22: CUANDO reemplaza el archivo de la bóveda, EL SISTEMA conservará la versión anterior como `<archivo>.bak`
  (una sola copia, que se reemplaza en cada guardado).
- RF-23: SI el guardado falla en cualquier paso, ENTONCES EL SISTEMA dejará el archivo original intacto, mantendrá los
  cambios en memoria, informará `GuardadoFallido` y permitirá reintentar o guardar en otra ruta.
- RF-24: SI el archivo cambió en disco desde la última vez que AlmarioKS lo leyó o escribió, ENTONCES EL SISTEMA no lo
  sobrescribirá, informará `CambioExterno` y permitirá guardar en otra ruta o recargar descartando lo pendiente.
- RF-25: EL SISTEMA conservará sin cambios todo lo que no edita: adjuntos, historial, campos propios, iconos, datos
  de plugins, objetos borrados y metadatos de la base.
- RF-26: CUANDO una entrada se edita, EL SISTEMA agregará la versión anterior a su historial y guardará el historial
  ordenado de la versión más vieja a la más nueva.
- RF-27: SI el proceso termina a mitad de un guardado, ENTONCES el archivo original seguirá abriendo con su llave.

### D. Bloquear y cerrar (H4)
- RF-28: CUANDO el dueño pide bloquear, EL SISTEMA dejará la bóveda bloqueada y mostrará la pantalla de desbloqueo.
- RF-29: CUANDO pasan N minutos sin teclado ni ratón sobre la ventana (N = 5 por defecto; opciones 1, 5, 15 y 30; no
  existe "nunca"), EL SISTEMA bloqueará la bóveda.
- RF-30: CUANDO se pide bloquear, por cualquier vía, EL SISTEMA responderá `Bloqueada` a todo comando de la bóveda en
  1 s o menos, aunque un guardado esté colgado o falle.
- RF-31: CUANDO bloquea, EL SISTEMA vaciará el portapapeles si su contenido lo puso AlmarioKS.
- RF-32: CUANDO bloquea, EL SISTEMA retirará de la ventana todo dato de la bóveda, incluidos valores revelados y
  formularios a medio llenar.
- RF-33: SI al bloquear hay cambios que no se pudieron guardar, ENTONCES EL SISTEMA intentará escribirlos cifrados
  con la misma llave en `<archivo>.recuperacion.kdbx` con un límite de 5 s y soltará la bóveda y las llaves pase lo
  que pase.
- RF-34: CUANDO el dueño desbloquea después de un bloqueo con cambios pendientes, EL SISTEMA le dirá si quedaron en
  el archivo de recuperación o si se perdieron.
- RF-35: MIENTRAS la bóveda está cerrada o bloqueada, EL SISTEMA responderá `Bloqueada` a todo comando que lea o
  cambie su contenido.
- RF-36: CUANDO el dueño cierra la bóveda, EL SISTEMA hará lo mismo que al bloquear y volverá a la bienvenida.
- RF-37: CUANDO Windows bloquea la sesión o suspende el equipo, EL SISTEMA bloqueará la bóveda. *(Recortable 4.)*

### E. Entradas (H5)
- RF-38: CUANDO el dueño confirma una entrada nueva con título, EL SISTEMA la creará en el grupo elegido con título,
  usuario, contraseña, URL, notas y etiquetas.
- RF-39: SI el título queda vacío después de quitarle los espacios de los extremos, ENTONCES EL SISTEMA rechazará
  crear o editar la entrada (`TituloVacio`).
- RF-40: SI un dato escrito por el dueño supera su límite (título y usuario 255 caracteres, URL 2 048, contraseña
  1 024, notas 100 000, 50 etiquetas de 50 caracteres), ENTONCES EL SISTEMA lo rechazará diciendo cuál campo y cuál
  límite (`CampoMuyLargo`).
- RF-41: EL SISTEMA guardará la contraseña exactamente como se escribió: sin recortar espacios, sin cambiar
  mayúsculas ni normalizar Unicode.
- RF-42: CUANDO el dueño abre un grupo, EL SISTEMA listará sus entradas con título, usuario, URL, etiquetas y fecha de
  modificación, ordenadas por título sin distinguir mayúsculas ni tildes.
- RF-43: CUANDO el dueño abre una entrada, EL SISTEMA mostrará sus datos no protegidos, las notas, los nombres de sus
  campos propios y de sus adjuntos, y si tiene contraseña, sin enviar a la ventana ningún valor protegido.
- RF-44: CUANDO el dueño pide mostrar un campo protegido de una entrada, EL SISTEMA entregará a la ventana solo ese
  valor de esa entrada.
- RF-45: CUANDO el dueño confirma la edición de una entrada, EL SISTEMA guardará los campos cambiados y actualizará
  su fecha de modificación.
- RF-46: CUANDO el dueño mueve una entrada a otro grupo, EL SISTEMA la dejará solo en el grupo destino.
- RF-47: CUANDO el dueño borra una entrada que no está en la papelera, EL SISTEMA la moverá a la papelera de la
  bóveda, creándola si no existe.
- RF-48: CUANDO el dueño confirma borrar una entrada que está en la papelera, EL SISTEMA la eliminará de forma
  definitiva y la anotará en los objetos borrados de la bóveda.
- RF-49: CUANDO el dueño pide abrir la URL de una entrada, EL SISTEMA la abrirá en el navegador solo si su esquema es
  `http` o `https`; SI es otro, responderá `UrlNoPermitida` y no abrirá nada.

### F. Grupos (H6)
- RF-50: CUANDO la bóveda está abierta, EL SISTEMA mostrará el árbol de grupos con el número de entradas de cada uno.
- RF-51: CUANDO el dueño confirma un grupo nuevo con nombre, EL SISTEMA lo creará dentro del grupo elegido.
- RF-52: SI el nombre de un grupo queda vacío después de quitarle los espacios de los extremos o supera 255
  caracteres, ENTONCES EL SISTEMA rechazará crearlo o renombrarlo (`NombreVacio`, `CampoMuyLargo`).
- RF-53: CUANDO el dueño renombra un grupo, EL SISTEMA guardará el nombre nuevo.
- RF-54: CUANDO el dueño mueve un grupo por el menú, EL SISTEMA lo dejará con todo su contenido dentro del destino.
- RF-55: SI el destino de un movimiento es el mismo grupo o uno de sus descendientes, ENTONCES EL SISTEMA lo
  rechazará (`MovimientoInvalido`).
- RF-56: CUANDO el dueño pide borrar un grupo, EL SISTEMA dirá cuántas entradas y subgrupos contiene y, al confirmar,
  lo moverá con su contenido a la papelera.
- RF-57: SI el dueño intenta mover o borrar el grupo raíz, o mover o renombrar la papelera, ENTONCES EL SISTEMA lo
  rechazará (`GrupoProtegido`).
- RF-58: CUANDO el dueño confirma vaciar la papelera, EL SISTEMA eliminará de forma definitiva todo su contenido y lo
  anotará en los objetos borrados.

### G. Búsqueda (H7)
- RF-59: CUANDO el dueño escribe una consulta, EL SISTEMA listará las entradas en las que **cada palabra** de la
  consulta aparece en el título, el usuario, la URL, las etiquetas o las notas.
- RF-60: EL SISTEMA comparará sin distinguir mayúsculas ni tildes y sin contar los espacios de los extremos.
- RF-61: EL SISTEMA nunca buscará dentro de contraseñas ni de otros valores protegidos.
- RF-62: EL SISTEMA excluirá de los resultados las entradas que están en la papelera.
- RF-63: EL SISTEMA ordenará los resultados poniendo primero las entradas cuyo título contiene la consulta y, dentro
  de cada bloque, por título.
- RF-64: SI la consulta no tiene resultados, ENTONCES EL SISTEMA lo dirá y ofrecerá limpiarla.

### H. Generador (H8)
- RF-65: CUANDO el dueño pide una contraseña, EL SISTEMA generará una de la longitud pedida (entre 8 y 128; 20 por
  defecto) con los tipos elegidos entre minúsculas, mayúsculas, dígitos y símbolos (los cuatro por defecto).
- RF-66: EL SISTEMA incluirá al menos un carácter de cada tipo elegido.
- RF-67: DONDE el dueño active "evitar caracteres parecidos", EL SISTEMA no usará `0`, `O`, `o`, `1`, `l`, `I` ni `|`.
- RF-68: SI no hay ningún tipo elegido o la longitud está fuera de 8 a 128, ENTONCES EL SISTEMA no generará nada y
  dirá por qué (`ReglasImposibles`).
- RF-69: EL SISTEMA tomará cada carácter del generador aleatorio del sistema operativo, con la misma probabilidad
  para cada carácter permitido.
- RF-70: CUANDO muestra una contraseña generada o escrita en el formulario de entrada, EL SISTEMA mostrará su
  fortaleza en la misma escala de 0 a 4.

### I. Copiar (H9)
- RF-71: CUANDO el dueño copia la contraseña u otro campo protegido, EL SISTEMA lo pondrá en el portapapeles desde el
  núcleo, sin que el valor pase por la ventana.
- RF-72: EL SISTEMA marcará lo que copia para que Windows no lo guarde en el historial del portapapeles (Win+V) ni lo
  sincronice con la nube.
- RF-73: CUANDO pasan N segundos desde la copia (N = 20 por defecto; entre 10 y 120), EL SISTEMA vaciará el
  portapapeles solo si todavía contiene lo que AlmarioKS puso.
- RF-74: CUANDO copia un valor protegido, EL SISTEMA dirá en cuántos segundos se limpiará.
- RF-75: CUANDO la app se cierra, EL SISTEMA vaciará el portapapeles si su contenido lo puso AlmarioKS.

### J. Reglas transversales
- RF-76: EL SISTEMA no entregará a la ventana ninguna contraseña, valor protegido, adjunto protegido ni llave, salvo
  por la acción de mostrar de RF-44 y la salida del generador mientras está abierto.
- RF-77: EL SISTEMA no escribirá en disco ningún secreto en claro: ni en la carpeta de datos de la app, ni en la de
  WebView2, ni en temporales, ni en registros.
- RF-78: EL SISTEMA mostrará como texto literal todo contenido de la bóveda: nunca lo interpretará como HTML, script
  o enlace ejecutable.
- RF-79: SI la ventana invoca un comando que no está en la lista de comandos permitidos, ENTONCES EL SISTEMA lo
  rechazará.
- RF-80: SI un comando recibe un argumento de tipo equivocado, fuera de rango o con campos que no conoce, ENTONCES
  EL SISTEMA lo rechazará sin cambiar la bóveda (`ArgumentoInvalido`).
- RF-81: CUANDO se abre AlmarioKS y ya hay una instancia corriendo, EL SISTEMA traerá al frente la ventana existente
  y no abrirá otra.
- RF-82: EL SISTEMA guardará los ajustes (tiempo de bloqueo, tiempo de limpieza, bóvedas recientes) fuera de la
  bóveda y, si al leerlos alguno no es válido, usará su valor por defecto.
- RF-83: EL SISTEMA dirá en cada error qué pasó y qué puede hacer la persona, en español y sin códigos técnicos a la
  vista.
- RF-84: EL SISTEMA no hará ninguna conexión de red fuera de la búsqueda de actualización que el dueño pide.

### K. Recortables, en este orden si el tiempo no alcanza (`docs/PROYECTO.md`)
1. RF-85 (H10): CUANDO el dueño abre el historial de una entrada, EL SISTEMA listará sus versiones anteriores con
   fecha, de la más nueva a la más vieja, con los mismos límites de RF-43 y RF-44.
2. RF-86: CUANDO el dueño agrega, quita o guarda en disco un adjunto, EL SISTEMA lo hará sin pasar su contenido por
   la ventana. *(Si se recorta: los adjuntos se conservan y se ven sus nombres, RF-25 y RF-43.)*
3. La papelera (RF-47, RF-48, RF-58): si se recorta, borrar confirma y elimina de forma definitiva.
4. RF-37 (bloqueo con la sesión de Windows) y mover grupos arrastrando (el menú de RF-54 se queda).

## Ejemplos trabajados (son criterios de aceptación)
1. **Limpieza del portapapeles (RF-73).** 10:00:00 copio la contraseña de GitHub con N = 20. (a) No copio nada más:
   a las 10:00:20 el portapapeles está vacío. (b) A las 10:00:08 copio "hola" desde el Bloc de notas: a las 10:00:20
   el portapapeles sigue teniendo "hola". (c) A las 10:00:05 copio la contraseña de otra entrada: la cuenta empieza
   de nuevo y se limpia a las 10:00:25.
2. **Búsqueda (RF-59, RF-60, RF-63).** Entradas: "Canción favorita" (usuario `ana`), "Banco" (notas: "cancion de
   cuna") y "GitHub" (usuario `ana@example.com`). Consulta `CANCION` → "Canción favorita" y luego "Banco". Consulta
   `ana git` → solo "GitHub". Consulta `  banco  ` → "Banco".
3. **Mover grupos (RF-55).** Árbol: Raíz › Trabajo › Clientes. Mover "Trabajo" dentro de "Clientes" → rechazado.
   Mover "Clientes" a Raíz → Raíz › Trabajo y Raíz › Clientes.
4. **Borrar un grupo (RF-56).** "Trabajo" tiene 3 entradas y el subgrupo "Clientes" con 2: el aviso dice "5 entradas
   y 1 subgrupo"; al confirmar, las 5 quedan en la papelera dentro de "Trabajo".
5. **Generador (RF-66, RF-67).** Longitud 8, los cuatro tipos, "evitar parecidos" activo: 8 caracteres, al menos una
   minúscula, una mayúscula, un dígito y un símbolo, y ninguno de `0 O o 1 l I |`.
6. **Respaldo (RF-22).** `cuentas.kdbx` tiene 3 entradas. Agrego la cuarta: `cuentas.kdbx` tiene 4 y
   `cuentas.kdbx.bak` abre con la misma llave y tiene 3. Agrego la quinta: 5 y 4.
7. **Llave incorrecta (RF-11).** Bóveda con contraseña y archivo llave. Contraseña mala + archivo bueno, contraseña
   buena + archivo de otra bóveda, y contraseña buena sin archivo dan el mismo código y el mismo texto.

## Requisitos no funcionales
- **Tiempos (PC de referencia, el del dueño):** desbloquear una bóveda creada aquí ≤ 1,5 s · arranque en frío hasta
  la pantalla de desbloqueo ≤ 2 s · buscar en 5 000 entradas ≤ 100 ms · bloquear ≤ 1 s (RF-30).
- **Seguridad:** requisitos [B] de la 0.1 en `docs/AMENAZAS.md` §4, que aquí son RF-2 y RF-17 (F1); RF-21, RF-26,
  RF-14 y RF-15 (F3); RF-30 a RF-33 (W3); RF-71 a RF-75 (W4); RF-78 y RF-49 (W1); RF-77 (F5, W2).
- **Compatibilidad:** una bóveda creada y editada aquí abre en KeePass 2.62 y KeePassXC 2.8 con todo su contenido.
- **Plataforma e idioma:** Windows 10 y 11 x64 · español.
- **Accesibilidad mínima de la fase:** todo se puede hacer con teclado, cada control tiene nombre accesible y el foco
  se ve. La revisión completa (lector de pantalla, DPI, contraste) es de la 1.0.

## Casos límite
| Caso | Qué pasa | RF |
|---|---|---|
| Contraseña maestra con `ñ`, `€` o emoji | Abre igual aquí y en KeePass | RF-1, RF-10, RF-41 |
| Solo archivo llave, sin contraseña | Válido | RF-1 |
| Carpeta sin permiso de escritura o disco lleno | No se crea o no se guarda; el original queda intacto | RF-23 |
| Bóveda en una carpeta sincronizada que otra app reescribe | No se pisa | RF-24 |
| Dos entradas con el mismo título | Permitido: se identifican por su UUID | RF-38 |
| Bóveda vacía, grupo vacío, búsqueda sin resultados | Estados vacíos con una acción | RF-64, `docs/UX.md` |
| Datos de otra app que superan nuestros límites | Se conservan y se muestran; el límite solo aplica a lo que se escribe aquí | RF-25, RF-40 |
| Campos con `<script>`, `<img onerror>`, `javascript:` | Texto literal; la URL no se abre | RF-78, RF-49 |
| Bloqueo automático con un formulario a medio llenar | El borrador se pierde (ADR-0002) | RF-32 |
| Bloqueo mientras un guardado está colgado | Bloquea en ≤ 1 s | RF-30, RF-33 |
| El usuario copió otra cosa antes de la limpieza | Se respeta | RF-73 |
| Bóveda sin papelera o con la papelera desactivada | Se crea y se activa al primer borrado | RF-47 |
| Ajustes dañados o editados a mano | Valores por defecto | RF-82 |
| Archivo de 0 bytes, texto plano, KDBX truncado | `ArchivoDanado` | RF-13 |

## Fuera de alcance
- Todo lo del navegador (spec 002) e importar o exportar (spec 003). **No hay exportación en la 0.1**, por eso el
  requisito "exportar pide la maestra" de AMENAZAS no aplica todavía.
- Cambiar la contraseña maestra o el archivo llave de una bóveda existente, y cambiar su KDF.
- Editar campos propios, iconos, vencimiento, auto-escritura y plantillas: se conservan y, los campos, se ven.
- Varias bóvedas abiertas a la vez, combinar cambios de dos copias, sincronizar.
- Generador de frases, TOTP, llaves de acceso.
- Inglés, lector de pantalla completo y la dirección visual definitiva (la aprueba el dueño sobre un mock).
- El patrón Isolation de Tauri y el fuzzing de la cabecera: `docs/BACKLOG.md`, con su motivo.

## Decisiones por defecto [por confirmar]
| # | Decisión | Por qué | Si se cambia |
|---|---|---|---|
| D1 | Bloqueo automático a los 5 min; opciones 1, 5, 15, 30; sin "nunca" | Un gestor que nunca se bloquea falla el pilar de seguridad | Un valor |
| D2 | El portapapeles se limpia a los 20 s (10 a 120) | KeePass usa 12 s y KeePassXC 10 s; 20 s da tiempo de pegar con calma | Un valor |
| D3 | Un solo respaldo `<archivo>.bak` junto a la bóveda | Recupera el error más común sin llenar la carpeta | Un número |
| D4 | Archivo llave opcional y recomendado | `docs/PROYECTO.md` | Texto de P2 |
| D5 | KDBX 3.1 en solo lectura, con "convertir" a un archivo nuevo | ADR-0001 | — |
| D6 | Borrar manda a la papelera (como KeePass) | Borrar por error una contraseña es caro | Recortable 3 |
| D7 | Cada operación confirmada se guarda sola; no hay botón "Guardar" de la bóveda | ADR-0002 | — |
| D8 | Las notas se ven al abrir la entrada (no son campo protegido en KDBX) y sí entran a la búsqueda | Así las tratan KeePass y KeePassXC | RF-43, RF-59 |
| D9 | La contraseña maestra débil avisa, no bloquea | `docs/UX.md` P2 | RF-7 |
| D10 | El desinstalador con ventana ya ofrece borrar los datos de la app; el silencioso no los borra | Es el comportamiento del instalador NSIS de Tauri; cierra el ítem del backlog | — |
| D11 | Interfaz funcional sin dirección visual hasta que el dueño apruebe un mock | El dueño no estaba para aprobarlo | Solo estilos |

## Criterios de finalización
- Todos los RF no recortados con al menos una prueba automática en verde (`validacion.md`).
- Matriz de completitud en verde: crear, ver, editar, mover y borrar para entradas y grupos, probado en la ventana real.
- Una bóveda creada y editada aquí abre en KeePass 2.62 y KeePassXC 2.8 sin pérdida.
- Demo del flujo principal de `docs/UX.md` en la ventana real, sin errores.
- Release `v0.1.0` instalable y actualizable desde la `v0.0.2`.

## Dudas abiertas
- [NECESITA ACLARACIÓN: ¿cambiar la contraseña maestra entra en la 0.1 o espera a la 1.0? Supuesto aplicado: espera;
  hoy quien la quiera cambiar lo hace en KeePass.]
- [NECESITA ACLARACIÓN: ¿la app debe bloquearse también al minimizarla? Supuesto aplicado: no; solo por tiempo, por
  orden del dueño y, si no se recorta, por la sesión de Windows.]

<!-- Plantilla adaptada de mouredev/hello-sdd (Apache-2.0). -->
