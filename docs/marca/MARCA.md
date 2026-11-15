# AlmarioKS — marca: icono, logotipo y voz

> Propuesta del 2026-11-11 dentro de la dirección "Señalética"; si esa dirección cambia, la marca se revisa. Lo
> marcado **(por decidir)** no está confirmado. El nombre de los logotipos está convertido a trazado desde Overpass
> ExtraBold (SIL OFL 1.1): no necesitan la fuente. Iconos de la app: `pnpm tauri icon docs/marca/icono-1024.png`.

## La idea del símbolo
Tres líneas de color llegan a una misma estación de transbordo: cada grupo es una línea y la bóveda es el punto
donde todas se encuentran. Es el gesto del contrato ("grupo = línea con su color") dibujado como pictograma de mapa
de transporte, sin letra y sin ningún objeto de la categoría (candado, escudo, llave).

## Por qué el concepto 1 y no los otros dos (`conceptos.png`)
- **1 · Transbordo (recomendado).** Usa los colores como lo que son en el contrato, líneas, sin apropiarse de uno.
  Está dibujado sobre una rejilla de 16 × 16, así que a 16 px cada borde cae en un píxel entero. No depende del
  idioma. **Riesgo:** es abstracto; solo no dice "contraseñas" ni "AlmarioKS" y puede leerse como un icono de alinear
  o de deslizadores. A 24 px la rejilla no cae en píxeles enteros y los bordes se suavizan (se lee, pero menos nítido).
- **2 · Placa de estación** (inicial blanca y tres distintivos). Se reconoce como letra, pero a 16 px los distintivos
  son manchas de 3 px y la inicial baja a 8 px; "letra blanca en cuadro negro" es un recurso muy usado.
- **3 · Distintivo** (disco amarillo con la inicial). El más legible, pero toma el amarillo, que el contrato reserva
  para nombrar grupos: se confundiría con el distintivo de un grupo y se parece a los de líneas de metro reales.

## Colores (los del contrato y su boceto; no hay otros)
| Color | Hex | Rol en la marca |
|---|---|---|
| Negro de señal | `#16181D` | Placa del icono; nombre sobre fondos claros |
| Fondo claro | `#F6F6F3` | Fondo de trabajo sobre el que se prueba la marca |
| Blanco | `#FFFFFF` | La estación; nombre sobre fondos oscuros |
| Línea azul | `#6AA5FF` | Línea superior del símbolo |
| Línea naranja | `#F26A1B` | Línea central del símbolo |
| Línea amarilla | `#F2B705` | Línea inferior del símbolo |
| Línea verde | `#1E9E6A` | Color de línea de la interfaz; no aparece en el símbolo |
| Rojo | `#D7263D` | Reservado a peligro; nunca en la marca |

## Tamaños mínimos y área de respeto
- Símbolo: mínimo 16 px de lado. Logotipo horizontal: mínimo 24 px de alto (unos 117 px de ancho).
- Área de respeto: el ancho de la estación (un cuarto del lado del símbolo) libre por los cuatro lados.
- Sobre fondos claros, siempre con placa. Sin placa solo sobre el negro de señal u otros fondos oscuros: sobre claro
  la estación blanca desaparece y el amarillo y el azul no contrastan.

## Usos incorrectos
1. Quitar la placa sobre un fondo claro, o poner el símbolo sin placa sobre una foto o un color medio.
2. Cambiar los colores de las líneas o su orden, usar el rojo o pasar el símbolo a degradados, sombras o contornos.
3. Acompañarlo de candados, escudos, llaves o de palabras como "seguro" o "auditado".
4. Deformarlo, girarlo, recomponer el nombre con otra fuente o separar y colorear "KS".

## Voz (sale de `PRODUCT.md`)
1. Un error dice qué pasó y qué hacer ahora: "No se pudo guardar: el archivo está abierto en otra app. Ciérralo
   allí y vuelve a intentarlo." Nunca códigos técnicos ni la palabra "inválido".
2. Toda acción destructiva dice qué se pierde antes de pedir la confirmación.
3. No se promete "seguro", "auditado" ni "protegido": se describe lo que la app hace (se bloquea, borra lo copiado).
4. Lo propio se dice sin superlativos: el archivo es tuyo, lo abren otras apps, no hay nube ni cuenta.
5. Palabras de quien la usa (cuenta, grupo, bóveda, copiar), frases cortas y en español desde el primer día.

## Por decidir por el dueño
- Aprobar el concepto 1 o escoger otro: los tres quedan en esta carpeta y cambiar es regenerar los iconos.
- Qué significa "KS" y cómo se presenta el nombre. El logotipo lo escribe en una sola tinta; el boceto pintaba "KS"
  en amarillo, lo que choca con "los colores de línea solo nombran grupos". Lo resuelve Impeccable con el dueño.
- Si el símbolo puede fijar azul, naranja y amarillo cuando aún no se sabe cuántos colores de línea habrá.
- Pendiente: versión a una tinta, icono de 24 px afinado a mano y búsqueda de parecidos o registro de marca (solo
  se comparó con los iconos de KeePass, KeePassXC, Bitwarden y 1Password: candado, llave, escudo y cerradura).
