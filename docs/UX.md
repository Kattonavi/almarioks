# UX mínimo — AlmarioKS

> Base para las specs y la QA. La dirección visual la decide Impeccable (`/impeccable shape`) y el usuario aprueba
> el mock **antes** de construir la interfaz de la 0.1. Los flujos del navegador se detallan en la spec 002.

## Necesidades
1. Necesito crear mi bóveda en un minuto y entender qué pasa si pierdo la contraseña o el archivo llave, para no
   quedarme por fuera de mis propias cuentas.
2. Necesito encontrar una entrada y copiar su contraseña sin verla en pantalla, para usarla con gente al lado.
3. Necesito que la app se bloquee sola cuando me voy, para no dejar todo abierto.
4. Necesito crear una cuenta en un sitio sin inventar ni copiar la contraseña a mano, para no reutilizar la misma.
5. Necesito saber que mi archivo abre en KeePass si esta app desaparece, para confiarle mis contraseñas.

## Flujo principal de la 0.1
Abrir la app → (primera vez) crear bóveda: nombre y lugar del archivo, contraseña maestra con medidor, archivo llave
opcional con aviso → bóveda abierta: grupos a la izquierda, entradas al centro, detalle a la derecha → nueva entrada
con contraseña generada → guardar → buscar → copiar → se bloquea sola → desbloquear.
Tarea principal (buscar y copiar una contraseña): 3 pasos desde la bóveda abierta (atajo de búsqueda, escribir, copiar).

## Mapa de pantallas (0.1)
| # | Pantalla | Para qué |
|---|---|---|
| P1 | Bienvenida | Crear bóveda nueva o abrir una existente; recuerda la última |
| P2 | Crear bóveda | Archivo, contraseña maestra, archivo llave opcional, aviso de recuperación |
| P3 | Desbloquear | Contraseña, archivo llave o ambos; también es la pantalla de bloqueo |
| P4 | Bóveda | Árbol de grupos + lista de entradas + búsqueda |
| P5 | Detalle y edición de entrada | Ver, mostrar u ocultar, copiar, editar, mover, borrar |
| P6 | Generador | Longitud, tipos de caracteres, fortaleza; usable dentro de P5 |
| P7 | Grupo | Crear, renombrar, mover, borrar (diálogo) |
| P8 | Ajustes y Acerca de | Tiempo de bloqueo, limpieza del portapapeles, actualización, versión y licencia |
| P9 | Actualización | "Hay una versión nueva": ver cambios, aceptar o dejar para después (existe desde el H0) |

## Matriz pantalla × estado
"Sin permiso" en una app local = **bóveda bloqueada**.

| Pantalla | Vacío | Carga | Error | Éxito | Bloqueada |
|---|---|---|---|---|---|
| P1 Bienvenida | Sin bóvedas recientes: dos acciones claras | — | El archivo reciente ya no existe: ofrecer buscarlo o quitarlo | Va a P2 o P3 | — |
| P2 Crear | Campos vacíos con ayuda | Derivando la llave (puede tardar 1 s o más): progreso | Contraseñas no coinciden · no se puede escribir en esa carpeta · contraseña débil (avisa, no bloquea) | Bóveda creada; recuerda guardar copia del archivo llave | — |
| P3 Desbloquear | — | Derivando la llave: progreso, sin congelar la ventana | Llave incorrecta (sin decir cuál parte falló) · archivo dañado · archivo llave no encontrado · formato no soportado | Va a P4 | Es su estado normal |
| P4 Bóveda | Bóveda sin entradas: explica y ofrece "Nueva entrada" · búsqueda sin resultados: lo dice y ofrece limpiar | Guardando: indicador discreto | No se pudo guardar: los cambios siguen en memoria, reintentar o guardar en otro lugar · el archivo cambió en disco por otra app | Guardado | Oculta todo y muestra P3 |
| P5 Entrada | Entrada nueva | Guardando | Título vacío · no se pudo guardar | Guardada · copiado (se limpia en N s) | Cierra y descarta lo mostrado |
| P6 Generador | Valores por defecto sensatos | — | Reglas imposibles (0 tipos de caracteres) | Contraseña generada y aplicada | — |
| P7 Grupo | — | — | Nombre vacío · mover a sí mismo o a un hijo · borrar grupo con entradas (confirma y dice cuántas) | Hecho | — |
| P8 Ajustes | — | Buscando actualización | Sin conexión (no es un error grave) | Guardado | Visible sin datos de la bóveda |
| P9 Actualización | No hay versión nueva | Descargando con progreso y cancelar | Firma inválida: no instala y lo explica · descarga fallida: reintentar | Instalada, reabre | Disponible |

## Reglas de comportamiento que ya están decididas
- Las contraseñas se muestran ocultas; "mostrar" es una acción explícita por entrada.
- Copiar una contraseña avisa cuándo se limpia el portapapeles y lo limpia aunque la app se bloquee.
- Toda acción destructiva confirma y dice qué se pierde. Todo arrastre tiene alternativa por menú y teclado.
- Los errores dicen qué pasó y qué hacer; sin códigos técnicos ni "inválido".
- La app nunca se queda congelada mientras deriva la llave o guarda.

## Pendiente para la spec 002 (navegador)
Emparejar la extensión · rellenar en inicio de sesión · ofrecer contraseña junto al campo · "¿guardar?" y
"¿actualizar?" dentro de la página · app cerrada o bloqueada · sitio sin entrada · varias entradas para el mismo sitio.

## Para la 1.0
Evaluación heurística de Nielsen (tabla heurística · pantalla · severidad · evidencia) y, si hay personas a mano,
prueba de tareas con 5.
