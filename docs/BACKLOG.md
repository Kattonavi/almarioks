# Backlog — AlmarioKS

Lo que no cabe en la fase va aquí, un renglón por ítem: `- [origen] qué · severidad o valor`.

- [lab 19] Probar bóvedas con XML que KeePassLib no conoce (datos de plugins fuera de CustomData) · Medio
- [lab 19] Política para abrir KDBX 3.1: convertir con aviso o solo lectura · se decide en la spec 001
- [H0] Dependabot: `minimatch` (solo desarrollo, DoS, GHSA-g3hv-75wg-m249) y `glib` (solo Linux, no se compila en Windows, GHSA-wrw7-89jp-8q8g) · Medio, no afectan el binario
- [lab] El crate no conserva el orden de los ítems de `CustomData` (HashMap): sin pérdida, pero el archivo cambia aunque no se edite · Bajo
- [H0] Aviso del Firewall de Windows para Node.js al correr `pnpm tauri dev` · Info
- [H0] El desinstalador silencioso deja la carpeta de datos de WebView2 (17 MB): decidir si se ofrece borrarla (amenaza W2) · Medio, seguridad: spec 001
- [H0] Prueba automática de que un `invoke` a un comando no listado falla (hoy solo está por configuración) · Medio: inicio de la 0.1, cuando exista el primer comando
- [H0] Repetir la instalación con un usuario estándar de Windows (el ensayo, el runner y la instalación desde GitHub corrieron con cuenta administradora); crear la cuenta es del dueño · Medio
- [H0] PR #2 de Dependabot (grupo `rust`, 2 actualizaciones): revisar y fusionar al empezar la 0.1 · Bajo
- [H0] PR #3 de Dependabot (TypeScript 6 → 7, versión mayor): probar `pnpm typecheck`, `lint`, `test` y `build` antes de aceptar; si rompe, agregar la regla de `ignore` para mayores · Medio
- [revisión H0, S3] El consentimiento de actualizar vive solo en la interfaz: al pintar contenido ajeno en la 0.1, revisar W1 (un script en la ventana podría instalar una actualización firmada sin clic) · Bajo hoy, revisar en la spec 001
- [visión] Significado público de "KS" · Bajo
- [base] Agregar al `.gitignore` los ajustes locales de Claude Code cuando el proyecto tenga carpeta `.claude/` · Bajo
- [0.1, T9] Cada cambio confirmado vuelve a derivar la llave al cifrar (~1 s en una bóveda calibrada; medido 53 ms con 64 MiB e I = 2) y las lecturas esperan detrás: sacar el cifrado de la hebra de la orden o reutilizar la llave derivada si la librería lo permite · Alto para la experiencia; revisar antes de la 1.0
- [0.1, T9] `lock()` y `close()` pueden tardar hasta 5 s escribiendo la recuperación aunque el estado ya sea bloqueado: la app los llama fuera de la hebra de la ventana · Medio
- [0.1, T9] Casos que hoy responden `Ocupada` o `ArgumentoInvalido` por falta de código propio (desbloquear con una bóveda abierta, guardado anterior aún en el disco, recuperación en curso) · Bajo: decidir códigos en la 0.2
- [0.1, T6] La poda del historial por tamaño (`HistoryMaxSize`) usa una estimación no contrastada con KeePass y no cuenta adjuntos; las bóvedas nuevas llevan 10 versiones y 6 MiB sin verificar que sean los valores de KeePass · Medio
- [0.1, T15] En CI las pruebas reales del portapapeles se saltan si el runner no tiene estación de ventana y nextest las muestra como aprobadas: agregar `--success-output final` a ese paso · Medio
- [0.1, T15] Sincronización del portapapeles con la nube sin verificar (estaba apagada en el PC de pruebas) · Medio: prueba manual de la 1.0
- [0.1, lab] `zxcvbn` da 4 de 4 a contraseñas de 11 caracteres con sustituciones conocidas: el aviso de contraseña maestra débil (< 3) casi nunca salta. Decide el dueño si se sube el umbral · Medio
- [0.1] KDBX 4.0 sin fixture propia: todas las de prueba son 4.1 · Bajo
- [0.1, RF-37] Bloqueo inmediato al bloquear la sesión de Windows: recortable no hecho; el bloqueo por tiempo real sí cubre la suspensión · Medio
