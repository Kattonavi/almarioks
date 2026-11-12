# AlmarioKS — referencias visuales (2026-11-11)

> Insumo de `design-references` para `/impeccable shape`. **No es una decisión:** la dirección la decide Impeccable
> y la aprueba el dueño sobre un mock. Contrastes calculados con la fórmula WCAG 2.x. Lo marcado "(sin verificar)"
> viene de conocimiento previo y se confirma antes de empaquetar (licencias de fuentes e iconos incluidas).

Orden de paleta: fondo · superficie · texto · texto secundario · acento · peligro · éxito. Todos los pares
texto/fondo superan AA (4,5:1).

## A. "Cuaderno": papel cálido, calma
Una libreta de bolsillo donde guardas lo importante; cálida, legible, sin aspecto de herramienta de seguridad.
Se parece a Bear (https://bear.app) y Things 3 (https://culturedcode.com/things/) (sin verificar capturas actuales).
Se toma: neutros cálidos, mucho aire, un solo acento sobrio, lista con una línea por fila.

| Rol | Claro | Oscuro |
|---|---|---|
| Fondo | #F6F1E7 | #1C1A17 |
| Superficie | #FFFDF8 | #26231F |
| Texto | #2B2620 (13,3 sobre fondo; 14,7 sobre superficie) | #EDE6D8 (14,0 / 12,6) |
| Secundario | #6B6256 (5,9 sobre superficie) | #A89F90 (6,0) |
| Acento (verde tinta) | #1F6B52 (6,3 sobre superficie) | #6FC3A0 (7,4) |
| Peligro | #B3261E (6,4) | #F08A80 (6,4) |
| Éxito | #2E7D32 (5,0) | #8BCB8F (8,2) |

Tipografía: Atkinson Hyperlegible Next, OFL-1.1 (https://github.com/googlefonts/atkinson-hyperlegible-next,
verificada) + JetBrains Mono, OFL-1.1 (https://github.com/JetBrains/JetBrainsMono) (sin verificar). Atkinson se
diseñó para distinguir caracteres parecidos.
Densidad cómoda (filas de 44 px) · radio 8 px · iconos Phosphor, MIT (https://github.com/phosphor-icons/core) (sin verificar).
Riesgos: (1) el beige puede leerse como "app de notas" y restar sensación de seguridad; (2) con cientos de entradas la
comodidad cuesta desplazamiento: haría falta una opción compacta.

## B. "Taller": nativo de Windows, preciso y compacto
Herramienta sobria que parece parte de Windows 11, con rejilla estricta y cero adorno.
Se parece a Fluent 2 (https://fluent2.microsoft.design) y Zed (https://zed.dev) (sin verificar).
Se toma: grises fríos, selección por franja de acento, barras de comandos, foco muy visible.

| Rol | Claro | Oscuro |
|---|---|---|
| Fondo | #EEF0F2 | #14171A |
| Superficie | #FFFFFF | #1D2126 |
| Texto | #16191D (15,4 / 17,6) | #E6E9EC (14,8 / 13,3) |
| Secundario | #566069 (6,4) | #9AA4AE (6,4) |
| Acento (azul acero) | #0F5FA8 (6,5) | #6CB2F2 (7,2) |
| Peligro | #C62828 (5,6) | #FF8A80 (7,1) |
| Éxito | #1B7F4B (5,0) | #5FCB8E (8,0) |

Tipografía: IBM Plex Sans + IBM Plex Mono, OFL-1.1 (https://github.com/IBM/plex) (sin verificar). Segoe UI no es
libre: no se empaqueta.
Densidad compacta (filas de 32 px) con opción cómoda · radio 4 px · iconos Lucide, ISC (https://lucide.dev/license) (sin verificar).
Riesgos: (1) es la más cercana a KeePassXC en sensación; sin disciplina de "pocas cosas" cae en lo que la visión
quiere evitar; (2) el azul es el color más genérico de la categoría y apenas diferencia la marca.

## C. "Mate": oscuro primero, objeto de metal con cobre
Una caja fuerte mate, con un acento cobre que marca solo lo accionable; transmite robustez.
Se parece a 1Password 8 (https://1password.com) y Linear (https://linear.app) en modo oscuro (sin verificar).
Se toma: superficies apiladas por luminosidad, título de entrada grande, la contraseña como pieza central del detalle.

| Rol | Claro | Oscuro |
|---|---|---|
| Fondo | #F4F2F0 | #151413 |
| Superficie | #FFFFFF | #1F1D1B |
| Texto | #1E1B19 (15,3 / 17,1) | #F0EBE6 (15,5 / 14,2) |
| Secundario | #625B55 (6,7) | #A39A92 (6,1) |
| Acento (cobre) | #A8481F (5,8) | #E0794B (5,6) |
| Peligro | #B42318 (6,6) | #FF8F85 (7,6) |
| Éxito | #1F7A3D (5,4) | #6CCB85 (8,4) |

Tipografía: Figtree, OFL-1.1 (https://github.com/erikdkennedy/figtree) + Geist Mono, OFL-1.1
(https://github.com/vercel/geist-font) (ambas sin verificar).
Densidad cómoda · radio 12 px en tarjetas y 8 px en controles · iconos Tabler, MIT (https://github.com/tabler/tabler-icons) (sin verificar).
Riesgos: (1) el modo claro se debilita si el diseño es oscuro primero; (2) cobre y rojo de peligro están cerca en
tono: exigir icono y texto, nunca solo color, y no usar el acento para errores.

## Patrones en gestores reales
| Aspecto | KeePassXC | Bitwarden | 1Password | Proton Pass |
|---|---|---|---|---|
| Contraseña oculta | Puntos con botón de mostrar (sin verificar) | Campo oculto con ojo (sin verificar) | Puntos; revela con botón (sin verificar) | Puntos con ojo (sin verificar) |
| Copiar | Barra "Copy Password"; doble clic solo si se activa en ajustes (https://keepassxc.org/docs/KeePassXC_UserGuide) | Icono junto al campo; acciones rápidas opcionales (https://bitwarden.com/help/managing-items/) | Botón junto al campo (sin verificar) | Botón junto al campo (sin verificar) |
| Medidor de fortaleza | Barra de entropía en generador y edición (sin verificar) | Indicador en generador (sin verificar) | Barra con etiqueta junto a la contraseña (sin verificar) | Indicador en generador (sin verificar) |
| Bloqueada | Ventana de desbloqueo; "Lock Database" en la barra (guía de usuario) | Pantalla con contraseña maestra (sin verificar) | Pantalla con campo y logo (sin verificar) | Pantalla de desbloqueo (sin verificar) |
| Vacíos | Panel vacío sin guía (sin verificar) | Mensaje con botón (sin verificar) | Texto y acción (sin verificar) | Ilustración y acción (sin verificar) |

Lectura: copiar sin ver la contraseña es universal y `docs/UX.md` ya lo exige. El hueco que los demás no cubren bien
es el estado vacío con explicación y el aviso de limpieza del portapapeles.

## Lo que no se recomienda
- Gradiente azul-violeta, hero centrado con tres tarjetas e Inter o Roboto por defecto: es el aspecto genérico que la
  visión descarta.
- Contraseña siempre visible, ilustraciones de candados o escudos, emoji como iconos.
- Fuentes de Windows o macOS empaquetadas (Segoe UI, SF): no son libres para distribuir con GPL-3.0.

## Pendiente antes de elegir
Capturas reales de los competidores, prueba de legibilidad de `0 O 1 l I` en las dos monoespaciadas y verificación
de las licencias marcadas "(sin verificar)".
