# Product

<!-- impeccable:product-schema 1 -->

> Escrito el 2026-11-11 sin entrevista en vivo: el dueño no estaba. Lo que dice **(confirmado)** sale de la visión y
> del plan que él aprobó ese día (`docs/PROYECTO.md`). Lo que dice **(inferido)** lo dedujo la directora de esos
> documentos y espera su confirmación.

## Platform

web

<!-- La interfaz corre en una webview (Tauri 2, WebView2) dentro de una app de escritorio para Windows. Su lenguaje
de diseño es web, no nativo de Windows. (inferido de la arquitectura confirmada) -->

## Users
- **Dueño de una bóveda** (confirmado): persona que quiere sus contraseñas en un archivo propio, sin cuenta ni nube,
  con la comodidad que hoy solo dan el navegador o los gestores en línea. No es necesariamente técnica.
- **Situación** (inferido): en su PC con Windows, muchas veces con gente al lado; entra a la app varias veces al día
  por pocos segundos para buscar una cuenta y copiar su contraseña, y de vez en cuando para guardar una nueva.

## Product Purpose
Gestor de contraseñas local y abierto (confirmado). La bóveda es un archivo KDBX 4 del usuario, protegido con
contraseña maestra, archivo llave o ambos. Éxito de la fase actual (0.1): crear una bóveda, abrirla, guardar y
encontrar cuentas, copiar una contraseña sin verla, y que la app se bloquee sola. Éxito de la 1.0: además, la
extensión del navegador ofrece una contraseña al crear una cuenta y pregunta si la guarda al enviar el formulario.

## Positioning
Compatible con KeePass **y** con la experiencia de navegador de un gestor moderno, en una app liviana, en español
desde el primer día y 100 % abierta (GPL-3.0) (confirmado). El mecanismo que un vecino no puede copiar con verdad:
**el archivo es del usuario y lo abren otras apps**; si AlmarioKS desaparece, las contraseñas siguen siendo suyas.

## Operating Context
- Tarea principal (confirmado, `docs/UX.md`): buscar y copiar una contraseña en 3 pasos desde la bóveda abierta.
- Ventana de escritorio de tres zonas: grupos, entradas, detalle. Pantallas P1 a P9 de `docs/UX.md`.
- Se bloquea sola por inactividad; lo copiado se borra del portapapeles a los pocos segundos.
- El mismo archivo puede abrirse en KeePass 2 o KeePassXC cuando AlmarioKS no lo tiene abierto.

## Capabilities and Constraints
- Sin red (salvo la actualización que el usuario pide), sin telemetría, sin cuenta (confirmado).
- Las contraseñas no viajan a la interfaz salvo cuando el usuario pide "mostrar"; copiar sale del núcleo (confirmado).
- CSP estricta: `style-src 'self'`, `script-src 'self'`; sin estilos ni scripts en línea, sin recursos remotos:
  fuentes e iconos van empaquetados y con licencia libre (OFL, Apache, MIT o ISC) por ser GPL-3.0.
- Windows 10 y 11, ventana mínima de 480 × 360. Español; inglés en la 1.0.
- No es un clon de KeePassXC: sin Auto-Type, plugins ni agente SSH. Sin TOTP ni llaves de acceso antes de la 1.0.
- **Sin decidir:** qué significa "KS" y cómo se presenta el nombre en público.

## Brand Commitments
- Nombre: **AlmarioKS** (confirmado). No hay logo ni identidad: el icono actual es provisional.
- Voz (confirmado, `docs/UX.md`): los errores dicen qué pasó y qué hacer, sin códigos técnicos ni la palabra
  "inválido". Toda acción destructiva dice qué se pierde.
- Lo que el dueño pidió evitar (confirmado, visión): la cantidad de opciones y pantallas de configuración de
  KeePassXC; la nube y la cuenta; lo cerrado.

## Evidence on Hand
- Bóvedas de prueba con datos falsos en `crates/almarioks-core/tests/fixtures/`.
- Referencias visuales reunidas, sin decisión: `docs/diseno/referencias.md`.
- No hay usuarios, testimonios, cifras de uso ni auditorías externas: nada de eso se puede afirmar.
- Las versiones 0.x son de desarrollo y no llevan firma de Windows: la interfaz no puede prometer "seguro" ni
  "auditado".

## Product Principles
1. **La bóveda es del usuario.** Archivo estándar que otras apps abren sin perder nada.
2. **Cero criptografía propia.** Lo que protege sale de librerías probadas.
3. **El navegador pide, la app decide.** Rellenar y guardar exigen un gesto del usuario.
4. **Entrega antes que funciones.**
5. **Pocas cosas, terminadas.**

## Accessibility & Inclusion
Todo operable con teclado, nombre accesible en cada control, foco visible y contraste AA desde la 0.1. Lector de
pantalla, escalado al 150 % y movimiento reducido se revisan completos en la 1.0 (confirmado, `docs/CHECKLIST.md`).
