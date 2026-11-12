---
name: AlmarioKS
description: Gestor de contraseñas de escritorio con señalética de transporte; cada grupo es una línea con su color.
colors:
  work-ground: "#f6f6f3"
  surface: "#ffffff"
  ink: "#16181d"
  ink-muted: "#565b66"
  rule: "#ddddd6"
  control-border: "#868b95"
  signal: "#16181d"
  signal-raised: "#23262e"
  signal-rule: "#2e323b"
  signal-border: "#6b7280"
  on-signal: "#ffffff"
  on-signal-muted: "#c3cae0"
  signal-panel: "#ffffff"
  signal-panel-hover: "#dcdcd6"
  on-signal-panel: "#16181d"
  panel: "#16181d"
  panel-hover: "#2a2d35"
  on-panel: "#ffffff"
  on-panel-muted: "#c3cae0"
  danger: "#d7263d"
  danger-on-signal: "#ff7a85"
  on-danger: "#ffffff"
  confirmation: "#157a52"
  line-1: "#6aa5ff"
  line-2: "#f26a1b"
  line-3: "#1e9e6a"
  line-4: "#f2b705"
  line-neutral: "#c9cedb"
  line-station: "#ffffff"
  on-line: "#16181d"
  dark-work-ground: "#191b21"
  dark-surface: "#20232a"
  dark-ink: "#f2f2ee"
  dark-ink-muted: "#a7adba"
  dark-rule: "#343842"
  dark-control-border: "#6b7280"
  dark-signal: "#0c0d10"
  dark-signal-raised: "#1c1f26"
  dark-signal-rule: "#2a2d36"
  dark-signal-panel: "#f2f2ee"
  dark-panel: "#f2f2ee"
  dark-panel-hover: "#dcdcd6"
  dark-on-panel: "#16181d"
  dark-on-panel-muted: "#4a4f59"
  dark-danger: "#ff7a85"
  dark-on-danger: "#16181d"
  dark-confirmation: "#4cc38a"
  on-panel-ok: "#4cc38a"
  dark-on-panel-ok: "#157a52"
typography:
  entry-title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 800
    lineHeight: 1.15
  screen-title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 800
    lineHeight: 1.15
  list-title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 800
    lineHeight: 1.2
  heading:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 800
    lineHeight: 1.1
  title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 800
    lineHeight: 1.1
  body:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.45
  control:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 600
    lineHeight: 1.2
  label:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 600
    lineHeight: 1.45
  caption:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.45
  data:
    fontFamily: "Overpass Mono Variable, Cascadia Mono, Consolas, monospace"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.45
rounded:
  sm: "3px"
  md: "4px"
  card: "6px"
  dialog: "8px"
  round: "50%"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "20px"
  "6": "24px"
  "8": "32px"
  "10": "40px"
  bar-height: "56px"
  rail-width: "232px"
  dialog-width: "560px"
  entry-window-width: "600px"
  table-head-height: "36px"
  table-row-height: "52px"
  table-min-width: "756px"
  entry-badge-list: "32px"
  entry-badge-window: "48px"
  password-column: "112px"
  date-column: "80px"
  actions-column: "92px"
  generator-exact-width: "80px"
  brand-panel-width: "240px"
  create-column-width: "520px"
  badge: "26px"
  row-comfortable: "44px"
  row-compact: "32px"
  control-comfortable: "44px"
  control-compact: "32px"
  control-inline: "36px"
  focus-ring: "3px"
  content-width: "40rem"
