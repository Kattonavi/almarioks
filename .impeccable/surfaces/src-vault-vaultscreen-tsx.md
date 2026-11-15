---
version: 1
slug: "src-vault-vaultscreen-tsx"
primary_target: "src/vault/VaultScreen.tsx"
related_targets: ["src/vault/VaultRoot.tsx","src/App.tsx"]
---

# Bóveda — ventana principal (P1 a P8)

## Scope and visitor mode
Operate. Toda la ventana de la app: bienvenida, crear, desbloquear, bóveda abierta, entrada, generador, grupos y
ajustes. Ventana de escritorio, mínimo 480 × 360, tema claro y oscuro.

## Audience, job, task
Una persona en su PC, a veces con gente al lado, que entra por segundos a buscar una cuenta y copiar su contraseña
sin verla, y de vez en cuando guarda una nueva. Debe saber de un vistazo: dónde busca, en qué grupo está, qué cuenta
tiene seleccionada, cuándo se borra lo copiado y cuándo se bloquea la app.

## Constraints
CSP sin estilos en línea; fuentes e iconos libres y empaquetados (nada remoto); contraste AA; todo con teclado; sin
candados ni escudos de adorno; la interfaz no promete "seguro" ni "auditado". Controles estándar de la web: el mundo
presta tipo, paleta, densidad y un solo gesto propio.

## Direction contract
THESIS: cada grupo es una línea con su color, y una cuenta se encuentra como se encuentra un andén. Rechaza el
arreglo por defecto de la categoría: barra lateral gris, un acento azul y candados.

OWN-WORLD: panel negro de señal para el armazón (riel de grupos y barra superior), fondo claro neutro para el
trabajo, y una familia fija de colores de línea (azul, naranja, verde, amarillo) que solo nombra grupos. El rojo se
reserva para peligro y el verde oscuro para confirmaciones. Tipografía de señalización (Overpass) con su monoespaciada
para datos. Distintivo redondo con la inicial del grupo, texto oscuro sobre el color.

STORY: la persona ve en qué línea está, encuentra la cuenta en la lista, copia sin revelar y lee en la misma fila
cuánto falta para que se borre.

FIRST VIEWPORT: barra superior negra con marca, búsqueda ancha, cuenta regresiva de bloqueo y "Bloquear". Riel
negro de 272 px con los grupos y su distintivo. Lista de cuentas del grupo con encabezado grande. Detalle a la derecha
con la ruta del grupo dibujada como paradas, el título a 44 px y los campos en filas; la acción principal es
"Copiar" la contraseña.

FORM: señalización de transporte, candidata 4 de la lista ordenada de la directora; seed:7c72a16acf96e848. Gesto
propio: grupo = línea (distintivo de color en riel, lista y ruta). Subidas heredadas: el estado se estampa en la fila
y se queda; la fortaleza se lee por trama y texto; la contraseña oculta es una barra de ancho fijo; densidad compacta
disponible.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Unresolved
- El dueño dio una autorización general ("autorizo todo") después de leer la recomendación; no nombró la dirección.
  Si prefiere otra, los cuatro bocetos restantes están en `docs/diseno/mocks/`.
- Cuántos colores de línea existen y qué pasa con el grupo número cinco en adelante.
- Marca: icono y logo (brand-designer), todavía provisional.
- Boceto de referencia: `docs/diseno/mocks/senaletica.html` (crítica al final, no contrato de píxeles: build code-led).
