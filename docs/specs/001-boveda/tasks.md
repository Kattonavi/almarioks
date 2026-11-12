# Tareas — Spec 001 Bóveda

Cada tarea: sus RF y "Hecho cuando:" ejecutable. Una a la vez: prueba que falla → código → suite en verde (salida
real) → commit → marcar. Las pruebas citan el RF (`rf_NN_…`, `it("RF-NN: …")`). Comando base del núcleo:
`cargo nextest run -p almarioks-core --locked`; de la app: `cargo nextest run -p almarioks --locked`; de la interfaz:
`pnpm test`. Antes de cada commit: `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`.

## Núcleo (`almarioks-core`)
- [ ] T1. Esqueleto: módulos `kdbx`, `vault`, `storage`, `search`, `generator`, `strength`, `settings`, `error` con
      tipos propios y funciones que compilan y devuelven un valor neutro; dependencias fijadas. (RF: —)
      Hecho cuando: `cargo nextest run --workspace --locked` y `cargo deny check` en verde; `cargo tree -p almarioks-core -i tauri` vacío.
- [ ] T2. Abrir: pre-lector propio de la cabecera (versión, tamaños de campo, topes del KDF) antes de la librería;
      llave con contraseña, archivo llave o ambos; formatos de archivo llave; errores estables por versión. (RF-10 a RF-13, RF-16, RF-87, RF-94)
      Hecho cuando: esas `rf_*` en verde contra las fixtures de KeePass 2 y KeePassXC, incluido el ejemplo 7 y las tres cabeceras que tumban a la librería (`docs/lab/boveda-0.1`).
- [ ] T3. Crear: Argon2id calibrado, `.keyx` v2 propio con relectura, rechazos. (RF-1 a RF-6)
      Hecho cuando: `rf_01_*` a `rf_06_*` en verde; la cabecera de la bóveda creada dice Argon2id, ≥ 64 MiB, 2 a 50 iteraciones.
- [ ] T4. KDBX 3.x en solo lectura y conversión; aviso de KDF débil. (RF-14, RF-15, RF-17)
      Hecho cuando: `rf_14_*`, `rf_15_*`, `rf_17_*` en verde; el original 3.x queda con el mismo SHA-256 tras convertir.
- [ ] T5. Modelo de lectura: árbol de grupos, lista y detalle con la regla de campo protegido, mostrar un campo. (RF-42 a RF-44, RF-50, RF-93)
      Hecho cuando: `rf_42_*`, `rf_43_*`, `rf_44_*`, `rf_50_*`, `rf_93_*` en verde, con una fixture de notas protegidas.
- [ ] T6. Entradas: crear y editar como parche, validaciones, historial. (RF-26, RF-38 a RF-41, RF-45, RF-91)
      Hecho cuando: esas `rf_*` en verde, incluidos los ejemplos 8 y 9.
- [ ] T7. Mover y borrar entradas y grupos; papelera y objetos borrados. (RF-46 a RF-48, RF-51 a RF-58)
      Hecho cuando: esas `rf_*` en verde, incluidos los ejemplos 3 y 4.
- [ ] T8. Guardado: cifrar a bytes, temporal, relectura, reemplazo atómico, `.bak`, cambio externo, disco de prueba
      que falla o corta en cada paso, cola de guardados; ida y vuelta sin pérdida tras editar. (RF-21 a RF-25, RF-27, RF-89)
      Hecho cuando: esas `rf_*` en verde, incluido el ejemplo 6; las pruebas `roundtrip_*` siguen en verde.
- [ ] T9. Sesión: `Closed` / `Locked` / `Open`, desbloqueo único, bloqueo que no espera al disco, recuperación desde
      bytes ya cifrados, aviso al desbloquear. (RF-28, RF-30, RF-33 a RF-36, RF-88)
      Hecho cuando: esas `rf_*` en verde con el disco colgado y con el disco que falla; en ambos el estado queda `Locked` en < 1 s.
- [ ] T10. Búsqueda. (RF-59 a RF-63, RF-92)
      Hecho cuando: esas `rf_*` en verde, incluido el ejemplo 2 y una propiedad de plegado con semilla fija; 5 000 entradas en < 100 ms (medido y anotado).
- [ ] T11. Generador y fortaleza. (RF-7, RF-65 a RF-70)
      Hecho cuando: esas `rf_*` en verde, incluido el ejemplo 5 y la estadística con semilla fija.
- [ ] T12. Ajustes y bóvedas recientes. (RF-19, RF-20, RF-82)
      Hecho cuando: esas `rf_*` en verde, con un archivo de ajustes dañado.