components:
  signal-bar:
    backgroundColor: "{colors.dark-signal}"
    textColor: "{colors.on-signal}"
    height: "{spacing.bar-height}"
    padding: "0 16px"
  group-rail:
    backgroundColor: "{colors.dark-signal}"
    textColor: "{colors.on-signal}"
    width: "{spacing.rail-width}"
    padding: "12px 8px 0"
  rail-item-selected:
    backgroundColor: "{colors.dark-signal-panel}"
    textColor: "{colors.dark-on-panel}"
    rounded: "{rounded.md}"
    height: "{spacing.row-comfortable}"
  group-badge:
    backgroundColor: "{colors.line-1}"
    textColor: "{colors.on-line}"
    rounded: "{rounded.round}"
    size: "{spacing.badge}"
    typography: "{typography.label}"
  table-head:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink-muted}"
    height: "{spacing.table-head-height}"
    padding: "0 16px"
  entry-row:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    height: "{spacing.table-row-height}"
    padding: "0 16px"
  entry-row-selected:
    backgroundColor: "{colors.dark-panel}"
    textColor: "{colors.dark-on-panel}"
  row-copy:
    backgroundColor: "{colors.dark-on-panel}"
    textColor: "{colors.dark-panel}"
    rounded: "{rounded.md}"
    size: "{spacing.control-comfortable}"
  entry-card:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.card}"
  card-row:
    textColor: "{colors.dark-ink}"
    height: "56px"
    padding: "4px 8px 4px 16px"
  toast:
    backgroundColor: "{colors.dark-panel}"
    textColor: "{colors.dark-on-panel}"
    rounded: "{rounded.md}"
    padding: "10px 16px"
  button-neutral:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "0 14px"
  button-primary:
    backgroundColor: "{colors.dark-panel}"
    textColor: "{colors.dark-on-panel}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "0 16px"
  button-primary-hover:
    backgroundColor: "{colors.dark-panel-hover}"
  button-danger:
    textColor: "{colors.dark-danger}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
  button-danger-hover:
    backgroundColor: "{colors.dark-danger}"
    textColor: "{colors.dark-on-danger}"
  input:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "4px 12px"
  menu:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.card}"
    padding: "4px"
  dialog:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.md}"
    padding: "24px"
    width: "30rem"
  entry-dialog:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.dialog}"
    width: "{spacing.dialog-width}"
  entry-window:
    backgroundColor: "{colors.dark-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.dialog}"
    width: "{spacing.entry-window-width}"
---

# Design System: AlmarioKS

## Overview

**Creative North Star: "La señalética del andén"**

AlmarioKS se dibuja como la señalización de una estación de transporte: un armazón de señal casi negro (barra superior y riel de grupos) que orienta, y un fondo de trabajo oscuro donde se lee. Cada grupo de la bóveda es una línea con su color; una cuenta se encuentra como se encuentra un andén. La app es siempre oscura (`styles.css` importa `theme-dark.css` sin condición); los valores claros siguen en `tokens.css` sin uso en pantalla. Lo único claro es lo seleccionado y la acción principal: una placa de señal. El mundo rechaza el arreglo por defecto de la categoría (barra lateral gris, un acento azul, escudos y llaves).

La composición es de herramienta de uso diario y única (la ventana no baja de 1000 × 640): una barra de 56 px arriba y, debajo, dos columnas a toda la altura: riel que solo navega | lista ancha en tabla. No hay panel de detalle: la entrada se abre en una ventana modal de 600 px (doble clic en el título o su botón de la fila elegida). Crear y editar una entrada es otro diálogo modal con el generador dentro del campo de contraseña. Tipografía de señalización (Overpass) con su monoespaciada solo para datos, controles de 44 px, bordes de 1 px y una sola regla gruesa (2 px, tinta) bajo los títulos de los formularios de acceso y de los diálogos nativos. La profundidad es tonal; la sombra solo la llevan las superficies flotantes (diálogos y menús).

El sistema se verificó con las capturas de la app instalada a 1120 × 720 (bóveda, entrada abierta, crear bóveda, nueva entrada con las opciones del generador); los huecos conocidos están al final. El contrato de dirección de `.impeccable/surfaces/` quedó desactualizado en composición (describe la distribución anterior, con tres columnas); esta página manda.

**Key Characteristics:**
- Armazón de señal negro; placa clara de señal solo para lo seleccionado y la acción principal (fila de entrada elegida, grupo del riel, "Copiar contraseña", "Guardar entrada", aviso de copiado, opción elegida de archivo llave).
- Cuatro colores de línea que nombran grupos; rojo solo para peligro; verde solo para confirmar.
- Distintivo redondo con la inicial (del título de la entrada en la fila y en la ventana de la entrada) sobre el color de su grupo; el riel marca cada grupo con un punto de ese color.
- Datos (conteos, secretos, atajos, nombres de archivo) en monoespaciada; todo lo demás en Overpass.
- Estado leído por palabras, forma o trama, nunca por color solo.

## Colors

