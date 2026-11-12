# Validación — Spec 001 Bóveda (2026-11-11 y 12)

Puerta de la fase: qué prueba cubre cada requisito y con qué resultado. El detalle está en los informes enlazados;
este archivo es el índice y el veredicto.

## Veredicto
**Spec cumplida para la 0.1.0, con seis pendientes declarados** (abajo). No lleva firma de Windows ni revisión
externa.

## De dónde sale cada prueba
| Fuente | Qué cubre | Resultado |
|---|---|---|
| Pruebas del implementador (`rf_NN_*` en Rust, `it("RF-NN: …")` en la interfaz) | Los 95 requisitos, salvo los tres recortables no hechos | 487 pruebas Rust y 428 de interfaz en verde (tras la segunda ronda del dueño) (incluye las independientes de la fila siguiente) |
| Pruebas independientes desde la spec, sin ver la implementación (`spec_rf_NN_*`) — `validacion-nucleo.md` | Núcleo: bloqueo, guardado, secretos, entradas, grupos, búsqueda, abrir y crear, generador, ajustes | 83 pruebas; 1 defecto (RF-94), corregido |
| Prueba en la ventana real con la app instalada — `validacion-app.md` | Flujo principal, matriz de completitud, portapapeles real, bloqueo automático, errores visibles, rastro en disco, superficie de navegador | Hito cumplido. Cinco corridas: la primera halló 2 defectos de integración bloqueantes y 6 menores; la de release, el menú contextual y las teclas de navegador de WebView2 (D-9 a D-12); la última comprobó la capa nativa aislada y el valor mostrado no seleccionable. Todos corregidos y reverificados en la app de release |
| Revisión de código — `docs/revisiones/0.1-codigo.md` | Fidelidad a la spec, constitución regla por regla, preparación para producción | Apta con condiciones; 1 Alto (C-1), corregido |
| Revisión de seguridad — `docs/revisiones/0.1-seguridad.md` | Requisitos [B] del modelo de amenazas, frontera ventana–núcleo, memoria, archivo, configuración, dependencias | Apta con condiciones; sin Críticos ni Altos; S-1 y S-3 corregidos, S-2 al backlog. Reverificación del delta de la superficie de navegador: D-9 era Alto (corregido); R-1 (¿la capa nativa surte efecto en frío?) no se confirmó en la ventana real: surte efecto; R-4 (valor mostrado seleccionable) corregido y verificado; Bajos e Info al backlog |
| Revisión de diseño — `docs/revisiones/0.1-diseno.md` | Resultado contra el contrato de dirección | Ver ese archivo |

## Pase de interacción (2026-11-12): RF-96 a RF-104
Encargo y resultado en `docs/revisiones/0.1-ux-dueno.md`; revisión de seguridad del delta al final de
`docs/revisiones/0.1-seguridad.md`.

| Requisito | Prueba | En la app instalada |
|---|---|---|
| RF-96 nombre de la bóveda al crearla | `RF-96` ×3 (`CreateScreen.test.tsx`) | "Familia" en la app y en KeePassLib 2.62. Sin ejercitar: archivo con nombre distinto al de la bóveda; KeePassXC |
| RF-97 ubicación propuesta | `rf_97_*` (núcleo y comandos), `RF-97` ×5 | Creó `Documentos\AlmarioKS\Familia.kdbx` y la carpeta en el PC real |
| RF-98 no reemplaza si ya existe | `rf_98_*`, `RF-98` ×2 | Se niega; los dos archivos quedan idénticos (sha256) |
| RF-99 archivo llave junto a la bóveda, con contraseña obligatoria | `rf_99_*`, `RF-99` ×3 | Crea `Familia.keyx`; sin contraseña maestra se niega y no escribe nada |
| RF-100 entrada en un diálogo modal | `RF-100` (`EntryDialog.test.tsx`, `EntryPanel.test.tsx`) | Foco en Título; Esc repetido (10 ráfagas) no pierde el borrador |
| RF-101 "Generar" llena el campo | `RF-101` | Un clic en Generar y uno en Guardar, 14 de 14 |
| RF-102 "Todas las entradas" | `RF-102` ×5 | Lista y total coinciden (13) |
| RF-103 tema oscuro siempre | `rf_103_*` (`webview_config.rs`) | Las tres pantallas |
| RF-104 ventana mínima 1000 × 640 | `rf_104_*` (`webview_config.rs`) | No se intentó achicar la ventana a mano |

