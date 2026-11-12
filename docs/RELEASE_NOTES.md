> **Versión de desarrollo: no la uses con tus contraseñas reales.**
> Ya crea y abre bóvedas KDBX 4 y guarda entradas, pero sigue en pruebas: no lleva firma de Windows ni revisión externa.
> Úsala con datos de prueba y guarda copia de cualquier archivo que abras con ella.

**Instalar:** descarga el archivo `*-setup.exe`. Se instala solo para tu usuario y no pide administrador.

**Windows va a advertir** ("Windows protegió su PC"): el instalador todavía no lleva firma de Windows. Antes de ejecutarlo
puedes comprobar que es el que publicamos:

- Suma: compara `Get-FileHash <archivo> -Algorithm SHA256` con el archivo `SHA256SUMS` de esta página.
- Procedencia (si el release la trae): `gh attestation verify <archivo> -R Kattonavi/almarioks`.

**Actualizaciones:** la app avisa cuando hay una versión nueva y solo la descarga si aceptas. Cada actualización va
firmada y la app verifica la firma antes de instalar.

Cambios: ver [CHANGELOG.md](https://github.com/Kattonavi/almarioks/blob/main/CHANGELOG.md) ·
[Code signing policy](https://github.com/Kattonavi/almarioks#firma-de-código-code-signing-policy).