Paleta de dos mundos: un armazón de señal casi negro y un fondo de trabajo gris oscuro, con cuatro colores de línea saturados como única voz cromática. Los tokens se nombran por rol, no por matiz; los componentes leen roles, y `theme-dark.css` los redefine (los valores que se ven son los `dark-*` del esquema; los claros viven en `tokens.css` sin uso en pantalla).

### Primary
- **Placa de señal** (`dark-panel` #f2f2ee, texto `dark-on-panel` #16181d; claro `panel` #16181d): fila de la tabla elegida, acción principal ("Copiar contraseña", "Guardar entrada", "Crear bóveda"), aviso de copiado y opción elegida del archivo llave. Su texto secundario es `dark-on-panel-muted` (#4a4f59).
- **Placa del riel** (`dark-signal-panel` #f2f2ee, texto `on-signal-panel` #16181d): grupo abierto en el riel y "Todas las entradas"; la misma placa clara que en el trabajo.

### Secondary: familia de líneas
- **Línea 1, azul** (#6aa5ff), **Línea 2, naranja** (#f26a1b), **Línea 3, verde** (#1e9e6a), **Línea 4, amarillo** (#f2b705): en orden, solo para nombrar grupos (punto del riel, distintivo, punto bajo el título). El texto sobre cualquier línea es siempre `on-line` (#16181d).
- **Neutro de línea** (`line-neutral` #c9cedb): la papelera y lo que cuelga de ella.
- **Estación** (`line-station` #ffffff, aro `on-line`): el grupo raíz.

### Tertiary: estados
- **Peligro** (`dark-danger` #ff7a85, texto sobre él `dark-on-danger` #16181d; claro `danger` #d7263d): errores, acciones destructivas, borde del aviso de problema.
- **Confirmación** (`dark-confirmation` #4cc38a; claro `confirmation` #157a52): relleno del medidor de fortaleza en el diálogo de entrada (`--ok`).
- **Confirmación sobre placa** (`dark-on-panel-ok` #157a52; claro `on-panel-ok` #4cc38a): la marca verde del aviso de copiado, sobre la placa clara. Rol nuevo (`--on-panel-ok`): el verde que se lee sobre la placa, que invierte con el tema.

### Neutral
- **Fondo de trabajo** (`dark-work-ground` #191b21) y **Superficie** (`dark-surface` #20232a): fondo de pantalla; la lista (`--surface`), menús y diálogos. En la ventana de la entrada la tarjeta de filas va sobre el fondo (`--ground`), no la superficie, y en el diálogo de entrada también los campos, el de la contraseña y las opciones del generador.
- **Tinta** (`dark-ink` #f2f2ee) y **Tinta secundaria** (`dark-ink-muted` #a7adba).
- **Regla** (`dark-rule` #343842) y **Borde de control** (`dark-control-border` #6b7280).
- **Armazón**: `dark-signal` #0c0d10, `dark-signal-raised` #1c1f26, `dark-signal-rule` #2a2d36, `signal-border` #6b7280, `on-signal` #ffffff, `on-signal-muted` #c3cae0.
- Los fondos de pasar el puntero son tintas con alfa (9 % `rgb(242 242 238)` en el trabajo, 9 % blanco sobre señal) y viven en el CSS, no en el esquema.

### Named Rules
**The Line-Names-Groups Rule.** Un color de línea nombra un grupo y nada más: no es acento, ni estado, ni decoración. La asignación es determinista: los grupos de primer nivel toman las líneas 1 a 4 en orden de posición y vuelven a empezar al pasar de cuatro; los subgrupos heredan la línea de su grupo de primer nivel y su distintivo se dibuja con aro en lugar de relleno; la papelera es siempre neutra y no ocupa posición.

**The Red-Is-Danger Rule.** El rojo significa peligro y solo peligro; nunca es una línea. En reposo, enviar a la papelera y las acciones raras de los menús se leen como acciones normales: se pintan rojas solo en su opción de menú de peligro, bajo el puntero, con foco y en su diálogo. El botón de peligro con borde rojo solo vive dentro de la papelera ("Borrar definitivamente").

**The One-Ink Name Rule.** El nombre de la marca va en una sola tinta (la del armazón), nunca partido en colores.

**The Light-Plate Rule.** Lo único claro en pantalla es lo seleccionado y la acción principal (la placa). Todo lo demás es oscuro; una placa clara nueva debe significar "esto está elegido" o "esto es lo principal".

**The Dual-Theme AA Rule.** Cada par texto/fondo cumple contraste AA (comprobado por cálculo al definir los tokens); un rol nuevo se añade a `tokens.css` y a `theme-dark.css`, aunque hoy solo se vea el oscuro.

## Typography

**Display / Body Font:** Overpass Variable (con Segoe UI, system-ui, sans-serif), empaquetada, OFL-1.1.
**Data Font:** Overpass Mono Variable (con Cascadia Mono, Consolas, monospace), solo para datos.

**Character:** tipografía de señalización: pesos extremos (800 para títulos, 400 para lectura, 600 para controles), cuerpo de 15 px, sin serifas ni display ornamental.

### Hierarchy
- **Entry title** (800, 1.5rem/24 px, 1.15): título de la ventana de la entrada (`.entry-title`, valor propio en `vault.css`, no un token).
- **Screen title** (800, 1.75rem/28 px, 1.15; `--text-2xl`): títulos de las pantallas de acceso y de crear bóveda (acceso: interlineado 1.05 y -0.01em).
- **List title** (800, 1.25rem/20 px, 1.2; `--text-xl`): encabezado del grupo sobre la tabla, título de los diálogos de formulario, inicial del distintivo de 48 px y nombre de la marca en el tótem.
- **Heading** (800, 1.5rem, 1.1): h1/h2 por defecto.
- **Title** (800, 1.125rem/18 px, 1.1): h3/h4, nombre de la marca en la barra con la bóveda abierta; el nombre de la bóveda en el riel es de 1.0625rem (17 px).
- **Body** (400, 0.9375rem/15 px, 1.45): lectura general, filas del riel y de la lista, valores de la tarjeta.
- **Control** (600, 0.875rem/14 px, 1.2): botones por defecto; en barra, diálogos y ventana de la entrada los botones suben a 15 px (`--text-md`).
- **Label** (600, 0.8125rem/13 px): textos de apoyo, fechas de la tabla y de la ventana, ayuda del pie (`--text-sm`), inicial del distintivo (800); las etiquetas de campo del diálogo van a 14 px/600.
- **Caption** (0.75rem/12 px): pie de estado, atajo `Ctrl K`, fila de rótulos de la tabla (mayúscula con 0.04em, ver Known gaps).
- **Data** (Mono 400, 13 px; tabulares): conteos del riel (600), secretos, la máscara de la tabla (14 px, 0.08em), la casilla de longitud del generador, ruta de archivo, atajos. La fecha de la tabla ("12 nov") va en Overpass 13 px en gris, no en mono.

### Named Rules
**The Mono-Only-For-Data Rule.** La monoespaciada es para datos que se leen carácter a carácter o se alinean dígito bajo dígito; nunca para títulos ni etiquetas.

**The Quiet-Weight Rule.** El peso 800 es para títulos y distintivos; los controles usan 600 y la lectura 400.

## Layout

Una sola rejilla de 4 px (pasos 4, 8, 12, 16, 20, 24, 32, 40). Ventana por defecto 1120 × 720 y mínimo 1000 × 640 (`tauri.conf.json`); no hay composición más estrecha. El `.app` es una rejilla de tres filas (barra de `--bar-h` 56 px, trabajo, franja de estado); con la bóveda abierta la franja de estado desaparece y la barra de bóveda se tiende sobre la barra de la app desde el borde del riel hasta el final.

- **Dos columnas:** `.open-vault` es la rejilla `--rail-w` 232 px | `minmax(0, 1fr)`; la lista toma todo lo que deja el riel (768 px en la ventana mínima). Ya no hay columna de detalle.
- **Barra:** marca de 26 px y nombre de 18 px (16 px de margen) sobre el riel; la búsqueda empieza donde acaba el riel (alto 44 px, hasta 520 px, atajo `Ctrl K` en `kbd`), y al extremo la cuenta regresiva en dato, "Bloquear" (con candado) y Ajustes como icono de 44 px.
- **Riel:** solo navega. Cabecera con la etiqueta y el nombre de la bóveda, "Todas las entradas" con su conteo, grupos con su `+` y, al pie y fijas, la papelera y el estado de guardado; solo la lista de grupos se desplaza (con sombra de pista en el borde donde hay más). Los subgrupos se sangran 20 px por nivel (hasta tres). Lo que se puede pedir de un grupo está en su menú ("más acciones", 36 × 44).
- **Lista:** columna en tres partes: encabezado (nombre del grupo en 20 px, conteo y "Nueva entrada", relleno 14 × 24 px, regla inferior), tabla que se desplaza y pie fijo. La tabla es una tabla real de filas flexibles: fila de rótulos de 36 px (`--entry-head-h`, fija al desplazarse), filas de 52 px (`--entry-row-h`) con regla inferior; columnas Título (flex 1.6, mín. 160) / Usuario (1.2, mín. 120) / Contraseña (112, máscara fija) / Sitio (1.2, mín. 120) / Modificada (80) / Acciones (92), con 16 px a cada extremo y 8 px entre columnas: mínimo de 756 px (`.entries`), que cabe en los 768 de la ventana mínima; el envoltorio solo se desplaza de lado como red de seguridad. Nada en una fila toma dos líneas: el texto largo se corta con puntos suspensivos. Las etiquetas y la ruta del grupo existen en la fila solo para lectores, fuera del flujo.
- **Pie de la lista:** lo que dejó la última orden, el aviso de copiado mientras no hay ventana abierta y una línea de ayuda del doble clic (regla superior, 13 px, relleno 10 × 24 px): título abre la entrada, usuario o contraseña copian, sitio lo abre en el navegador.
- **Ventana de la entrada:** `dialog[data-layout="entry"]`, 600 px (máx. ancho de ventana − 48 px), radio 8. Cabecera (relleno 20 / 12 / 16 / 24 px): distintivo de 48 px, título de 24 px con su grupo y punto de 8 px debajo, "Editar", menú "más" y "Cerrar" de 44 px. Tarjeta a 24 px de cada lado sobre `--ground`, filas de 56 px: etiqueta (104 px) | valor | botones de icono de 44 al extremo; debajo, ambas fechas con hora (13 px, separador de punto dibujado) y, dentro de la ventana, el aviso de copiado pegado al pie. Relleno inferior de 20 px.
- **Diálogo de entrada (nueva/editar):** 560 px (máx. ancho de ventana − 48 px), cabecera y pie quietos, cuerpo con scroll; los campos toman el ancho de la hoja menos dos márgenes de 24 px.
- **Crear bóveda:** panel de marca de 240 px a la izquierda (marca de 56 px, nombre de 26 px, lema) y formulario en una columna centrada de 520 px, a 20 px entre bloques; con alto ≤ 760 px el espaciado baja un paso y las acciones se pegan al pie. Bienvenida y desbloqueo conservan el tótem del riel (232 px, marca de 72 px).
- **Densidad:** cómoda (activa): fila de riel y control 44 px, fila de tabla 52 px. Compacta (`data-density="compact"`): fila de riel y control 32 px (la fila de la tabla baja a 36 px y el distintivo a 24 px). La compacta existe en el CSS pero ningún control la activa.
- **Foco:** contorno de 3 px (`--focus`, tinta; blanca sobre señal; `on-panel` sobre placas), desplazado 2 px; en filas y menús, hacia dentro.
- **Movimiento:** solo `transform` y `opacity`: el botón cede 1 px al pulsarse y la barra de progreso indeterminada marcha en 700 ms lineales; `prefers-reduced-motion` lo reduce a 0.01 ms. Los menús cuelgan de su botón por posicionamiento por ancla (`position-anchor`) y se abren hacia arriba si no hay sitio abajo.

## Elevation & Depth

Plano por defecto. La profundidad es tonal: el armazón (#0c0d10) contra el fondo (#191b21), la superficie (#20232a) de lista, tarjeta y diálogos, y la placa clara para lo seleccionado. Las únicas sombras son las de las superficies flotantes, diálogos y menús (`--shadow`: `0 12px 32px rgb(0 0 0 / 50%)`; claro `0 12px 32px rgb(22 24 29 / 22%)`), sobre un velo en los diálogos (`--backdrop` `rgb(0 0 0 / 65%)`). El estado de error añade un anillo interior de 1 px, no una sombra.

### Named Rules
**The Flat-Until-Floating Rule.** Nada proyecta sombra salvo un diálogo nativo abierto o un menú desplegado. Sin sombras duras desplazadas ni brillos.

## Shapes

Esquinas contenidas en tres pasos: 4 px (`--radius`) en botones, campos, el aviso estándar y el diálogo nativo de confirmación; 6 px (`--radius-card`) en la tarjeta de la ventana de la entrada, menús, opciones del generador y el aviso de crear; 8 px (`--radius-dialog`) en el diálogo de entrada y en la ventana de la entrada; 3 px (`--radius-sm`) en etiquetas y atajos; 0 en los avisos de franja del trabajo. Lo redondo (50 %) es del distintivo, los puntos de color y la marca. Los subgrupos son un aro (2.5 px) en el color de la línea; la estación es blanca con aro oscuro de 2 px. La marca del grupo en el riel es un punto de 12 px; en la ventana de la entrada, uno de 8 px.

## Components

### Barra de señal y riel de grupos
Armazón oscuro (`signal`); dentro de él los mismos roles apuntan a la paleta de señal, de modo que botones, campos y errores se adaptan sin reglas aparte. Barra: marca de 26 px, nombre en una tinta, búsqueda con lupa y atajo, cuenta regresiva de bloqueo en dato monoespaciado, "Bloquear" con candado y sin placa, Ajustes como icono. Riel: grupos de 44 px con punto, nombre y conteo (mono, tabular), el abierto sobre la placa clara; "más acciones" por fila y menú desplegable.

### Lista de entradas (tabla)
Una tabla de verdad: rótulos de columna en 12 px mayúscula `--ink-muted` sobre `--surface`, fijos; filas de 52 px sin placa en reposo, con velo de tinta (`--hover`) bajo el puntero. Título con su distintivo de 32 px (inicial sobre el color del grupo); usuario y sitio en 15 px; contraseña como ocho marcas fijas en la cara de datos (no revela la longitud ni el valor); fecha en gris. La fila elegida es la placa clara (`--panel`) con sus tintas propias y, en la última celda, dos botones de icono de 44: "Copiar contraseña" sobre una placa de la tinta de la fila (`.row-copy`) y "Abrir la entrada" sin placa; las demás filas guardan esos 92 px vacíos. El foco de la fila se dibuja hacia dentro. Doble clic: ver Layout, pie de la lista.

### Ventana de la entrada
Hoja modal de 600 px con cabecera propia (distintivo de 48 px, título de 24 px, grupo con punto, "Editar" con borde sobre `--surface`, menú "más" y "Cerrar" sin placa) y la tarjeta de filas etiqueta | valor | iconos de 44. **Barra de oculto:** un secreto oculto es una barra de ancho fijo (12 periodos de 13 px: marca de 9 px y hueco de 4 px, 156 px); así no revela la longitud ni termina en una astilla. "Copiar contraseña" es el único icono de la tarjeta con placa (principal). Las notas cierran la tarjeta con sus saltos de línea. **Aviso de copiado:** placa clara centrada, con marca verde (`on-panel-ok`) y palabras; dentro de la ventana, pegado al pie mientras esta se desplaza.

### Diálogo de entrada
`<dialog>` modal de 560 px, radio 8, cabecera (título de 20 px, "En el grupo …" con su punto, cerrar de 44) separada del cuerpo por una regla de 1 px; campos Título, Usuario o correo, Contraseña, Sitio y Notas; pie con "Cancelar" (secundario) y "Guardar entrada" (placa). La contraseña comparte una caja con el ojo y "Generar" (36 px, dentro del campo); las opciones del generador se pliegan debajo (enlace "Mostrar/Ocultar opciones del generador") en un panel de `--ground` con radio 6: "Longitud" con un deslizante (8 a 64) y una casilla numérica de 80 px en la cara de datos con el número a la derecha (hasta 500; ayuda debajo: "El deslizante llega a 64. En la casilla puedes escribir hasta 500."), cuatro casillas de tipo y "Evitar caracteres parecidos". La fortaleza se lee en palabras con una barra de 120 × 6 px.

### Fortaleza
Barra de 120 × 6 px, relleno proporcional al nivel (de 20 % a 100 %), siempre junto a su texto ("Fortaleza: muy buena (4 de 4)"); nunca solo color. En crear bóveda es tinta sobre regla; en el diálogo de entrada, `--ok` sobre regla. Las tres tramas rayadas de `base.css` (`meter`) siguen definidas pero la pantalla de crear las sobreescribe (Known gaps).

### Botones
- **Secundario (por defecto):** transparente o sobre la superficie en la ventana de la entrada, diálogo y crear; borde de 1 px `control-border`, 4 px de radio, 44 px de alto, 600/14 px (15 px en barra, ventana de la entrada y pies); hover con velo de tinta.
- **Primario:** placa (`panel` con `on-panel`); es "Copiar contraseña", "Guardar entrada" y "Crear bóveda".
- **Peligro:** texto y borde `danger`; con el puntero se rellena. Solo dentro de la papelera y sus diálogos; en menús, la opción de peligro es solo texto rojo.
- **Icono:** cuadrado de 44 px sin placa, icono propio de un solo trazo (1.8, `currentColor`, rejilla de 24 px, 16–18 px en pantalla); lleva nombre y `title`. El de Ajustes es solo icono con su nombre para lectores.
- **Enlace:** texto subrayado de 14 px con destino de 44 px de alto ("Opciones del generador", "Cambiar ubicación…").
- Estados: deshabilitado al 50 %, pulsado baja 1 px; guardando, el primario queda en su sitio al 70 %.

### Campos y avisos
Campo: superficie (fondo de trabajo en el diálogo de entrada), borde 1 px `control-border`, 44 px de alto, hover con borde de tinta, error con borde `danger` y anillo interior de 1 px más mensaje en palabras debajo. Aviso: franja con borde, icono y palabras; el que informa un problema cierra con regla de 2 px `danger`. En el trabajo los avisos cruzan todo el ancho, con las acciones al extremo.

### Menús
Superficie flotante de `min-width` 208 px (224 px en el de la entrada), radio 6, borde de 1 px, sombra; opciones de 44 px sin borde, texto de 15 px alineado al inicio.

### Diálogos nativos de confirmación
`<dialog>` modal, 30rem, superficie y radio 4 px, título sobre una regla de 2 px en tinta, texto de qué hace, acciones al final. La acción destructiva nunca es la predeterminada.

### Pantallas de acceso (bienvenida, crear, desbloquear)
El armazón se vuelve tótem de estación: panel de señal en el lugar del riel con la marca y el nombre (72 px en bienvenida y desbloqueo; 56 px y 240 px de ancho en crear, con su lema); la tarea va en el fondo de trabajo, en una columna bajo una regla de 2 px (30rem; 520 px en crear). Las bóvedas recientes son filas como las del riel. En desbloquear, el archivo llave propuesto muestra su nombre (o su ruta completa si salió de otra carpeta) y una línea que dice de dónde salió ("junto a la bóveda" o "recordada"); se cambia con "Elegir archivo llave…" y se quita con "Quitar archivo llave". En crear: nombre de la bóveda, dónde se guarda, contraseña maestra con ojo y fortaleza, repetirla, "Archivo llave (opcional)" como tres tarjetas de opción en una fila, con "Sin archivo llave" primero y elegida por defecto, luego "Crear uno nuevo" y "Usar uno mío" (la elegida es una placa clara; su círculo de opción toma `--line-2`, ver Known gaps), con una línea de ayuda debajo y el aviso de que no hay recuperación.

## Do's and Don'ts

### Do:
- **Do** nombrar cada grupo con su línea (punto, distintivo) y tomar el color de `--line-N`; texto siempre `on-line`.
- **Do** añadir cada rol nuevo a `tokens.css` y a `theme-dark.css`, y comprobar AA.
- **Do** reservar la placa clara para lo seleccionado y la acción principal.
- **Do** leer todo estado por palabra, forma o trama además de color (fortaleza, aviso de copiado, avisos).
- **Do** usar Overpass Mono solo para datos; iconos propios de un solo trazo (1.8) en `currentColor`.
- **Do** mantener el CSS sin estilos en línea (CSP `style-src 'self'`): todo valor dinámico viaja por clases o atributos `data-*`.
- **Do** dar a todo destino táctil 44 px (36 px solo para el botón dentro de un campo, cuyo destino llega a 44 con su marco).
- **Do** llevar abrir, crear y editar una entrada a un diálogo modal sobre la lista; la bóveda no tiene panel de detalle.

### Don't:
- **Don't** usar un color de línea como acento, estado o decoración.
- **Don't** usar rojo para nada que no sea peligro, ni dejar un botón rojo en reposo fuera de la papelera.
- **Don't** dibujar escudos ni llaves, ni escribir promesas como "seguro" o "auditado"; el candado existe una sola vez, en "Bloquear", donde nombra la acción.
- **Don't** partir el nombre de la marca en varios colores.
- **Don't** usar la monoespaciada para títulos o etiquetas.
- **Don't** dar sombra a nada que no flote (diálogo o menú).
- **Don't** hacer de la acción destructiva la predeterminada de un diálogo.

## Known gaps (no resueltos)

- **Verificado solo a 1120 × 720** (capturas de la app instalada: bóveda, entrada abierta, crear, nueva entrada con opciones). Sin captura a 1000 × 640 (mínimo): que la tabla (756 px mínimos) quepa en los 768 de la lista sale de la lectura del CSS; el diálogo de editar, los menús desplegados, el desbloqueo con la llave propuesta y la bienvenida no se vieron en captura en esta pasada; la franja de la tabla a ese ancho con barra de desplazamiento vertical tampoco.
- **El contrato de dirección de `.impeccable/surfaces/` describe la distribución anterior** (riel con acciones, lista 2fr/3fr o tres columnas, formulario en el detalle, tema claro); no es verdad de composición. Lo vigente es este archivo y `docs/revisiones/0.1-ux-dueno.md` y `0.1-ronda-2-dueno.md`.
- **Reglas inalcanzables en el CSS:** `gate.css` aún tiene reglas para ventanas menores de 720 y 1100 px; `.back-to-list` está oculto; la densidad compacta no la activa ningún control y nunca se verificó.
- **El punto de "Guardado" del riel usa `--line-3`** (un color de línea como estado), contra la Line-Names-Groups Rule; no se canoniza.
- **Etiquetas en mayúscula pequeña con espaciado** ("BÓVEDA", "GRUPOS" en el riel; los rótulos de columna de la tabla, 12 px/0.04em) son un rótulo tipo eyebrow que la build lleva; los de la tabla nombran columnas reales, pero la forma no es regla del sistema y no debe heredarse como rótulo suelto en pantallas nuevas.
- **Un color de línea como acento de control:** el círculo de las tarjetas de archivo llave usa `accent-color: var(--line-2)` (naranja, visible en captura), contra la Line-Names-Groups Rule; no se canoniza.
- **Tokens sin uso tras la ronda 2:** `--list-w` (340 px) y `--detail-min-w` (420 px) siguen en `tokens.css` pero nada los lee; no son parte del sistema. El título de 24 px de la ventana de la entrada es un valor suelto (`1.5rem`), sin token.
- **Fecha de la tabla fuera de la mono:** el sistema decía "fechas en monoespaciada"; la tabla las pone en Overpass 13 px, y la ventana de la entrada también. No verificado si es intención o deriva.
- **Valores sueltos fuera de escala (detector, sin token):** nombre de marca de 1.625rem/26 px en crear (`gate.css`), nombre de la bóveda en el riel de 1.0625rem/17 px (`rail.css`), radio de 2 px de la barra de oculto (`vault.css`); tres colores literales en `src/preview.tsx` (vista previa solo de desarrollo, fuera del bundle).
- **Fortaleza rayada de `base.css` sin uso visible** (crear la reemplaza por una barra sólida); y no se muestra en la ventana de la entrada (el núcleo no la entrega sin revelar la contraseña).
- **Radio del aviso estándar y del diálogo de confirmación (4 px)** frente al de 6/8 del resto de superficies flotantes: no se unificó.
- **Armazón tira a azul** (`dark-signal` #0c0d10 y grises con matiz frío) frente al "negro de señal" del contrato original.
- **Sin revisar:** búsqueda y "sin resultados" en la tabla (columna de ruta del grupo), cuenta regresiva en captura, grupo vacío, quinto grupo (vuelve a la línea 1), diálogos de renombrar y mover, panel de actualización, tema claro (existe, sin selector)..
