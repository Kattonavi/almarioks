# Changelog

Formato: [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/). Versiones: [SemVer](https://semver.org/lang/es/).
Las versiones 0.x son de desarrollo: no las uses con tus contraseñas reales.

## [Unreleased]

## [0.1.0] - 2026-11-12
Primera versión que guarda contraseñas. Sigue siendo de desarrollo: no la uses con tus contraseñas reales.

### Added
- Crear una bóveda KDBX 4 con contraseña maestra, archivo llave o ambos, y abrir las hechas con KeePass 2 o KeePassXC.
  Las bóvedas KDBX 3.x se abren en solo lectura y se pueden convertir a un archivo nuevo.
- Entradas y grupos: crear, ver, editar, mover y borrar, con papelera e historial de versiones compatible con KeePass.
- Búsqueda que no distingue mayúsculas ni tildes.
- Generador de contraseñas con medidor de fortaleza.
- Copiar sin mostrar: lo copiado no entra al historial del portapapeles de Windows y se borra solo a los 20 segundos.
- Bloqueo manual y por inactividad (5 minutos por defecto). Al bloquear se vacía el portapapeles y la ventana.
- Cada cambio se guarda solo, con una copia de respaldo `.bak` junto a la bóveda. Si otra app cambió el archivo, no se
  sobrescribe.
- Ajustes: tiempo de bloqueo y de limpieza del portapapeles.
- Identidad visual "Señalética" e icono nuevo; tema claro y oscuro; ventana inicial de 1120 × 720.

### Changed
- La ventana corre en modo privado y con el autocompletado apagado: WebView2 no guarda en disco lo que se escribe.
- Antes de instalar una actualización, la app cierra la bóveda y vacía el portapapeles.

### Known issues
- Guardar un cambio tarda uno o dos segundos en una bóveda con la protección recomendada.
- Sin probar todavía con KeePassXC en esta versión (sí con KeePass 2.62).

## [0.0.2] - 2026-11-11
Sin cambios en la app: esta versión existe para probar la actualización de la 0.0.1 a la 0.0.2 desde GitHub.

## [0.0.1] - 2026-11-11
### Added
- Ventana mínima que muestra el nombre y la versión de la app.
- Instalador de Windows por usuario (no pide administrador).
- Botón para buscar e instalar actualizaciones, con tu permiso y firma verificada.
- Prueba de ida y vuelta de bóvedas KDBX 4 hechas con KeePass 2 y con KeePassXC.

[Unreleased]: https://github.com/Kattonavi/almarioks/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Kattonavi/almarioks/compare/v0.0.2...v0.1.0
[0.0.2]: https://github.com/Kattonavi/almarioks/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/Kattonavi/almarioks/releases/tag/v0.0.1
