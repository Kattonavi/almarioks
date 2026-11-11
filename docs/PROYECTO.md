# AlmarioKS

> Generado por /nuevo-proyecto el 2026-11-11 desde la visión aprobada por el autor ese día.
> Lo marcado **(pendiente)** está por confirmar. **Plan aprobado por el usuario el 2026-11-11.**

## Resumen
- **Cliente / negocio:** proyecto propio de Kattonavi, de código abierto (GPL-3.0).
- **Qué hace y para quién:** gestor de contraseñas local. La bóveda es un archivo KDBX 4 del usuario, protegido con
  contraseña maestra, archivo llave o ambos. Una extensión de navegador ofrece una contraseña al crear una cuenta y
  pregunta si la guarda al enviar el formulario. Para quien quiere sus contraseñas en un archivo propio, sin cuenta
  ni nube, con la comodidad de un gestor en línea. Nada sale del equipo.
- **Ciudad / país / idioma:** Colombia · español desde el día 1, inglés en la 1.0.
- **Objetivo medible:** 1.0 publicada (release firmado + extensión en la tienda) en la que (a) el ejemplo de GitHub
  de la visión funciona de punta a punta, (b) 16 de 20 sitios de la lista fija se detectan y guardan bien, y (c) una
  bóveda creada aquí se edita en KeePass 2 y KeePassXC y vuelve sin pérdida.
- **Dominio:** escritorio (Tauri 2 + Rust) con extensión de navegador. Piloto de Tauri del Workspace.
- **Nivel:** E3 (≈ N4) + anexo Rust (bloques R y T) · **Talla:** L, partida en 4 apuestas de menos de una semana ·
  **Riesgo:** alto (guarda secretos).

## Apetito y circuit breaker
Fijado por el usuario el 2026-11-11: **64 h, fase por fase**. Al cerrar cada fase se le muestra publicada y decide si
sigue la siguiente. El tiempo no se alarga: se recorta alcance en el orden de "Qué se recorta primero".

| Fase | Apetito | Se para y recorta a las | Entrega | Hecho cuando |
|---|---|---|---|---|
| **H0** · entregar | 8 h | 4 h sin un instalador que instale y abra | Instalador desde un GitHub Release, actualización N → N+1, prueba de KDBX en el núcleo, ADR, `AMENAZAS.md` | Los 5 criterios de "Rebanada 0" |
| **0.1 · Bóveda** | 16 h | 8 h sin crear → abrir → guardar una entrada en la ventana real | Crear y abrir bóveda; CRUD de entradas y grupos; búsqueda; generador; copiar con limpieza; bloqueo | Matriz de completitud en verde; ida y vuelta con KeePass 2 y KeePassXC sin pérdida |
| **0.2 · Navegador** | 24 h | 12 h sin que el ejemplo de GitHub guarde una entrada | Extensión + anfitrión + emparejamiento; rellenar; ofrecer contraseña; "¿guardar?" y "¿actualizar?" | Ejemplo de GitHub; 16 de 20 sitios; nunca rellena en otro origen; bloqueada pide desbloquear |
| **1.0 · Publicable** | 16 h | 8 h sin release candidate completo | Revisión de seguridad por lote; firma; SBOM; `SECURITY.md`; importar CSV; inglés; accesibilidad; tienda | Checklist E3 con evidencia; release firmado; extensión publicada |

Al 100 % de una fase: se publica lo que hay o se renegocia con el usuario. Nunca se sigue "un poco más".

## Alcance
| Must (H0 + 0.1) | Should (0.2) | 1.0 | Después (si el usuario lo pide) |
|---|---|---|---|
| Instalador por usuario que instala, abre, actualiza con permiso y desinstala | Extensión MV3 (Chrome, Edge, Brave) + anfitrión nativo + emparejamiento | Firma del instalador (SignPath) | Firefox |
| Crear bóveda KDBX 4 con contraseña maestra, archivo llave o ambos | Rellenar al iniciar sesión, con gesto del usuario y solo en el origen exacto | SBOM, `SECURITY.md`, atestación, `THIRD-PARTY-NOTICES` | Linux |
| Abrir, bloquear (manual y automático) y cerrar | Ofrecer contraseña generada al crear una cuenta | Importar CSV de navegadores y de Bitwarden | TOTP |
| Entradas: crear, ver, editar, mover, borrar | "¿Guardar?" al enviar el formulario | Español e inglés | Llaves de acceso |
| Grupos: crear, ver, renombrar, mover, borrar | "¿Actualizar contraseña?" | Accesibilidad completa (lector de pantalla, DPI) | Android |
| Búsqueda | Botón manual de guardar (siempre) | Extensión en la tienda | |
| Generador de contraseñas con medidor de fortaleza | | Prueba de intrusión ligera (E3-9) | |
| Copiar con limpieza del portapapeles | | | |
| Guardado atómico con copia de respaldo | | | |

