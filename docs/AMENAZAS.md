# AlmarioKS — modelo de amenazas (E3-1, E2-8)

> 2026-11-11 · `security-engineer` · antes del código. Base: visión, ADR-0002 a 0005, `docs/lab/kdbx-roundtrip`,
> `estandares/escritorio.md` y `rust.md`, lecciones de Almario. Leído hoy en la fuente: Chrome native messaging y
> `runtime.MessageSender`, Tauri updater e Isolation, OWASP Desktop App Security Top 10. Lo que viene de memoria lleva
> "(s/v)" = sin verificar. **Prioridad = probabilidad × impacto (1–3) antes de mitigar:** 7–9 Crítico · 5–6 Alto ·
> 3–4 Medio · 1–2 Bajo. Crítico y Alto bloquean la fase donde su frontera se entrega por primera vez (sección 4).

## 1. Activos y fronteras de confianza

| Activo | Dónde vive | Si sale |
|---|---|---|
| Contenido de la bóveda | `.kdbx` cifrado; en claro solo en el núcleo en `Open` | todo |
| Maestra y `.keyx` | teclado → webview → núcleo (un `invoke`, ADR-0002) | todo, incluidas copias viejas del archivo |
| Llave privada del actualizador | secretos de Actions + respaldo del usuario (ADR-0003) | todos los usuarios |
| Llave de asociación extensión–bóveda | datos propios de la bóveda y `chrome.storage` (ADR-0005) | pedir credenciales por origen con la app abierta |
| Integridad de repo y releases | GitHub público | todos los usuarios |

| Frontera | Dirección de la confianza | Decide | Dueño |
|---|---|---|---|
| F-W webview ↔ núcleo | React con cientos de dependencias npm → `almarioks-core` | el núcleo, en cada comando | rust-engineer |
| F-N página ↔ extensión ↔ anfitrión ↔ app | página hostil → script → service worker → stdio → pipe → app | la app (`core::matching`) | rust-engineer |
| F-F archivo y memoria | disco, portapapeles, archivo de otra app → núcleo | el núcleo | rust-engineer |
| F-U actualización y suministro | Releases, npm, crates.io → binario | firma del actualizador, Authenticode | usuario + hilo principal |
| F-C repo y CI | PR de cualquiera → Actions → Release | entorno `release` | usuario + hilo principal |

## 2. STRIDE por frontera

