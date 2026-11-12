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
  dark-panel: "#f2f2ee"
  dark-panel-hover: "#dcdcd6"
  dark-on-panel: "#16181d"
  dark-on-panel-muted: "#4a4f59"
  dark-danger: "#ff7a85"
  dark-on-danger: "#16181d"
  dark-confirmation: "#4cc38a"
typography:
  entry-title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "2.75rem"
    fontWeight: 800
    lineHeight: 1.05
    letterSpacing: "-0.01em"
  list-title:
    fontFamily: "Overpass Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "1.875rem"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.01em"
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
  stamp:
    fontFamily: "Overpass Mono Variable, Cascadia Mono, Consolas, monospace"
    fontSize: "0.75rem"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "0.04em"
rounded:
  sm: "3px"
  md: "4px"
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
  bar-height: "64px"
  rail-width: "272px"
  badge: "26px"
  row-comfortable: "44px"
  row-compact: "32px"
  control-comfortable: "36px"
  control-compact: "32px"
  focus-ring: "3px"
  content-width: "40rem"
components:
  signal-bar:
    backgroundColor: "{colors.signal}"
    textColor: "{colors.on-signal}"
    height: "{spacing.bar-height}"
    padding: "0 20px"
  group-rail:
    backgroundColor: "{colors.signal}"
    textColor: "{colors.on-signal}"
    width: "{spacing.rail-width}"
    padding: "16px 12px 12px"
  rail-item-selected:
    backgroundColor: "{colors.signal-panel}"
    textColor: "{colors.on-signal-panel}"
    rounded: "{rounded.md}"
    height: "{spacing.row-comfortable}"
  group-badge:
    backgroundColor: "{colors.line-1}"
    textColor: "{colors.on-line}"
    rounded: "{rounded.round}"
    size: "{spacing.badge}"
    typography: "{typography.label}"
  entry-row:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    height: "{spacing.row-comfortable}"
    padding: "8px 24px 8px 62px"
  entry-row-selected:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.on-panel}"
  copied-stamp:
    textColor: "{colors.confirmation}"
    typography: "{typography.stamp}"
    rounded: "{rounded.sm}"
    padding: "2px 7px"
  field-row:
    backgroundColor: "{colors.work-ground}"
    textColor: "{colors.ink}"
    padding: "8px 0"
  button-neutral:
    backgroundColor: "{colors.work-ground}"
    textColor: "{colors.ink}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "0 12px"
  button-primary:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.on-panel}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "0 12px"
  button-primary-hover:
    backgroundColor: "{colors.panel-hover}"
  button-danger:
    backgroundColor: "{colors.work-ground}"
    textColor: "{colors.danger}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
  button-danger-hover:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.on-danger}"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "{spacing.control-comfortable}"
    padding: "4px 12px"
  notice:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
  dialog:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "24px"
    width: "30rem"
---

# Design System: AlmarioKS

## Overview

**Creative North Star: "La señalética del andén"**

AlmarioKS se dibuja como la señalización de una estación de transporte: un armazón negro de señal (barra superior y riel de grupos) que orienta, y un fondo de trabajo claro y neutro donde se lee. Cada grupo de la bóveda es una línea con su color; una cuenta se encuentra como se encuentra un andén. El mundo rechaza el arreglo por defecto de la categoría (barra lateral gris, un acento azul, candados).

La densidad es de herramienta de uso diario: filas de 44 px, tipografía de señalización (Overpass) con su monoespaciada solo para datos, bordes de 1 px y una sola regla gruesa (2 px, tinta) bajo los títulos de formularios y sobre la lista de campos. No hay sombras en reposo; la profundidad la da el contraste entre el panel de señal y el fondo. El gesto propio es el estado estampado en la fila ("COPIADO · 14 s"), que se queda mientras el portapapeles conserva el valor.

El sistema se verificó en la ventana por defecto (1120 × 720) y en 800 × 600 y 480 × 360 según las capturas de revisión; los huecos conocidos están al final.