**Regla de la 0.1:** lo que la app no edita (adjuntos, historial, campos propios, iconos, datos de plugins) **lo
conserva**. El lab 19 demostró que la librería lo permite.

**Qué se recorta primero** (de arriba hacia abajo, dentro de cada fase):
- 0.1: ver el historial → editar adjuntos → papelera (se borra con confirmación) → mover grupos arrastrando (queda por menú).
- 0.2: "¿actualizar contraseña?" → inicios de sesión en varios pasos e iframes → Brave y Edge (primero Chrome).
- 1.0: importar de Bitwarden (queda el CSV de navegadores) → inglés.

- **No-goals (3):** (1) no sincroniza ni tiene servidor ni cuenta; (2) no es un clon de KeePassXC: sin Auto-Type,
  plugins ni agente SSH; (3) sin TOTP, llaves de acceso, móvil, Firefox ni Linux antes de la 1.0. Además: no importa
  desde Almario y no inventa formato ni cifrado.
- **Riesgos top 3:** (1) primer proyecto con Tauri y el PC aún no compila Rust: el H0 puede comerse su tiempo en
  entorno; (2) detectar "estoy creando una cuenta" es una heurística: meta 16 de 20 con botón manual siempre;
  (3) SignPath puede tardar o negar la firma, y solo se puede pedir con un release ya publicado: las 0.x salen sin
  firma Authenticode (sí con la firma del actualizador, `SHA256SUMS` y atestación) y Windows advierte.
  Riesgo de fondo: repetir a Almario (pulido sin publicar), que es lo que este plan ataca.
- **Contenido:** textos de interfaz los escribe Sara con Impeccable · **Marca:** no hay. Nombre AlmarioKS decidido;
  icono y logo los hace `brand-designer` antes de la dirección visual de la 0.1 (el H0 usa un icono provisional).
  Significado de "KS": **(pendiente)**, no bloquea.

## Rebanada 0 = H0 (sin spec; 5 criterios de aceptación)
1. `cargo nextest run -p almarioks-core` en verde con la ida y vuelta de KDBX (abrir y guardar sin tocar) contra
   bóvedas hechas por KeePass 2 **y KeePassXC** (el experimento solo cubrió KeePass 2). Es una prueba de la librería,
   no una función: las envolturas (orden del historial, KDF, archivo llave) son de la 0.1.
2. La etiqueta `v0.0.1` dispara `release.yml` y publica en un GitHub Release el instalador, `latest.json` y
   `SHA256SUMS`, sin pasos manuales.
3. En una máquina limpia el instalador instala por usuario sin pedir administrador, abre la ventana real y muestra
   la versión. Windows 11 Home no trae Windows Sandbox: se prueba en un job de GitHub Actions y con un segundo
   usuario local de Windows.
4. Con la 0.0.1 instalada, la app detecta la 0.0.2, pregunta, descarga, verifica la firma, instala y reabre como
   0.0.2. Un paquete alterado se rechaza (prueba negativa, E2-2).
5. Desinstalar no deja archivos del programa y no toca las bóvedas del usuario.

6. Los seis requisitos **[B]** del H0 en `docs/AMENAZAS.md` §4: llave del actualizador con contraseña, solo en el
   entorno `release` de GitHub y con respaldo en 2 lugares (simulacro de firma desde el respaldo); `.sig` alterado, otra
   llave y `http://` rechazados; CSP sin `unsafe-*`, sin devtools y capacidades explícitas; `release.yml` solo por
   etiqueta protegida con aprobación; `cargo deny check` y `pnpm audit --prod` en verde; `SHA256SUMS` y atestación
   (`gh attestation verify`).

Además en el H0: README con el aviso de SmartScreen y la solicitud a SignPath **preparada** ("Code signing policy",
roles, privacidad, 2FA). SignPath exige un release ya publicado: se envía después del primer release (ADR-0004).
El anfitrión nativo y sus claves de registro **no** entran al H0: llegan el primer día de la 0.2 (ADR-0005).

