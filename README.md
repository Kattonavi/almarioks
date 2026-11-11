# AlmarioKS

Gestor de contraseñas local y abierto. Tu bóveda es un archivo **KDBX 4** estándar, protegido con contraseña maestra,
archivo llave o ambos, que también abren KeePass 2 y KeePassXC. Sin cuenta, sin nube y sin telemetría.

> **Estado: H0 en curso (2026-11-11). Todavía no hay instalador publicado.**
> Las versiones 0.x serán **de desarrollo: no las uses con tus contraseñas reales**. Saldrán sin firma de Windows
> (SmartScreen va a advertir) hasta la 1.0; cada una llevará la firma del actualizador, `SHA256SUMS` y atestación.

## Qué va a hacer
- Crear y abrir una bóveda KDBX 4; entradas y grupos con búsqueda; generador de contraseñas.
- Extensión de navegador (Chrome, Edge, Brave): ofrecer una contraseña al crear una cuenta y preguntar si la guarda.
- Windows primero. Rust + Tauri 2.

## Cómo va
| Fase | Entrega | Estado |
|---|---|---|
| H0 | Instalador que instala, abre, actualiza con permiso y desinstala | ensayado en local ([evidencia](docs/H0-EVIDENCIA.md)); falta el primer release |
| 0.1 | Bóveda | pendiente |
| 0.2 | Navegador | pendiente |
| 1.0 | Publicable: firma, SBOM, importar CSV, inglés, accesibilidad | pendiente |

## Documentos
- [Plan y alcance](docs/PROYECTO.md) · [Arquitectura](docs/ARQUITECTURA.md) · [Decisiones](docs/adr/)
- [Modelo de amenazas](docs/AMENAZAS.md) · [Checklist de calidad](docs/CHECKLIST.md) · [UX](docs/UX.md)
- [Cómo se publica una versión](docs/RELEASE.md)
- [Experimento: KDBX 4 desde Rust sin pérdida](docs/lab/kdbx-roundtrip/EXPERIMENTO.md)

## Seguridad
Si encuentras una vulnerabilidad, repórtala en privado con "Report a vulnerability" en la pestaña Security de este
repositorio, no en un issue público. La política completa (`SECURITY.md`) llega con la 1.0.

## Licencia
[GPL-3.0](LICENSE).