Sin revisión de código independiente de este delta. La ventana a 1280 × 800 solo se vio para "Crear bóveda" y el
diálogo (la pantalla del PC de pruebas no da para capturar la bóveda entera a ese tamaño).

## Segunda ronda del dueño (2026-11-12): RF-42, RF-43, RF-65, RF-68 y RF-105 a RF-113
Encargo y resultado en `docs/revisiones/0.1-ronda-2-dueno.md`; seguridad en `docs/revisiones/0.1-seguridad.md`
(tres secciones del 2026-11-12) y código en `docs/revisiones/0.1-codigo.md` (sección del 2026-11-12).

| Requisito | Pruebas que lo citan | En la app instalada |
|---|---|---|
| RF-42 lista en tabla | `RF-42` (`VaultScreen.test.tsx`) | Medidas de la tabla; sin barra horizontal a 1000 × 640 |
| RF-43 entrada en una ventana | `RF-43`, `RF-108` | Abre con doble clic, Enter y el botón de la fila; 500 caracteres mostrados completos |
| RF-65, RF-68 generador de 8 a 500 | `rf_65_*`, `rf_68_*`, `spec_*` (solo cotas), `RF-65`, `RF-68` | 500 de punta a punta, también leídos con KeePassLib; 7, 501 y vacío no generan; teclear la longitud no regenera |
| RF-105 la contraseña no llega a la lista | 14 (`rf_105_*`, `RF-105`) | 0 hallazgos de 13 contraseñas en el árbol de accesibilidad; máscara igual para 8, 18 y 40 caracteres |
| RF-106 gestos de doble clic | 11 | Ratón real: contraseña, usuario, sitio, título y fecha; camino de teclado |
| RF-107 fechas con hora | 3 | "Modificada" cambia al minuto al editar |
| RF-108 Editar reemplaza la ventana y vuelve | 8 | Esc devuelve el foco a la fila |
| RF-109 error propio sin aleatoriedad | 6 (`rf_109_*`, `RF-109`) | Sin ejercitar (no se puede tumbar la fuente del sistema) |
| RF-110 llave junto a la bóveda propuesta | 18 | Viene elegida; se desbloquea escribiendo solo la contraseña |
| RF-111 llave recordada | 31 (`rf_111_*`, `RF-111`) | Movida y elegida una vez, viene elegida; si falta, lo dice; `settings.json` guarda solo la ruta |
| RF-112 mensaje de fallo de desbloqueo | 4 | Texto exacto |
| RF-113 ruta de la llave antes de crear | 2 | Las dos rutas a la vista; sin barra a 1120 × 720 |

Calidad del generador (punto 4): uniformidad con 9 000 000 de caracteres a ±2 % para el alfabeto completo y sin
parecidos; subconjuntos de tipos con 40 000 por carácter a ±3 %; primera y última posición a ±15 %; las pruebas
rechazan un generador con +3 % en un carácter y otro que pone primero una minúscula. Límite conocido: solo miden
frecuencias marginales. En la app instalada: 200 generadas, todas distintas, de 20 y con los cuatro tipos; rastro
en disco en cero.

Sin ejercitar en la app instalada: "Usar uno mío", llave recordada en una unidad desconectada, RF-109, otras
escalas y lector de pantalla.

## Requisitos sin prueba automática
- **Recortables no hechos:** RF-37 (bloqueo inmediato al bloquear la sesión de Windows), RF-85 (ver el historial),
  RF-86 (gestionar adjuntos). Los adjuntos y el historial se conservan (RF-25); el bloqueo por tiempo real cubre la
  suspensión del equipo (RF-29).
- **Solo en la interfaz, por diseño:** RF-8, RF-9, RF-78.
- **Con parte manual declarada en la spec ("Cómo se prueba"):** RF-72 (Win+V), RF-77 (carpeta de WebView2 sobre la app
  instalada), RF-81 (abrir dos veces), RF-83 (redacción de los mensajes).