**El H0 creció al cruzarlo con el modelo de amenazas** (criterio 6) y sigue en 8 h. Si no cabe, se recorta en este
orden: (1) bóvedas de KeePassXC en el corpus → inicio de la 0.1; (2) atestación → 0.1; (3) prueba con segundo
usuario local (queda el job de Actions). **No se recorta** porque es de una vía o es el H0 mismo: identificador,
custodia y respaldo de la llave del actualizador, prueba del `.sig` alterado, e instalar → abrir → actualizar →
desinstalar.

## Specs
| # | Feature | Spec | Estado |
|---|---|---|---|
| 001 | Bóveda (0.1) | `docs/specs/001-boveda/spec.md` | se escribe al empezar la 0.1 |
| 002 | Navegador (0.2) | `docs/specs/002-navegador/spec.md` | se escribe al empezar la 0.2 |
| 003 | Importar CSV e idiomas (1.0) | `docs/specs/003-importar-idiomas/spec.md` | se escribe al empezar la 1.0 |

## Avance (se actualiza al cerrar cada rebanada)
| Rebanada | Hecho | Falta | Horas usadas / apetito |
|---|---|---|---|
| Antes del plan | Contexto, lab 19 (KDBX sin pérdida), base de documentos, 5 ADR, arquitectura y modelo de amenazas | C++ Build Tools para empezar el H0 | ~0,5 h de reloj (09:15 a 09:45) más ~23 min de dos agentes en paralelo; cuenta dentro del H0 |

## Stack (según estandares/decisiones-stack.md → "Rust")
| Capa | Elección | Por qué / desviación |
|---|---|---|
| Interfaz | React + TypeScript strict + Vite + Tailwind + shadcn/ui | Dentro de Tauri no hay servidor de Next |
| App | Tauri 2.12.x + `tauri-plugin-updater` 2.x | La 3.0 está en alfa. `cargo info tauri` ya devuelve la alfa: las versiones se fijan a mano |
| Núcleo | crate `almarioks-core` sin Tauri: `keepass` =0.17.0 (`save_kdbx4`), `secrecy`, `zeroize`, `zxcvbn` | R-3. Lab 19 |
| Navegador | Extensión Manifest V3 en TypeScript + anfitrión de mensajería nativa en Rust | Visión |
| BD | Ninguna: el archivo KDBX es la base | Pilar 1 |
| Pruebas | cargo-nextest + proptest · Vitest · Playwright para la extensión | Estándar |
| CI y release | GitHub Actions (`ci.yml`, `release.yml`), `cargo deny`, pnpm | **Desviación:** no va a Railway; es escritorio y se entrega por GitHub Releases |

Decisiones de una vía: `docs/adr/` (máx. 5) · arquitectura: `docs/ARQUITECTURA.md` · amenazas: `docs/AMENAZAS.md`.

## Atributos de calidad que mandan
| Atributo | Meta medible |
|---|---|
| Compatibilidad KDBX sin pérdida | 0 diferencias en la ida y vuelta contra el corpus de KeePass 2 y KeePassXC, en cada CI |
| Los secretos no viven en la interfaz | 0 apariciones del canario en las respuestas de los comandos (salvo `reveal_field`) y en disco, incluida la carpeta de WebView2. El volcado de memoria de WebView2 se **mide y se anota** en la 1.0 (la maestra se teclea ahí): no se promete cero |
| Entrega continua | Cada fase termina con un release instalable y actualizable desde el anterior |

## Infra y entrega
- **Dónde se publica:** GitHub Releases del repositorio público `Kattonavi/almarioks` (instalador + actualizaciones).
  Extensión: Chrome Web Store en la 1.0.
- **Identidad (de una vía):** editor "Kattonavi" · identificador `io.github.kattonavi.almarioks`.
- **Autoría de git en este repo:** correo `noreply` de GitHub (configurado en local), porque el repo es público.

## Legal
- No recolecta datos personales ni telemetría. `docs/PRIVACIDAD.md` lo declara en la 1.0 (la tienda de extensiones lo pide).
- GPL-3.0: cada versión distribuida lleva su código. `THIRD-PARTY-NOTICES` en la 1.0 (E3-10).

