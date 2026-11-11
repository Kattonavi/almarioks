# Backlog — AlmarioKS

Lo que no cabe en la fase va aquí, un renglón por ítem: `- [origen] qué · severidad o valor`.

- [lab 19] Probar bóvedas con XML que KeePassLib no conoce (datos de plugins fuera de CustomData) · Medio
- [lab 19] Política para abrir KDBX 3.1: convertir con aviso o solo lectura · se decide en la spec 001
- [H0] Dependabot: `minimatch` (solo desarrollo, DoS, GHSA-g3hv-75wg-m249) y `glib` (solo Linux, no se compila en Windows, GHSA-wrw7-89jp-8q8g) · Medio, no afectan el binario
- [lab] El crate no conserva el orden de los ítems de `CustomData` (HashMap): sin pérdida, pero el archivo cambia aunque no se edite · Bajo
- [H0] Aviso del Firewall de Windows para Node.js al correr `pnpm tauri dev` · Info
- [H0] El desinstalador silencioso deja la carpeta de datos de WebView2 (17 MB): decidir si se ofrece borrarla (amenaza W2) · Medio, seguridad: spec 001
- [H0] Prueba automática de que un `invoke` a un comando no listado falla (hoy solo está por configuración) · Medio: inicio de la 0.1, cuando exista el primer comando
- [H0] Repetir la instalación con un usuario estándar de Windows (el ensayo corrió con consola elevada) · Medio
- [revisión H0, S3] El consentimiento de actualizar vive solo en la interfaz: al pintar contenido ajeno en la 0.1, revisar W1 (un script en la ventana podría instalar una actualización firmada sin clic) · Bajo hoy, revisar en la spec 001
- [visión] Significado público de "KS" · Bajo
- [base] Agregar al `.gitignore` los ajustes locales de Claude Code cuando el proyecto tenga carpeta `.claude/` · Bajo
