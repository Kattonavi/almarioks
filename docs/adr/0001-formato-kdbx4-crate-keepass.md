# ADR-0001: Formato KDBX 4 con el crate `keepass` =0.17.0, envuelto en `almarioks-core`
Estado: **aceptado** (formato KDBX 4 y licencia GPL-3.0: decisión del usuario, 2026-11-11) · **propuesto** (crate
`keepass`, envolturas, política 3.1 y plan de salida) · Fecha: 2026-11-11

**Contexto:** pilar 1 ("la bóveda es del usuario") y E3-2 (cero criptografía propia). Atributo conductor: compatibilidad
sin pérdida con KeePass 2 y KeePassXC. `docs/lab/kdbx-roundtrip/EXPERIMENTO.md`: abrir y guardar con `save_kdbx4` 4
bóvedas KDBX 4.1 de KeePassLib 2.62 da **0 líneas distintas** sobre 55 por bóveda; la creada en Rust abre en KeePass 2.
La licencia del crate (MIT) es compatible con GPL-3.0.

**Opciones y descartadas:** A `keepass` 0.17 · B formato propio (rompe el pilar 1 y E3-2) · C binding a KeePassXC
(segundo toolchain C++ y FFI con `unsafe`) · D otro crate KDBX en Rust (sin evidencia propia de escritura KDBX 4).

**Decisión:** Haremos A, fijado con `=0.17.0` en `Cargo.toml` y `Cargo.lock`. Ningún tipo de `keepass` sale de
`almarioks-core::kdbx`: el resto del código ve tipos propios. El módulo envuelve las 4 cosas que encontró el experimento:
1. **Historial:** se ordena de la versión más vieja a la más nueva antes de guardar (el crate inserta en la posición 0).
2. **KDF:** al crear, Argon2id con 64 MiB o más (se calibra a ≤ 1 s de desbloqueo en el H0). Nunca
   `DatabaseConfig::default()` (Argon2d con 1 MiB). En bóvedas existentes se respetan sus parámetros; si son débiles, aviso.
3. **Archivo llave:** generador propio de `.keyx` v2 (32 bytes aleatorios en hex + 4 bytes de SHA-256) y validación
   propia del `Hash` al leer, porque el crate no la hace.
4. **KDBX 3.1:** se abre en **solo lectura**. "Convertir a KDBX 4" escribe un archivo nuevo y deja el original intacto.
   Nunca se sobrescribe un 3.1 en sitio.

**Por qué es de una vía:** el formato es lo que queda guardado en el disco del usuario y lo leen otras apps.

**Consecuencias:** + interoperabilidad probada y secretos con `SecretBox`/`ZeroizeOnDrop` (sirve para E3-3) ·
− la escritura KDBX 4.1 del crate se declara "experimental" y depende de un solo mantenedor (sin verificar) ·
~ el núcleo carga con 4 envolturas y sus pruebas.

**Si el crate se abandona** (señal: más de 12 meses sin release, o un aviso RUSTSEC sin parche en 30 días): se hace fork
en `vendor/keepass/` con `vendor/UPSTREAM.md` (E3-2). Como ningún tipo del crate cruza `kdbx`, cambiarlo toca un solo módulo.

**Cómo lo verificamos:**
- Corpus en `crates/almarioks-core/tests/fixtures/`: bóvedas de KeePass 2.62 (las del lab) **y de KeePassXC** (falta,
  H0). Prueba de ida y vuelta: 0 diferencias en el resumen. Cuatro pruebas fijan las envolturas: orden del historial,
  KDF al crear, hash de `.keyx` y 3.1 sin escritura.
- Fuera de lo medido y **pendiente para el H0 o la 0.1**: KeePassXC, AES-KDF, papelera, XML desconocido (plugins),
  bóvedas grandes. "Sin pérdida" vale solo para lo medido.
- Fuzzing básico del parser con `cargo fuzz` (E3-4) en la 1.0. `cargo deny` vigila licencias y avisos (R-6).

**Revisar si:** falla la prueba con KeePassXC o con un caso pendiente; sale `keepass` 0.18; aparece un aviso RUSTSEC.