**Key Characteristics:**
- Panel negro de señal para el armazón; panel de señal también dentro del trabajo (fila seleccionada, acción principal).
- Cuatro colores de línea que solo nombran grupos; rojo solo para peligro; verde oscuro solo para confirmar.
- Distintivo redondo con la inicial del grupo, texto oscuro sobre el color.
- Datos (fechas, conteos, secretos, rutas de archivo) en monoespaciada; todo lo demás en Overpass.
- Estado leído por palabras, trama o forma, nunca por color solo.

## Colors

Paleta de dos mundos: un armazón de señal casi negro y un fondo de trabajo gris cálido, con cuatro colores de línea saturados como única voz cromática. Los tokens se nombran por rol, no por matiz; los componentes leen roles y el tema oscuro los redefine.

### Primary
- **Panel de señal** (`panel` #16181d; en oscuro `dark-panel` #f2f2ee): la fila de entrada seleccionada y la acción principal del área de trabajo (texto `on-panel`). Se invierte en el tema oscuro, igual que el grupo seleccionado en el riel.
- **Placa de señal** (`signal-panel` #ffffff, texto `on-signal-panel`): grupo seleccionado y acción principal dentro del armazón; es blanca en ambos temas.

### Secondary: familia de líneas
- **Línea 1, azul** (#6aa5ff), **Línea 2, naranja** (#f26a1b), **Línea 3, verde** (#1e9e6a), **Línea 4, amarillo** (#f2b705): en orden, solo para nombrar grupos (distintivo, ruta, paradas). El texto sobre cualquier línea es siempre `on-line` (#16181d).
- **Neutro de línea** (`line-neutral` #c9cedb): la papelera y lo que cuelga de ella.
- **Estación** (`line-station` #ffffff, aro `on-line`): el grupo raíz, donde se cruzan todas las líneas.

### Tertiary: estados
- **Peligro** (`danger` #d7263d; sobre señal `danger-on-signal` #ff7a85; oscuro `dark-danger` #ff7a85): errores, acciones destructivas, borde del aviso de problema.
- **Confirmación** (`confirmation` #157a52; oscuro `dark-confirmation` #4cc38a): el sello "copiado" y el nivel óptimo del medidor de fortaleza.

### Neutral
- **Fondo de trabajo** (`work-ground` #f6f6f3; oscuro #191b21), **Superficie** (`surface` #ffffff; oscuro #20232a): fondo de pantalla y de lista, diálogos, campos.
- **Tinta** (`ink` #16181d; oscuro #f2f2ee) y **Tinta secundaria** (`ink-muted` #565b66; oscuro #a7adba).
- **Regla** (`rule` #ddddd6; oscuro #343842) y **Borde de control** (`control-border` #868b95; oscuro #6b7280).
- **Armazón**: `signal` #16181d (oscuro #0c0d10), `signal-raised` #23262e, `signal-rule` #2e323b, `signal-border` #6b7280, `on-signal` #ffffff, `on-signal-muted` #c3cae0.
- Los fondos de pasar el puntero son tintas con alfa (7 % claro, 9 % oscuro, 9 % blanco sobre señal); viven en el CSS, no en el esquema.

### Named Rules
**The Line-Names-Groups Rule.** Un color de línea nombra un grupo y nada más: no es acento, ni estado, ni decoración. La asignación es determinista: los grupos de primer nivel toman las líneas 1 a 4 en orden de posición y vuelven a empezar al pasar de cuatro (el distintivo los distingue por su inicial); los subgrupos heredan la línea de su grupo de primer nivel y se dibujan con aro en lugar de relleno; la papelera es siempre neutra y no ocupa posición.

**The Red-Is-Danger Rule.** El rojo significa peligro y solo peligro; nunca es una línea. En reposo, enviar a la papelera y las acciones raras del riel se leen como acciones normales: se pintan rojas bajo el puntero, con foco y en su diálogo. El botón de peligro con borde rojo solo vive dentro de la papelera ("Borrar definitivamente").

**The One-Ink Name Rule.** El nombre de la marca va en una sola tinta (la del armazón), nunca partido en colores.

**The Dual-Theme AA Rule.** Cada par texto/fondo cumple contraste AA en claro y en oscuro (comprobado por cálculo al definir los tokens); un token nuevo se añade a los dos temas.

## Typography

**Display / Body Font:** Overpass Variable (con Segoe UI, system-ui, sans-serif), empaquetada, OFL-1.1.
**Data Font:** Overpass Mono Variable (con Cascadia Mono, Consolas, monospace), solo para datos.

**Character:** tipografía de señalización: pesos extremos (800 para títulos, 400 para lectura, 600 para controles), cuerpo de 15 px, sin serifas ni display ornamental.

### Hierarchy
- **Entry title** (800, 2.75rem/44 px, 1.05): título de la entrada en el detalle; baja a 1.875rem cuando el detalle mide ≤ 25rem.
- **List title** (800, 1.875rem/30 px, 1): encabezado del grupo en la lista y título de las pantallas de acceso; baja a 1.5rem en lista estrecha.
- **Heading** (800, 1.5rem, 1.1): h1/h2 por defecto, formulario de entrada en detalle estrecho.
- **Title** (800, 1.125rem/18 px): h3/h4, título del grupo en el riel, de diálogos.
- **Body** (400, 0.9375rem/15 px, 1.45): lectura general, filas del riel.
- **Control** (600, 0.875rem/14 px, 1.2): botones.
- **Label** (600, 0.8125rem/13 px): etiquetas de campo, sugerencias, distintivo (800).
- **Caption** (0.75rem/12 px): estado, pie del riel, etiquetas.
- **Data** (Mono 400, 13 px; tabulares): fechas, conteos del riel, secretos, rutas de archivo, atajos de teclado.
- **Stamp** (Mono 600, 12 px, 0.04em, mayúscula solo en la palabra): "COPIADO · 14 s".

### Named Rules
**The Mono-Only-For-Data Rule.** La monoespaciada es para datos que se leen carácter a carácter o se alinean dígito bajo dígito; nunca para títulos ni etiquetas.

**The Quiet-Weight Rule.** El peso 800 es para títulos y distintivos; los controles usan 600 y la lectura 400.

## Layout

Una sola rejilla de 4 px (pasos 4, 8, 12, 16, 20, 24, 32, 40). Ventana por defecto 1120 × 720, mínimo 480 × 360. La pantalla es una rejilla de tres filas (barra de 64 px, trabajo, franja de estado); con la bóveda abierta, riel de grupos | trabajo, y el trabajo se parte en lista | detalle.

- **Riel:** 272 px (208 px bajo 1100 px; 56 px bajo 720 px, solo distintivos). Solo la lista de grupos se desplaza; las acciones de grupo quedan al pie (en columna con etiqueta a 1100 px o más; fila de botones de icono entre 720 y 1099 px o con alto ≤ 640 px; columna de botones de 44 px bajo 720 px).
- **Lista | detalle:** 2fr / 3fr (mínimo 15 rem en la lista) de 720 a 1099 px; mínimo 18 rem en la lista de 1100 a 1289 px; lista fija de 456 px desde 1290 px. Bajo 720 px se muestra una sola zona: la lista hasta elegir una entrada; entonces el detalle y un botón para volver.
- **Contenedores:** la lista (`list`) se pliega a modo estrecho a 22rem o menos (encabezado de 24 px, botón de ancho completo, sin fechas ni direcciones, el sello pasa a su línea); el detalle (`detail`) reduce el título a 25rem o menos y pasa a campos compactos a 30rem o menos (etiqueta de 5.5rem, "Copiar" sin "contraseña", barra de oculto de 5 periodos); los avisos (`banner`) apilan sus acciones a 46rem o menos.
- **Alto ≤ 480 px:** barra de 48 px, filas y controles de 32 px, fila de "Creada" oculta para que "Copiar" quede a la vista.
- **Densidad:** cómoda (activa): fila 44 px, control 36 px, fila de lista de 60 px como mínimo (44 + 16). Compacta (`data-density="compact"`): fila 32 px, control 32 px, una sola línea por fila. La compacta existe en el CSS pero ningún control la activa y nunca se verificó visualmente.
- **Foco:** contorno de 3 px en `focus` (tinta; blanca sobre señal y sobre panel), desplazado 2 px; en filas, hacia dentro.
- **Detalle:** campos en filas de tres columnas: etiqueta (7.5rem) | valor | acciones al extremo, separadas por reglas de 1 px bajo una regla de 2 px.
- **Movimiento:** solo `transform` y `opacity`: el sello entra en 180 ms (`cubic-bezier(0.16, 1, 0.3, 1)`), el botón cede 1 px al pulsarse y la barra de progreso indeterminada marcha en 700 ms lineales; `prefers-reduced-motion` lo reduce a 0.01 ms.

## Elevation & Depth

Plano por defecto. La profundidad es tonal: el armazón negro contra el fondo claro, la superficie blanca de la lista contra el fondo de trabajo, y el panel de señal invertido para lo seleccionado. La única sombra es la de los diálogos modales (`0 12px 32px rgb(22 24 29 / 22%)`; oscuro `0 12px 32px rgb(0 0 0 / 50%)`), sobre un velo (`rgb(22 24 29 / 55%)`; oscuro `rgb(0 0 0 / 65%)`). El estado de error añade un anillo interior de 1 px, no una sombra.

### Named Rules
**The Flat-Until-Modal Rule.** Nada proyecta sombra salvo un diálogo nativo abierto. Sin sombras duras desplazadas ni brillos.

## Shapes

Esquinas casi rectas: 4 px en botones, campos, avisos y diálogos; 3 px en etiquetas, sellos y atajos; 0 en medidores y avisos de franja. Lo redondo está reservado al distintivo de grupo (círculo de 26 px) y a las paradas de la ruta. Los subgrupos son un aro (2.5 px) en el color de la línea; la estación es blanca con aro oscuro de 2 px. La ruta de un grupo se dibuja como paradas: puntos de 14 px con borde de tinta de 2 px unidos por tramos de 4 px × 56 px (24 px en detalle estrecho) en el color de la línea.

## Components

### Barra de señal y riel de grupos
Armazón negro (`signal`); dentro de él los mismos roles apuntan a la paleta de señal, de modo que botones, campos y errores se adaptan sin reglas aparte. Barra: marca de 28 px, nombre en una tinta, búsqueda ancha (máx. 35rem, atajo en `kbd` que desaparece al escribir), cuenta regresiva de bloqueo en dato monoespaciado y "Bloquear" sin candado. Riel: cabecera con el nombre de la bóveda, lista de grupos con distintivo, nombre y conteo (mono, tabular); el grupo seleccionado es una placa blanca con texto de señal; al pie, las acciones de grupo y el estado de guardado.

### Lista de entradas
Una tabla real dibujada como lista: título, usuario · dirección, y la fecha (mono) al cierre. Distintivo del grupo a la izquierda de cada fila. La fila seleccionada es un panel de señal (`panel`), con foco invertido. **Sello "copiado · N s":** recuadro de 1.5 px en `confirmation`, inclinado -2°, sustituye la fecha de la fila y se queda mientras dura el portapapeles (en la fila seleccionada toma `on-panel`). En una lista estrecha el sello baja a su propia línea.

### Detalle
Ruta como paradas, título de 44 px, fechas en mono, y filas etiqueta | valor | acciones. **Barra de oculto:** un secreto oculto es una barra de ancho fijo (12 periodos de 13 px: marca de 9 px y hueco de 4 px; 5 periodos en detalle estrecho), así no revela la longitud ni termina en una astilla. La contraseña generada se muestra en una caja por carácter: los dígitos subrayados y los símbolos enmarcados, a peso 700, para distinguirlos sin color.

### Fortaleza
Medidor de 9rem × 12 px con tres tramas: óptimo, rayas de 135° densas en `confirmation`; medio, rayas finas en tinta; bajo, rayas verticales en `danger`. Siempre junto a su texto; nunca solo color.

### Botones
- **Neutro (por defecto):** transparente, borde de 1 px `control-border`, 4 px de radio, 36 px de alto, 600/14 px; hover con velo de tinta.
- **Primario:** relleno `panel` con `on-panel` (sobre el armazón, la placa blanca); es "Copiar" en el detalle.
- **A la papelera:** neutro en reposo; rojo solo con hover y foco.
- **Peligro:** texto y borde `danger`; con el puntero se rellena. Solo dentro de la papelera y en sus diálogos.
- **Icono:** cuadrado de 36 px, icono propio de un solo trazo (1.8, `currentColor`, rejilla de 24 px, 18 px en pantalla); lleva nombre y `title`.
- Estados: deshabilitado al 50 %, pulsado baja 1 px.

### Campos y avisos
Campo: superficie, borde 1 px `control-border`, hover con borde de tinta, error con borde `danger` y anillo interior de 1 px más mensaje en palabras debajo. Aviso: franja con borde, icono y palabras; el que informa un problema cierra con regla de 2 px `danger`. En el área de trabajo los avisos cruzan todo el ancho, con las acciones al extremo.

### Diálogos nativos
`<dialog>` modal, 30rem, superficie y radio 4 px, título sobre una regla de 2 px en tinta, texto de qué hace, acciones al final. Toma otra vez los roles de área de trabajo aunque se monte dentro del armazón. La acción destructiva nunca es la predeterminada.

### Pantallas de acceso (bienvenida, crear, desbloquear)
El armazón se vuelve tótem de estación: panel de señal en el lugar del riel con la marca de 72 px y el nombre; la tarea va en el fondo de trabajo, en una columna (máx. 30rem) bajo una regla de 2 px; las bóvedas recientes son filas como las de la lista, cada una una estación con su distintivo. Bajo 720 px el tótem se pliega a la barra superior.

## Do's and Don'ts

### Do:
- **Do** nombrar cada grupo con su línea (distintivo, ruta) y tomar el color de `--line-N`; texto siempre `on-line`.
- **Do** añadir cada rol nuevo a `tokens.css` y a `theme-dark.css`, y comprobar AA en ambos.
- **Do** leer todo estado por palabra, trama o forma además de color (fortaleza, sello, avisos).
- **Do** usar Overpass Mono solo para datos; iconos propios de un trazo (1.8) en `currentColor`.
- **Do** mantener el CSS sin estilos en línea (CSP `style-src 'self'`): todo valor dinámico viaja por clases o atributos `data-*`.
- **Do** mantener los destinos táctiles de 32 px como mínimo.

### Don't:
- **Don't** usar un color de línea como acento, estado o decoración.
- **Don't** usar rojo para nada que no sea peligro, ni dejar un botón rojo en reposo fuera de la papelera.
- **Don't** dibujar candados, escudos ni llaves, ni escribir promesas como "seguro" o "auditado".
- **Don't** partir el nombre de la marca en varios colores.
- **Don't** usar la monoespaciada para títulos o etiquetas.
- **Don't** dar sombra a nada fuera de un diálogo abierto.
- **Don't** hacer de la acción destructiva la predeterminada de un diálogo.

## Known gaps (revisión de cierre abierta, no resueltos)

- **La lista cae en modo estrecho en la ventana por defecto:** a 1120 × 720 mide 338 px, por debajo del umbral de 22rem: encabezado de 24 px, "Nueva entrada" a ancho completo, sin fechas ni direcciones.
- **Valores partidos a mitad de palabra** a 800 × 600 (por ejemplo, una dirección de correo en dos líneas).
- **Riel a 480 × 360:** los iconos de acción de grupo apilados ocupan más que la lista de grupos.
- **Densidad compacta sin verificar** y sin control que la active.
- **Fortaleza no se muestra en el detalle** (el núcleo no la entrega sin revelar la contraseña); sí en crear y en el generador.
- **Tema oscuro y armazón tiran a azul** (`dark-signal` #0c0d10 y grises con matiz frío) frente al "negro de señal" del contrato.
- **El verde de confirmación y el verde de la línea 3 comparten matiz**; el sello y la línea no se confunden hoy por forma y texto, pero el riesgo existe.
- Sin revisar: búsqueda y "sin resultados", sello en la fila seleccionada, cuenta regresiva en captura, tema oscuro a 1120/800/480, grupo vacío, quinto grupo (vuelve a la línea 1), diálogos de renombrar y mover, panel de actualización.