| ID | STRIDE | Amenaza | P×I | Mitigación (lo que decide) | Prueba que la demuestra |
|---|---|---|---|---|---|
| **W1** | T E I | XSS con datos no confiables (campos de un `.kdbx` ajeno, CSV, usuario capturado en una página) que llama comandos y saca secretos | 2×3=6 Alto | Sin `dangerouslySetInnerHTML` ni markdown; CSP sin `unsafe-*` y `connect-src` solo IPC; capacidades a mano, sin `fs`/`shell`/`http`; abrir URL solo vía núcleo, `http(s)` y con clic; ningún comando masivo; exportar pide la maestra; Isolation recomendado por Tauri (en Windows su app no carga módulos ES) | fixture KDBX con `<img onerror>`, `<script>` y `javascript:` en cada campo: nada se ejecuta ni se abre; `invoke` a lo no listado falla |
| **W2** | I | La maestra y las contraseñas nuevas se teclean en la webview: cadenas JS sin borrado; WebView2 puede guardar campos en su carpeta de datos (s/v) | 2×2=4 Medio | `type=password`, `autocomplete=off`, `spellcheck=false`; un `invoke` y vaciar (ADR-0002); autocompletar de WebView2 apagado (s/v cómo lo expone Tauri); recargar la webview al bloquear | canario tecleado: 0 apariciones en la carpeta de datos de WebView2 (s/v: `EBWebView\`); en volcado, medido y anotado |
| **W3** | E I | El bloqueo falla abierto: el guardado se cuelga o falla y la bóveda sigue visible (Almario F7) | 2×3=6 Alto | Orden: (1) estado a bloqueando y todo comando con secretos responde `Locked`; (2) la vista se desmonta y el portapapeles se limpia; (3) guardar o escribir `.recuperacion.kdbx` con tiempo límite; (4) soltar `db` y llaves en un `finally`. Si (3) falla, lo pendiente se pierde y se avisa al desbloquear | guardado que cuelga y que falla: en ≤ 1 s los comandos dan `Locked`, la vista es la de bloqueo y el portapapeles está vacío |
| **W4** | I | La contraseña copiada queda en Win+V, sube a la nube o no se limpia | 3×2=6 Alto | Copia desde Rust (ADR-0002), nunca desde JS; excluir de historial y nube (s/v: `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`, `CanUploadToCloudClipboard`); limpiar a los N s solo si sigue siendo lo nuestro, y al bloquear | tras copiar, Win+V no tiene el canario; a los N s está vacío; si el usuario copió otra cosa, se respeta |
| **F1** | I | Fuerza bruta del `.kdbx` robado (nube del usuario, portátil); el KDF por defecto del crate es Argon2d de 1 MiB (lab 19) | 3×3=9 Crítico | Argon2id ≥ 64 MiB, iteraciones calibradas a ~1 s; nunca `DatabaseConfig::default()`; `zxcvbn`; archivo llave recomendado; avisar al abrir una bóveda débil | bóveda nueva: Argon2id y memoria ≥ 64 MiB; KeePassLib la reabre con esos parámetros |
| **F2** | D | Archivo con parámetros de KDF absurdos (cabecera sin autenticar antes de derivar) o malformado | 2×2=4 Medio | Acotar memoria, iteraciones y paralelismo antes de derivar; fuzzing de la cabecera (E3-4) | fixtures extremos rechazados en < 1 s; `cargo fuzz` 10 min sin pánico |
| **F3** | T D | Corrupción al guardar (corte, disco lleno); historial al revés (lab 19); KDBX 3.1 no guardable | 2×3=6 Alto | Temporal en la misma carpeta, releer y descifrar, reemplazo atómico; versión anterior conservada; historial ordenado; 3.1 en solo lectura o conversión con aviso | matar el proceso al escribir: el original abre; historial de viejo a nuevo en KeePassLib; 3.1 no se sobrescribe sin confirmar |
| **F4** | D | El crate no crea ni valida `.keyx`: uno corrupto al crear deja la bóveda irrecuperable. Es el único código propio que produce material de llave (roza E3-2) | 1×3=3 Medio | Generador mínimo en `core::kdbx`: 32 bytes del RNG del sistema, formato v2 con hash; validar el hash al leer; reabrir antes de dar por hecho | `.keyx` nuestro abre en KeePass 2 y KeePassXC; hash alterado da error claro |
| **F5** | I | Secretos a disco: logs, pánicos, `Debug`, volcados de WER, temporales, exportación | 2×3=6 Alto | `secrecy` sin `Debug`; logs sin datos de entradas; gancho de pánico sin valores; WER excluido (s/v la API); cero temporales en claro | recorrido con canario: 0 en `%LocalAppData%`, `%TEMP%` y logs |
| **N1** | S I | Relleno en origen equivocado: phishing, homógrafo, `http`, subdominio, `user@host`, iframe legítimo en página ajena | 3×3=9 Crítico | `core::matching` exacto (ADR-0005); el service worker toma el origen de `sender.origin`/`sender.url` y `frameId` (leído), nunca del mensaje del script; en subframes, solo si coincide con la página superior | 10 casos engañosos (`github.com.evil.io`, `github.com@evil.io`, homógrafo, puerto, iframe…): 0 rellenos y 0 credenciales en el registro |
| **N2** | I | Relleno sin intención: al cargar, formularios ocultos, capa transparente sobre el botón en la página, clics sintéticos | 3×3=9 Crítico | Nunca al cargar; ningún secreto antes del gesto; gesto fuera del DOM de la página (atajo de `chrome.commands`, menú contextual o ventana de la extensión, s/v); solo el campo visible y enfocado. Si queda el botón en la página: conflicto 3 | página con formulario oculto, capa transparente y `dispatchEvent(click)`: 0 credenciales pedidas |
| **N3** | S E | Pipe falso: con la app cerrada, otro usuario de Windows crea `almarioks-<SID>` y el anfitrión le entrega datos de "guardar" o se deja suplantar | 1×3=3 Medio | ADR-0005 cubre DACL, primera instancia y clientes remotos. Falta en el anfitrión: verificar que el dueño del proceso servidor es su mismo usuario y conectar sin permitir suplantación (s/v: `SECURITY_IDENTIFICATION`) | otro usuario crea el pipe antes: el anfitrión lo rechaza; prueba que lee los flags del cliente |
| **N4** | S E | Cliente no autorizado: otra extensión, `postMessage` de la página, emparejamiento sin aprobación, mensajes malformados | 2×3=6 Alto | `allowed_origins` exactos (leído: sin comodines) y el anfitrión comprueba su primer argumento; emparejamiento con código aprobado en la app, revocable; cada mensaje autenticado; el script no atiende `postMessage`; esquema estricto y tope de tamaño | sin asociación o revocada: solo `pair`/`status`; `postMessage` no dispara nada; 2 MiB o campos extra: descartado sin pánico |
| **N5** | S | La página dibuja un falso AlmarioKS y pide la maestra | 2×3=6 Alto | La maestra solo en la ventana de la app; con la app bloqueada, la extensión pide desbloquear allí y la trae al frente (s/v en Windows) | ningún campo de maestra en el código de la extensión; bloqueada, pedir rellenar abre la app |
| **N6** | I R | Extensión comprometida (cuenta de tienda, error propio) pide en masa; "¿actualizar?" abusado | 1×3=3 Medio | Protocolo sin listar ni buscar; límite por minuto; aviso por relleno y registro local (origen, entrada, hora, sin secretos); actualizar deja lo viejo en el historial; 2FA en tiendas | no existe operación de listado; 30 solicitudes/min frenadas; la contraseña previa queda en el historial |
| **U1** | T E | Actualización maliciosa: llave o release comprometidos, artefacto alterado, HTTP, versión vieja anunciada como nueva (`latest.json` no va firmado, s/v) | 2×3=6 Alto | Firma obligatoria (no se puede apagar) y TLS en producción; sin `dangerousInsecureTransportProtocol`; comprobar y descargar son llamadas explícitas, con "Actualizar" (ADR-0003); versión firmada obligatoria (`requireSignedVersion`: la firma lleva `version:` y el plugin la compara; el comparador `>` solo no frena una bajada de versión); bloquear antes de cerrar | `.sig` alterado y artefacto con otra llave: rechazados, sigue en N; endpoint `http://` rechazado; sin clic no hay descarga |
| **U2** | D | Pérdida de la llave del actualizador: la `pubkey` va horneada desde el primer instalador | 1×3=3 Medio (irreversible: bloquea H0) | Respaldo fuera del repo en dos lugares (ADR-0003); rotación escrita (s/v: publicar con la llave vieja la versión que trae la `pubkey` nueva) | firmar un build de prueba con la llave restaurada y que N lo acepte |
| **U3** | S T | Primera instalación falsa: 0.x sin Authenticode y públicos como "latest", sitios parecidos, DLL plantadas junto al instalador | 2×3=6 Alto | Descarga solo del Release; `SHA256SUMS` y atestación desde el H0; Authenticode (SignPath) antes del primer usuario externo; búsqueda de DLL restringida a System32 (s/v en el NSIS de Tauri) | `gh attestation verify` pasa; Process Monitor sin cargas desde Descargas; firma válida en 1.0 |
| **U4** | T E | Dependencia comprometida o vulnerable: npm, crates, WebView2 | 2×3=6 Alto | Lockfiles con `--locked`/`--frozen-lockfile`; `cargo deny` y `pnpm audit --prod` en CI; `keepass =0.17.0`; Dependabot con espera; extensión sin dependencias de ejecución ni código remoto (s/v: regla MV3) | ejecución verde de ambos; 0 dependencias de ejecución en la extensión |
| **C1** | E T I | CI abusado: PR de fork, inyección en expresiones de Actions, Action sin fijar, etiqueta ajena; secreto o bóveda real commiteados | 2×3=6 Alto | Sin `pull_request_target`; secretos solo en el entorno `release` con aprobación y etiquetas `v*` protegidas (s/v: rulesets); `permissions: contents: read`; Actions por SHA; protección de push activa (s/v); fixtures falsos; 2FA con llave física | PR de fork sin secretos; `zizmor`/`actionlint` limpios (s/v); etiqueta sin aprobación no publica |

## 3. OWASP Desktop App Security Top 10 (E2-8)

| Ítem | Aplica | Dónde |
|---|---|---|
| DA1 - Injections | Sí: HTML/JS en la webview, JSON del anfitrión; sin SQL ni shell | W1, N4 |
| DA2 - Broken Authentication & Session Management | Sí: desbloqueo, bloqueo, emparejamiento | W3, F1, N4, N5 |
| DA3 - Sensitive Data Exposure | Sí: memoria, logs, portapapeles, carpeta de WebView2 | W2, W4, F5 |
| DA4 - Improper Cryptography Usage | Sí: KDF y archivo llave; cero cripto propia | F1, F2, F4 |
| DA5 - Improper Authorization | Sí: capacidades, DACL del pipe, instalación por usuario | W1, N3 |
| DA6 - Security Misconfiguration | Sí: CSP, devtools, pipe, manifiesto en `HKCU` | W1, N3, N4 |
| DA7 - Insecure Communication | Parcial: la única red es el actualizador (TLS forzado) | U1 |
| DA8 - Poor Code Quality | Sí: firma, DLL preloading, carreras del bloqueo; `unsafe` solo en `ffi` (R-7) | U3, W3 |
| DA9 - Using Components with Known Vulnerabilities | Sí: crates, npm, WebView2 | U4 |
| DA10 - Insufficient Logging & Monitoring | Parcial: sin telemetría por diseño; registro local de accesos | N6, F5 |

## 4. Qué se exige en cada fase (copiar a las specs)

**[B]** = bloquea publicar la fase.

| Fase | Requisito verificable | Amenaza |
|---|---|---|
| **H0** | [B] Llave del actualizador con contraseña, solo en el entorno `release`; respaldo en 2 lugares y simulacro de firma desde el respaldo | U1, U2 |
| H0 | [B] `.sig` alterado y otra llave: rechazados; `http://` rechazado; sin clic no hay descarga | U1 |
| H0 | [B] Build de release: CSP sin `unsafe-*`, devtools fuera, capacidades explícitas (`allow-check`, `allow-download`, `allow-install`); `invoke` a lo no listado falla | W1 |
| H0 | [B] `release.yml` por etiqueta protegida con aprobación; Actions por SHA; `permissions` mínimas; sin `pull_request_target` | C1 |
| H0 | [B] Lockfiles con `--locked`/`--frozen-lockfile`; `cargo deny check` y `pnpm audit --prod` verdes (R-1, R-6) | U4 |
| H0 | [B] `SHA256SUMS` y atestación en el release; `gh attestation verify` pasa | U3 |
| H0 | Solicitud a SignPath enviada; instalación sin administrador | U3 |
| **0.1** | [B] Argon2id ≥ 64 MiB calibrado; aviso con bóveda débil | F1 |
| 0.1 | [B] Guardado atómico con relectura; historial ordenado; 3.1 no se sobrescribe sin confirmar | F3 |
| 0.1 | [B] Bloqueo en cuatro pasos a prueba de guardado colgado o fallido | W3 |
| 0.1 | [B] Portapapeles fuera de Win+V y la nube; limpieza a los N s y al bloquear | W4 |
| 0.1 | [B] Fixture XSS sin ejecución; URL solo `http(s)` vía núcleo; exportar pide la maestra | W1 |
| 0.1 | [B] Canario en disco: 0 apariciones (incluida la carpeta de WebView2) | F5, W2 |
| 0.1 | Límites del KDF y fuzz de cabecera; `.keyx` validado; Isolation | F2, F4, W1 |
| **0.2** | [B] 10 orígenes engañosos: 0 rellenos; origen tomado de `sender` | N1 |
| 0.2 | [B] Sin gesto no hay credencial (oculto, capa transparente, clic sintético) | N2 |
| 0.2 | [B] Emparejamiento aprobado en la app y revocable; `postMessage` ignorado; malformados descartados | N4 |
| 0.2 | [B] Ningún campo de maestra fuera de la ventana de la app | N5 |
| 0.2 | Pipe falso de otro usuario rechazado por el anfitrión; protocolo sin listado, límite y registro | N3, N6 |
| **1.0** | [B] Authenticode válido en binarios, instalador y actualizaciones; Process Monitor sin DLL plantadas | U3 |
| 1.0 | [B] E3-9: portapapeles, temporales y DLL sin hallazgos; volcados de núcleo y WebView2 medidos y anotados (no pasa/falla) | W2, F5 |
| 1.0 | [B] SBOM; extensión sin código remoto; 2FA en las tiendas | U4, N6 |
| 1.0 | `SECURITY.md` (Críticos ≤ 7 días); reversa de ADR-0003 ensayada | E3-7, E3-8 |

## 5. Fuera del modelo, sin rodeos

- **Malware con los privilegios del mismo usuario:** lee la memoria desbloqueada, inyecta en WebView2, cambia el
  anfitrión (manifiesto y carpeta son del usuario), lee `chrome.storage` y abre el pipe. Ni el emparejamiento ni la DACL
  lo frenan; solo achican lo que queda tras bloquear.
- **Keyloggers, capturas y lectores de portapapeles** mientras el secreto está en uso.
- **Administrador o SYSTEM comprometido**, controladores, firmware, acceso físico con el equipo desbloqueado.
- **Otra extensión maliciosa con permiso sobre la página:** ve lo que se rellena, como la página misma.
- **XSS o robo de sesión en el sitio legítimo:** si el origen es correcto, rellenar es correcto.
- **Maestra débil sin archivo llave:** el KDF solo encarece; se avisa, no se bloquea.
- **Perder la maestra o el `.keyx`:** no hay recuperación, por diseño.
- **Forks con malware bajo el mismo nombre** y la nube donde el usuario copie su `.kdbx`.

## 6. Conflictos con la arquitectura (los resuelve el director)

1. ADR-0002 pide 0 canarios en el volcado de WebView2, pero la maestra tecleada vive como cadena JS sin borrado: medir, no prometer (W2).
2. ADR-0005: el anfitrión no verifica al servidor del pipe; con la app cerrada, otro usuario puede crear el nombre (N3).
3. Un botón o aviso dentro de la página no sirve como gesto para rellenar; el gesto solo lo exige la extensión y la app no lo ve. ¿La app confirma el primer acceso por entrada? (N2, N6)
4. ADR-0003 publica los 0.x sin firma como "latest" en un repo público: choca con "firma antes del primer usuario externo" de E2/E3 (U3).
5. `docs/CHECKLIST.md` está redactado para .NET (E0-1, E2-3, E2-4, E2-9, E3-5): hay que pasarlo a Rust/Tauri.
