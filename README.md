# AlmarioKS

Gestor de contraseñas local y abierto. Tu bóveda es un archivo **KDBX 4** estándar, protegido con contraseña maestra,
archivo llave o ambos, que también abren KeePass 2 y KeePassXC. Sin cuenta, sin nube y sin telemetría.

> **Estado: versiones de desarrollo. Todavía no guarda contraseñas** (la bóveda llega con la 0.1).
> Las versiones 0.x son **de desarrollo: no las uses con tus contraseñas reales**. Salen sin firma de Windows
> (SmartScreen va a advertir) hasta la 1.0; cada una lleva la firma del actualizador, `SHA256SUMS` y atestación.
>
> **Descargar:** [último release](https://github.com/Kattonavi/almarioks/releases/latest). Se instala solo para tu
> usuario y no pide administrador.

## Qué va a hacer
- Crear y abrir una bóveda KDBX 4; entradas y grupos con búsqueda; generador de contraseñas.
- Extensión de navegador (Chrome, Edge, Brave): ofrecer una contraseña al crear una cuenta y preguntar si la guarda.
- Windows primero. Rust + Tauri 2.

## Cómo va
| Fase | Entrega | Estado |
|---|---|---|
| H0 | Instalador que instala, abre, actualiza con permiso y desinstala | hecho: `v0.0.1` y `v0.0.2` ([evidencia](docs/H0-EVIDENCIA.md)) |
| 0.1 | Bóveda | pendiente |
| 0.2 | Navegador | pendiente |
| 1.0 | Publicable: firma, SBOM, importar CSV, inglés, accesibilidad | pendiente |

## Compilar
Requisitos: Rust (el canal lo fija `rust-toolchain.toml`), las C++ Build Tools de Visual Studio, Node 26 y pnpm 12.

```
pnpm install
pnpm tauri dev      # ventana de desarrollo
pnpm test           # pruebas de la interfaz
cargo nextest run --workspace --locked
```
El instalador firmado solo lo construye el workflow de release (`docs/RELEASE.md`).

## Documentos
- [Plan y alcance](docs/PROYECTO.md) · [Arquitectura](docs/ARQUITECTURA.md) · [Decisiones](docs/adr/)
- [Modelo de amenazas](docs/AMENAZAS.md) · [Checklist de calidad](docs/CHECKLIST.md) · [UX](docs/UX.md)
- [Cómo se publica una versión](docs/RELEASE.md)
- [Experimento: KDBX 4 desde Rust sin pérdida](docs/lab/kdbx-roundtrip/EXPERIMENTO.md)

## Firma de código (Code signing policy)
- **Hoy (0.x):** el instalador no lleva firma de Windows (Authenticode). Lleva la firma del actualizador (la app la
  verifica antes de instalar cada actualización), `SHA256SUMS` y atestación de procedencia
  (`gh attestation verify <archivo> -R Kattonavi/almarioks`).
- **De dónde salen los binarios:** solo del workflow [`release.yml`](.github/workflows/release.yml), a partir de una
  etiqueta `v*` sobre `main`, y cada publicación se aprueba a mano. No se publican binarios compilados en un PC.
- **Roles:** autor, revisor y quien aprueba cada release: [Kattonavi](https://github.com/Kattonavi).
- **Para la 1.0:** la firma de Windows se va a solicitar a SignPath Foundation ([estado](docs/SIGNPATH.md)).

## Privacidad
AlmarioKS no envía información a otros sistemas. La única conexión de red ocurre cuando tú pulsas "Buscar
actualizaciones": consulta y descarga desde GitHub Releases. No hay cuenta, nube ni telemetría.

## Seguridad
Si encuentras una vulnerabilidad, repórtala en privado con "Report a vulnerability" en la pestaña Security de este
repositorio, no en un issue público. La política completa (`SECURITY.md`) llega con la 1.0.

## Licencia
[GPL-3.0](LICENSE).