## Mapa de lo que no domino (Fase 1b)
| Tema | Estado | Cuándo y cómo se aprende |
|---|---|---|
| Escribir KDBX 4 desde Rust con `keepass` 0.17 | **Aprendido** | `docs/lab/kdbx-roundtrip`: 4 de 4 bóvedas idénticas en KeePassLib 2.62 |
| Compatibilidad con KeePassXC | Pendiente | H0: instalar KeePassXC y sumarlo al corpus |
| Tauri 2 en un proyecto real (capacidades, CSP, comandos) | Pendiente | Inicio del H0: `rust-engineer` con fase de aprendizaje en la documentación oficial |
| Actualizador N → N+1 con GitHub Releases | Pendiente | Es el H0 mismo; ADR-0003 |
| Criterios de SignPath Foundation | En curso | ADR-0004 (el arquitecto los lee en la fuente) |
| Mensajería nativa MV3 en Windows y canal local | Pendiente | Primer día de la 0.2: lab con la extensión en modo desarrollador |
| Heurísticas de formularios de registro | Pendiente | 0.2: leer cómo lo hacen la extensión de KeePassXC y Bitwarden (ambas GPL) |
| Lector de pantalla y DPI sobre WebView2 | Pendiente | QA de la 0.1 |
| Publicar en Chrome Web Store | Pendiente | Inicio de la 1.0 |

## Quién hace qué
| Fase | Responsables |
|---|---|
| H0 | `rust-engineer` (scaffold, núcleo, instalador, actualizador, CI) · `security-engineer` (amenazas) · `solution-architect` (ADR) · Sara (plan, release, verificación) |
| 0.1 | `/especificar` → `brand-designer` + `design-references` → `/impeccable shape` (mock que aprueba el usuario) → `rust-engineer` (núcleo y comandos) + `react-next-engineer` (interfaz) + `test-engineer` |
| 0.2 | `/especificar` → `rust-engineer` (anfitrión, canal) + `react-next-engineer` (extensión) + `test-engineer` (Playwright sobre la lista de 20 sitios) |
| Cierre de cada fase | Completitud → verificación → `ui-qa` → `impeccable-finish-reviewer` → `code-reviewer`. `security-engineer` al cerrar 0.1, 0.2 y 1.0 |
| 1.0 | Lo anterior + `impeccable-documenter` (`DESIGN.md`) |

## Decisiones tomadas por defecto ("decide tú")
- Archivo llave **opcional y recomendado**, con aviso claro de que sin copia no hay recuperación. Se confirma en la spec 001.
- El aviso "¿guardar?" sale **dentro de la página**. Se confirma en la spec 002.
- Instalador por usuario (sin administrador). Lo fija el ADR-0003.

## Conflictos resueltos por la directora (2026-11-11)
- **Bloqueo con guardado colgado:** manda el orden de 4 pasos de AMENAZAS W3 (primero se niega el acceso, al final se
  intenta guardar con tiempo límite). ADR-0002 quedó alineado.
- **Volcado de WebView2:** se mide y se anota; lo que es pasa/falla es el canario en respuestas de comandos y en disco.
- **Anfitrión y pipe:** el anfitrión verifica quién creó el pipe antes de hablar (amenaza N3). Detalle en la spec 002.
- **Registro del anfitrión nativo:** sale del H0 y pasa a la 0.2.
- **Checklist:** ya está traducido a Rust/Tauri.

## Decisiones del usuario al aprobar el plan (2026-11-11)
Plan y los 5 ADR: **aceptados**. Repo público creado ese día.
1. **Releases 0.x sin firma Authenticode en un repo público.** E2/E3 piden firma antes del primer usuario externo y
   SignPath exige un release publicado para aceptar la solicitud. **Aceptado:** las 0.x salen marcadas "versión de
   desarrollo: no la uses con tus contraseñas reales"; E2-1 queda ❌ con ese motivo hasta la 1.0.
2. **Con SignPath, Windows muestra "SignPath Foundation" como editor**, no "Kattonavi" (el nombre de la app sí es el nuestro). **Aceptado.**
3. **Abierta, para la spec 002:** ¿la app confirma el primer uso de cada entrada desde el navegador? Un botón dentro de la
   página no cuenta como gesto seguro (amenaza N2). Se pregunta al empezar la 0.2.

## Pendientes
- **Usuario:** instalar las C++ Build Tools (bloquea el H0) · en el H0: activar 2FA en GitHub,
  generar la llave del actualizador con su contraseña y guardar el respaldo en 2 lugares fuera del repo, aprobar cada
  release en el entorno `release` · tras el primer release: enviar la solicitud a SignPath · en la 1.0:
  cuenta de la Chrome Web Store (tiene un pago único; es gasto y cuenta del usuario).
- **Sara:** instalar `pnpm`, `cargo-nextest`, `cargo-deny` y `tauri-cli` cuando el PC compile.
- `proyectos/vault-desktop` (Almario) no está en este disco: no se cuenta con sus fixtures.