## Matriz de completitud (en la ventana real)
| | Crear | Ver | Editar | Mover | Borrar | Definitivo / vaciar |
|---|---|---|---|---|---|---|
| Entradas | pasa | pasa | pasa (ejemplo 8) | pasa | pasa (a la papelera) | pasa |
| Grupos | pasa | pasa | pasa (renombrar) | pasa | pasa ("2 entradas y 1 subgrupo") | pasa (vaciar papelera) |

## Requisitos [B] del modelo de amenazas (AMENAZAS §4, fase 0.1)
| Requisito | Estado | Evidencia |
|---|---|---|
| Argon2id ≥ 64 MiB calibrado; aviso con bóveda débil (F1) | Cumplido | `rf_02_*`, `rf_17_*`, `spec_rf_02_*`; revisión de seguridad |
| Guardado atómico con relectura; historial ordenado; 3.x no se sobrescribe (F3) | Cumplido | `rf_21_*` a `rf_27_*`, `rf_14_*`, `rf_15_*`, `spec_rf_2x_*`; ejemplo 6 en disco real |
| Bloqueo que no falla abierto, con guardado colgado o fallido (W3) | Cumplido | `rf_30_*`, `rf_33_*`, `spec_rf_30_*`; en la ventana: bloquea en ~300 ms |
| Portapapeles fuera de Win+V y la nube; limpieza a los N s y al bloquear (W4) | Cumplido, con una parte manual pendiente | `rf_71_*` a `rf_75_*`, `rf_90_*`; portapapeles real vacío a los 20,8 s; historial verificado por WinRT. Falta mirar Win+V a mano y la nube (sincronización apagada en el PC de pruebas) |
| Fixture XSS sin ejecución; URL solo `http(s)` vía núcleo (W1) | Cumplido | `RF-78`, `rf_49_*`; la exportación no existe en la 0.1 |
| Canario en disco: 0 apariciones, incluida la carpeta de WebView2 (F5, W2) | Cumplido, también con el instalador de release | Tres pasadas sobre la app de release con control positivo (`validacion-app.md`); `rf_77_*` fija la configuración |
| Topes del KDF y cabecera mal formada (F2), subidos a [B] por el laboratorio | Cumplido | `rf_16_*`, `rf_94_*`, `spec_rf_16_*`, `spec_rf_94_*` |

## Pendientes declarados
1. **KeePassXC 2.8:** sin probar en esta fase. KeePass 2.62 (KeePassLib) sí abre las bóvedas creadas por la app con
   todo su contenido. El harness no permitió ejecutar el `keepassxc-cli` portable descargado; lo corre el dueño:
   `keepassxc-cli ls <bóveda.kdbx>` y `keepassxc-cli show <bóveda.kdbx> <entrada>`.
2. **Win+V a mano** (RF-72) y **portapapeles en la nube**: comprobación manual del dueño.
3. **Actualización real 0.0.2 → 0.1.0** con la bóveda abierta (C-1 / S-1): solo se puede probar con el release
   publicado; está cubierta por pruebas de la app y de contrato, no ejecutada de punta a punta.
4. **Usuario estándar de Windows:** sigue pendiente desde el H0.
5. **Diseño:** la revisión final quedó en "corregir" con tres puntos abiertos (`docs/revisiones/0.1-diseno.md`).
6. **Sin ejercitar en la ventana real:** AltGr (el teclado del PC de pruebas es en-US), guardado fallido, cambio
   externo, bóvedas KDBX 3.x, suspensión del equipo, y que la app se niegue a arrancar si no logra cerrar la
   superficie de navegador.

## Tiempos medidos (PC del dueño, compilación de depuración, bóveda calibrada)
Crear una bóveda: 9,0 s (incluye calibrar el KDF) · desbloquear: 1,2 s · guardar una entrada hasta el disco: 2,45 s ·
bloquear: ~0,3 s · buscar en 5 000 entradas: 13,6 ms en release. Meta de la spec para desbloquear: ≤ 1,5 s (cumple).
El guardado lento está en `docs/BACKLOG.md` como lo primero a mejorar.