## App (`src-tauri`)
- [ ] T13. Comandos del contrato de `plan.md`, errores `{ code, message }`, lista de comandos en `build.rs` y permisos
      en la capacidad; prueba de contrato con canario; `invoke` no listado. (RF-76, RF-79, RF-80, RF-83, RF-93)
      Hecho cuando: `rf_76_*`, `rf_79_*`, `rf_80_*`, `rf_83_*` en verde sobre la app de pruebas de Tauri.
- [ ] T14. Bloqueo en la app: inactividad con reloj inyectable y tiempo real, evento `vault-locked`, recarga de la
      webview, cierre de la app. (RF-29, RF-31, RF-32, RF-36)
      Hecho cuando: `rf_29_*` (incluido el ejemplo 10), `rf_31_*`, `rf_32_*` en verde.
- [ ] T15. Portapapeles desde Rust (`ffi`): copiar, exclusión de historial y nube, limpieza por número de secuencia,
      al bloquear y al salir; copiar y bloquear en serie. (RF-71 a RF-75, RF-90)
      Hecho cuando: `rf_71_*` a `rf_75_*` y `rf_90_*` en verde, incluido el ejemplo 1 (a–d).
- [ ] T16. Diálogos de archivo desde Rust, abrir URL, instancia única. (RF-49, RF-81)
      Hecho cuando: `rf_49_*` en verde; el complemento de instancia única está registrado primero (`rf_81_*`).

## Interfaz (`src/`), funcional y sin dirección visual (D11)
- [ ] T17. Estructura: capa tipada de comandos con validación en el borde, máquina de estados cerrada / bloqueada /
      abierta, bienvenida (P1) y desbloqueo (P3). (RF-10, RF-11, RF-18, RF-19, RF-20, RF-28, RF-32, RF-34, RF-83)
      Hecho cuando: `pnpm typecheck`, `pnpm lint` y `pnpm test` en verde con `it("RF-…")` por estado de P1 y P3.
- [ ] T18. Crear bóveda (P2). (RF-1, RF-3, RF-7, RF-8, RF-9)
      Hecho cuando: pruebas de P2 en verde (vacío, derivando, errores, éxito).
- [ ] T19. Bóveda (P4): árbol, lista, búsqueda, estados vacíos. (RF-42, RF-50, RF-59, RF-64, RF-78, RF-92)
      Hecho cuando: pruebas de P4 en verde, incluida la fixture XSS.
- [ ] T20. Entrada (P5) y generador (P6): ver, mostrar, copiar, crear, editar, mover, borrar. (RF-38, RF-43 a RF-49, RF-65 a RF-70, RF-74)
      Hecho cuando: pruebas de P5 y P6 en verde.
- [ ] T21. Grupos (P7) y papelera. (RF-51 a RF-58)
      Hecho cuando: pruebas de P7 en verde.
- [ ] T22. Ajustes (P8), avisos de guardado (`GuardadoFallido`, `CambioExterno`), solo lectura y conversión. (RF-14, RF-15, RF-17, RF-23, RF-24, RF-29, RF-73, RF-82)
      Hecho cuando: pruebas de P8 y de los avisos en verde.

## Recortables (solo si el corte de 8 h no se disparó)
- [ ] T23. Historial de una entrada (RF-85).
- [ ] T24. Bloqueo inmediato con la sesión de Windows (RF-37).
- [ ] T25. Adjuntos: agregar, quitar, guardar en disco (RF-86).

## Cierre
- [ ] T26. Compatibilidad: una bóveda creada y editada por la app abre en KeePassLib 2.62 y `keepassxc-cli` sin pérdida.
      Hecho cuando: el script de `docs/lab/` lo muestra con salida real.
- [ ] T27. Validación: `/especificar validar` → `validacion.md` con cada RF y su prueba, más pruebas escritas desde
      la spec por un agente que no vio la implementación (bloqueo, portapapeles, guardado, secretos fuera de la ventana). (Todos)
      Hecho cuando: ningún RF no recortado sin prueba o verificación declarada.
- [ ] T28. Entrega: instalador de depuración; en la ventana real, crear → abrir → entrada con contraseña generada →
      buscar → copiar → bloquear → desbloquear, y la matriz de completitud (crear, ver, editar, mover, borrar × entradas y grupos).
      Hecho cuando: el recorrido corre sobre la app instalada y su salida queda en `docs/specs/001-boveda/validacion.md`.

<!-- Formato adaptado de mouredev/hello-sdd (Apache-2.0). -->
