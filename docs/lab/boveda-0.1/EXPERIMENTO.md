# Experimento: preguntas técnicas de la fase 0.1 (bóveda)

> 2026-11-11 · Fase de aprendizaje antes de implementar `docs/specs/001-boveda/spec.md`. Datos falsos.
> Windows 11 x64, Rust 1.99.1 (MSVC), 16 hilos lógicos. Cada pregunta: respuesta, fuente, ejercicio, salida real y
> qué cambia en el diseño. Si el ejercicio contradice lo leído, manda el ejercicio.

## Cómo reproducir
Crate independiente (`[workspace]` vacío, no entra al workspace del proyecto). Desde esta carpeta:

```
powershell -NoProfile -ExecutionPolicy Bypass -File generar-kdbx3.ps1   # KDBX 3.1 en %TEMP% con KeePassLib 2.62
cargo run --release --bin p4_keepass
cargo run --release --bin p1_clipboard    # OJO: pisa el portapapeles de quien lo corra
cargo run --release --bin p3_atomic
cargo run --release --bin p5_secrets
cargo run --release --bin p2_session      # termina solo a los pocos segundos
cargo run --profile devkdf --bin p4_keepass   # debug con rust-argon2 optimizado (ver P4 b)
```
Las fixtures se leen de `crates/almarioks-core/tests/fixtures` (no se modifican) y se copian a memoria o a `%TEMP%`.

---

## P4. Crate `keepass` =0.17.0 (RF-2, RF-11, RF-13, RF-14, RF-16, RF-46 a RF-58)

**Fuente:** código del crate en el registro de cargo (`keepass-0.17.0/src`: `db/open.rs`, `format/mod.rs`,
`format/kdbx4/parse.rs`, `format/kdbx3.rs`, `format/variant_dictionary.rs`, `config.rs`, `crypt/kdf.rs`,
`db/types/{mod,group,entry,meta}.rs`) y https://docs.rs/keepass/0.17.0/keepass/.

### Respuestas
- **(a) Parámetros del KDF antes de derivar: el crate no los expone.** `Database::parse` lee la cabecera y deriva la
  llave en la misma función; `parse_outer_header` y `VariantDictionary::parse` son privados (`pub(crate)`). Lo único
  público sin derivar es `Database::get_version`. **Hace falta un pre-lector propio** del TLV de la cabecera y del
  `VariantDictionary` del KDF (~120 líneas, no es criptografía: no deriva ni descifra). El ejercicio lo hace en
  9–36 µs y rechaza `M=64 GiB` sin llamar al crate. Detalles que el pre-lector debe cubrir, sacados del código:
  - Si el campo 11 (KDF) se repite, el crate se queda con el último: se valida **cada** aparición.
  - `crypt/kdf.rs` hace `(memory / 1024) as u32` y `iterations as u32`: los límites se comparan en `u64`, antes.
  - **El crate entra en pánico con cabeceras mal formadas** (verificado, 3 casos): lee campos de tamaño fijo con
    `byteorder::LittleEndian::read_u32/u64`, que hace `panic` si el búfer es corto (compresión de 1 byte, `UInt32` de
    1 byte en un `VariantDictionary`, rondas AES de 1 byte en KDBX 3.1). El pre-lector valida el tamaño de cada campo
    fijo y de cada valor tipado del `VariantDictionary` (KDF **y** datos públicos, campo 12). Además, `open` del
    núcleo envuelve `Database::parse` en `catch_unwind` como segunda barrera.
- **(b) Argon2id propio al crear.** `DatabaseConfig` es `#[non_exhaustive]`: no se puede escribir con literal; se parte
  de `DatabaseConfig::default()` y se sobrescriben **todos** sus campos públicos. `KdfConfig::Argon2id` pide un
  `argon2::Version` (de `rust-argon2`, que `keepass` no reexporta); para no agregar ese crate de criptografía al
  núcleo, se construye por la vía pública `VariantDictionary` + `TryFrom` (`$UUID`, `M`, `I`, `P`, `V`, `S`), y
  el ejercicio lo confirma. `save()` genera sal y semillas nuevas. **Calibrar:** `crypt` es privado, así que se mide
  `Database::parse` de una bóveda vacía guardada en memoria (≈ el tiempo del KDF). En este PC, release: 64 MiB, P=2,
  I=42 → 966 ms. **En debug es 7–8 veces más lento** (I=2 → 432 ms; 64 MiB I=4 → 855 ms): con
  `[profile.dev.package.rust-argon2] opt-level = 3` vuelve al tiempo de release (I=43 → 978 ms).
- **(c) Papelera y objetos borrados: sí, como datos; no hay un "mover a papelera".** `meta.recyclebin_enabled`,
  `meta.recyclebin_uuid`, `meta.recyclebin_changed` y `Database::recycle_bin()` existen, y `deleted_objects` es un
  `HashMap<Uuid, Option<NaiveDateTime>>` que se guarda y se relee. Las fixtures de KeePass 2 traen
  `RecycleBinUUID = 00000000-…` (no hay papelera hasta el primer borrado): `recycle_bin()` da `None` y la creamos
  nosotros (grupo con icono 43, `enable_searching=false`, `enable_autotype=false`, y los tres campos de `meta`).
  - Borrado definitivo: `EntryTrack::remove()` y `GroupTrack::remove()` (vía `track_changes()`) **sí** anotan en
    `deleted_objects` (y el grupo, recursivo, anota cada hijo); `EntryMut::remove()` **no** lo anota. Usar siempre
    la variante `track_changes()`.
- **(d) Mover conservando UUID y tiempos: sí.** `EntryMut::move_to(GroupId)` y `GroupMut::move_to(GroupId)`
  conservan UUID, `creation` y `last_modification`, y fijan `previous_parent_group` (se guarda en el XML). El de grupo
  rechaza ciclos (`WouldCreateCycle`) y la raíz (`CannotMoveRoot`), que es RF-55 y parte de RF-57.
  - **Trampa:** `EntryTrack::move_to` (la variante "con seguimiento") **agrega una versión al historial** al soltarse
    (`impl Drop for EntryTrack` empuja la copia previa), aunque solo se movió. KeePass no hace eso. Para mover:
    `EntryMut::move_to` y `times.location_changed = Some(Times::now())` a mano. Para grupos `GroupTrack::move_to`
    sí sirve (no tiene `Drop`; solo actualiza `location_changed`).
- **(e) KDBX 3.1 al abrir:** `Database::get_version(&mut reader)` lee 12 bytes y da `DatabaseVersion::KDB3(1)` sin
  derivar; también queda en `db.config.version`. `save()` de un 3.1 devuelve `DatabaseSaveError::UnsupportedVersion`.
  Convertir (RF-15) = clonar, poner nuestro `config` KDBX 4 y guardar en otra ruta: reabre con la misma raíz y las
  mismas entradas, y el original queda idéntico byte a byte.
- **(f) Errores que distingue al abrir** (tabla medida abajo):
  - **KDBX 4:** llave incorrecta → `Key(IncorrectKey)` (el HMAC de la cabecera no verifica). Contenido dañado →
    `Format(Kdbx4(BlockStream(…)))`. Cabecera dañada → `HeaderHashMismatch` u `OuterHeader(…)`. Archivo corto →
    `UnexpectedEof` / `VersionParse(…)`. Versión 5 → `VersionParse(InvalidKDBXVersion)`.
  - **Límite real:** si alguien cambia la cabecera **y** recalcula su SHA-256, el crate deriva con esos parámetros y
    responde `IncorrectKey`: una cabecera manipulada no se distingue de una llave mala (es del formato, no del crate).
  - **KDBX 3.1 no tiene HMAC de cabecera:** la llave incorrecta sale como `Cryptography(InvalidPadding)`, el mismo
    error que un archivo truncado. Para cumplir RF-11 se mapea por versión: en 3.1, `Cryptography(_)` →
    `LlaveIncorrecta` (un 3.1 truncado también dirá "llave incorrecta"; se acepta y se documenta).
  - **KDB (KeePass 1):** el crate **sí lo intenta abrir** (`parse_kdb`); `FormatoNoSoportado` lo decidimos nosotros
    con `get_version` antes de llamar a `parse`.
  - Los enums son `#[non_exhaustive]`: el `match` necesita comodín y el comodín cae en `ArchivoDanado`.

### Ejercicio
`src/bin/p4_keepass.rs`: pre-lector propio, 22 casos de apertura (con `catch_unwind` y un `panic_hook` que solo imprime
el mensaje), calibración, papelera, movimientos y conversión 3.1 → 4. `generar-kdbx3.ps1` crea el 3.1 con KeePassLib
2.62. Hallazgo del script: `PwDatabase.Save` de KeePass 2.62 escribe KDBX **4.0** aunque el KDF sea AES-KDF; para un 3.1
hay que usar `KdbxFile` con `ForceVersion` (setter interno, por reflexión).

### Salida real (`cargo run --release --bin p4_keepass`)
```
== P4 (e) version sin derivar la llave: Database::get_version ==
  keepass2/f-pass.kdbx           -> KDB4(1) (KDBX4.1)
  keepassxc/xc-both4.kdbx        -> KDB4(1) (KDBX4.1)
  %TEMP%/v3-aes.kdbx             -> Ok(KDB3(1))

== P4 (a) parametros KDF de la cabecera ANTES de derivar (pre-lector propio) ==
  keepass2/f-pass.kdbx       v4.1 [Argon2 { id: true, memory: 67108864, iterations: 2, parallelism: 2 }] en 36 us
  keepass2/f-key.kdbx        v4.1 [Argon2 { id: true, memory: 67108864, iterations: 2, parallelism: 2 }] en 22 us
  keepass2/f-both.kdbx       v4.1 [Argon2 { id: true, memory: 67108864, iterations: 2, parallelism: 2 }] en 11 us
  keepass2/f-unicode.kdbx    v4.1 [Argon2 { id: true, memory: 67108864, iterations: 2, parallelism: 2 }] en 9 us
  keepassxc/xc-both4.kdbx    v4.1 [Argon2 { id: true, memory: 67108864, iterations: 2, parallelism: 2 }] en 11 us
  %TEMP%/v3-aes.kdbx         v3.1 [Aes { rounds: 60000 }]

  Cabecera manipulada (SHA-256 de cabecera recalculado, como haria un atacante):
  I=4 (era 2): el crate deriva y luego falla     118 ms | LlaveIncorrecta <- Key(IncorrectKey)
                                                        | pre-lector OK v4.1
  M=64 GiB: pre-lector -> Some(Excessive("M=68719476736 I=2 P=2")) en 3 us (el crate NO se llama: intentaria reservar 64 GiB)
  I=1e6: pre-lector -> Some(Excessive("M=67108864 I=1000000 P=2"))

== P4 (f) que error devuelve el crate en cada caso (y el codigo de la spec que le damos) ==
  0 bytes                                          0 ms | ArchivoDanado <- VersionParse(UnexpectedEof)
                                                        | pre-lector Truncated
  texto plano                                      0 ms | ArchivoDanado <- VersionParse(InvalidKDBXIdentifier)
                                                        | pre-lector NotKdbx
  truncado en la cabecera (100 B)                  0 ms | ArchivoDanado <- Format(Kdbx4(OuterHeader(UnexpectedEof)))
                                                        | pre-lector Truncated
  truncado entre hash y HMAC                       0 ms | ArchivoDanado <- UnexpectedEof
                                                        | pre-lector OK v4.1
  truncado en el flujo de bloques                 93 ms | ArchivoDanado <- Format(Kdbx4(BlockStream(Eof)))
                                                        | pre-lector OK v4.1
  1 bit cambiado en el contenido cifrado          61 ms | ArchivoDanado <- Format(Kdbx4(BlockStream(BlockHashMismatch { block_index: 0 })))
                                                        | pre-lector OK v4.1
  1 bit cambiado en la cabecera (sin rehash)       0 ms | ArchivoDanado <- Format(Kdbx4(OuterHeader(ParseCustomData(NotTerminated))))
                                                        | pre-lector Truncated
  1 bit cambiado en la sal del KDF (sin rehash)     0 ms | ArchivoDanado <- Format(Kdbx4(HeaderHashMismatch))
                                                        | pre-lector OK v4.1
  contrasena incorrecta                           57 ms | LlaveIncorrecta <- Key(IncorrectKey)
                                                        | pre-lector OK v4.1
  contrasena buena + archivo llave sobrante       59 ms | LlaveIncorrecta <- Key(IncorrectKey)
                                                        | pre-lector OK v4.1
  llave sin componentes                            0 ms | LlaveVacia <- Key(EmptyKey)
                                                        | pre-lector OK v4.1
  f-both: contrasena buena sin archivo llave      56 ms | LlaveIncorrecta <- Key(IncorrectKey)
                                                        | pre-lector OK v4.1
  f-both: archivo llave sin contrasena            56 ms | LlaveIncorrecta <- Key(IncorrectKey)
                                                        | pre-lector OK v4.1
  f-both: llave completa                          56 ms | ABRE (KDBX4.1)
                                                        | pre-lector OK v4.1
  version mayor 5                                  0 ms | FormatoNoSoportado <- VersionParse(InvalidKDBXVersion { version: 3041655655, file_major_version: 5, file_minor_version: 1 })
                                                        | pre-lector Unsupported("signature 0xb54bfb67, version 5.1")
  firma KDB (KeePass 1) con basura                 0 ms | ArchivoDanado <- Format(Kdb(InvalidFixedCipherID(0)))
                                                        | pre-lector Unsupported("KDB (KeePass 1)")
      (panic: range end index 4 out of range for slice of length 1)
  cabecera con campo de compresion de 1 byte       0 ms | PANIC (el proceso caeria sin catch_unwind)
                                                        | pre-lector Malformed("field 3 len 1 != 4")
      (panic: range end index 4 out of range for slice of length 1)
  datos publicos con un UInt32 de 1 byte           0 ms | PANIC (el proceso caeria sin catch_unwind)
                                                        | pre-lector Malformed("VariantDictionary item of type 0x4 with 1 bytes")
  KDBX 3.1: llave correcta                         1 ms | ABRE (KDBX3.1)
                                                        | pre-lector OK v3.1
  KDBX 3.1: contrasena incorrecta                  0 ms | LlaveIncorrecta <- Cryptography(InvalidPadding(Error))
                                                        | pre-lector OK v3.1
  KDBX 3.1: 1 bit cambiado al final                0 ms | ArchivoDanado <- Format(Kdbx3(BlockHashMismatch(0)))
                                                        | pre-lector OK v3.1
  KDBX 3.1: truncado                               0 ms | LlaveIncorrecta <- Cryptography(InvalidPadding(Error))
                                                        | pre-lector OK v3.1
      (panic: range end index 8 out of range for slice of length 1)
  KDBX 3.1: campo de rondas de 1 byte              0 ms | PANIC (el proceso caeria sin catch_unwind)
                                                        | pre-lector Malformed("u64 field is not 8 bytes")

== P4 (b) crear con Argon2id propio y calibrar (perfil release, 16 hilos logicos) ==
  DatabaseConfig::default().kdf_config = Argon2 { iterations: 50, memory: 1048576, parallelism: 4, version: Version13 }
  M=64 MiB P=2 I=2   -> desbloqueo 57 ms
  M=64 MiB P=2 I=4   -> desbloqueo 101 ms
  M=64 MiB P=2 I=8   -> desbloqueo 193 ms
  M=64 MiB P=2 I=16  -> desbloqueo 374 ms
  M=64 MiB P=2 I=32  -> desbloqueo 752 ms
  calibrado a ~1 s: I=42 -> 966 ms
  cabecera escrita: v4.1 [Argon2 { id: true, memory: 67108864, iterations: 42, parallelism: 2 }]
  config al reabrir: Argon2id { iterations: 42, memory: 67108864, parallelism: 2, version: Version13 }
  otra llave -> Err("LlaveIncorrecta")
  referencia M=256 MiB P=4 I=2 -> 157 ms

== P4 (c)(d) papelera, objetos borrados y mover ==
  fixture f-pass: RecycleBinEnabled=Some(true) RecycleBinUUID=Some(00000000-0000-0000-0000-000000000000) recycle_bin()=None DeletedObjects=1
  papelera creada por nosotros: true
  EntryTrack::move_to agrega historial: Some(2) -> Some(3)
  RF-47 mover a papelera -> guardar -> reabrir:
    misma UUID: true | padre = papelera: true | grupo anterior = Trabajo: true
    creation igual: true | last_modification igual: true | location_changed Some(2026-11-11T17:27:17) -> Some(2026-11-12T01:39:52)
    historial: Some(2) -> Some(2) | contrasena intacta: true
    meta al reabrir: RecycleBinEnabled=Some(true) RecycleBinUUID==papelera: true
  RF-55 mover Trabajo dentro de su hijo Subgrupo -> Some(WouldCreateCycle)
  RF-57 mover la raiz -> Some(CannotMoveRoot)
  RF-54 mover Subgrupo a la raiz: misma UUID true | padre raiz true | creation igual true | location_changed cambio true
  RF-48 EntryTrack::remove en la papelera: DeletedObjects 1 -> 2 (tras reabrir: 2, contiene UUID: true)
  EntryMut::remove (sin track_changes): DeletedObjects 2 -> 2 (no lo anota)
  RF-56 Trabajo (2 entradas, 1 subgrupos) a la papelera: la papelera tiene 2 entradas
  RF-58 vaciar papelera: DeletedObjects 1 -> 5 | grupos 4 -> 2 | papelera sigue: true | entradas en papelera 0

== P4 (e) KDBX 3.1: abrir, guardar y convertir ==
  config.version = KDB3(1) kdf = Aes { rounds: 60000 }
  primera entrada: Some((Some("Cuenta v3"), Some("ana@example.com")))
  save() del 3.1 tal cual -> Some(UnsupportedVersion)
  convertido: KDBX4.1 con Argon2id { iterations: 2, memory: 67108864, parallelism: 2, version: Version13 }, misma raiz: true, entradas 1 -> 1
  original identico byte a byte: true
```

Calibración en debug (`cargo run --bin p4_keepass`) y en debug con `rust-argon2` optimizado (`--profile devkdf`):
```
== P4 (b) crear con Argon2id propio y calibrar (perfil debug, 16 hilos logicos) ==
  M=64 MiB P=2 I=2   -> desbloqueo 432 ms
  M=64 MiB P=2 I=4   -> desbloqueo 855 ms
  calibrado a ~1 s: I=4 -> 863 ms
  referencia M=256 MiB P=4 I=2 -> 880 ms
  f-both: llave completa                         399 ms | ABRE (KDBX4.1)
--- devkdf (inherits dev + [profile.devkdf.package.rust-argon2] opt-level = 3)
  M=64 MiB P=2 I=32  -> desbloqueo 735 ms
  calibrado a ~1 s: I=43 -> 978 ms
  referencia M=256 MiB P=4 I=2 -> 164 ms
  f-both: llave completa                          63 ms | ABRE (KDBX4.1)
```

### Qué cambia en el diseño
1. `almarioks-core::kdbx` lleva un **pre-lector de cabecera** propio que corre antes de `Database::parse`: firma y
   versión (KDB/KDBX2/≥5 → `FormatoNoSoportado`), tamaño de cada campo fijo, cada valor del `VariantDictionary`
   (campos 11 y 12), y los límites de RF-16 en `u64` sobre **todas** las apariciones. RF-16 deja de ser opcional en
   la práctica: sin el pre-lector, un archivo hostil **tumba el proceso** (pánico) o reserva memoria sin límite.
   Además, `catch_unwind` alrededor de `Database::parse` → `ArchivoDanado`. Pruebas: los 3 pánicos de la tabla.
2. Mapeo de errores **por versión**: en 3.1, `Cryptography(_)` → `LlaveIncorrecta`. Prueba `rf_11_*` con un 3.1.
3. Crear: `DatabaseConfig::default()` + sobrescribir todo; KDF por `VariantDictionary` (sin `rust-argon2` directo).
   Calibrar **solo en release**; en `Cargo.toml` raíz: `[profile.dev.package.rust-argon2] opt-level = 3` para que
   `pnpm tauri dev` no tarde 8× ni calibre débil. Política de calibración propuesta: M = 64 MiB, P = 2 (como KeePass),
   subir I hasta ≈ 1 s (mínimo 2). En este PC da I≈42; con 256 MiB y P=4, I=2 son 157 ms, así que si se prefiere
   más memoria la decisión es de la spec (RF-2 solo pide ≥ 64 MiB y ≥ 2 iteraciones).
4. Mover entradas con `EntryMut::move_to` + `location_changed` a mano (no `EntryTrack`). Borrar siempre con
   `track_changes().remove()`. Papelera: crearla al primer borrado si `recycle_bin()` es `None` o está desactivada.
   Guardas propias: no mover ni renombrar la papelera (RF-57), y `GroupTrack::remove` sobre la papelera no limpia
   `meta.recyclebin_uuid` (lo hace solo `GroupMut::remove`): nunca borrar la papelera por esa vía.
5. KDBX 3.1: `get_version` al abrir → modo solo lectura; convertir = clonar + `config` nuevo + guardar en otra ruta.

---

## P3. Guardado atómico en Windows con `std`, sin `unsafe` (RF-21, RF-22, RF-23, RF-27)

**Fuente:** https://doc.rust-lang.org/std/fs/fn.rename.html (en Windows reemplaza un archivo destino existente),
https://doc.rust-lang.org/std/fs/fn.copy.html (copia también los permisos) y
https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all. `ReplaceFileW` conserva el respaldo en una sola
llamada, pero es FFI con `unsafe` y el núcleo lo prohíbe: se resolvió con `std`. Hecho por el hilo principal.

### Respuestas
- `std::fs::rename` sobre un destino existente **lo reemplaza** en un paso (probado en NTFS).
- `sync_all` del temporal antes de renombrar: sí, para que el contenido esté en disco antes de que el nombre apunte a él.
- **Si otro proceso tiene abierto el destino sin `FILE_SHARE_DELETE`** (un editor, un antivirus), `rename` falla con
  `PermissionDenied` (error 5) y la bóveda queda intacta: es un `GuardadoFallido` normal. Con `FILE_SHARE_DELETE`
  (como abre KeePass al leer) el reemplazo funciona.
- **Un destino con atributo de solo lectura** también da `PermissionDenied`.
- **Orden del respaldo.** La primera versión del ejercicio promovía la copia a `.bak` antes del reemplazo final: cuando
  el reemplazo falló, el respaldo viejo ya se había perdido (`bak = v3` con la bóveda en `v3`). Orden corregido:
  1. escribir `<bóveda>.tmp`, `sync_all`, releer y descifrar;
  2. `fs::copy(bóveda, <bóveda>.bak.tmp)` y quitarle el atributo de solo lectura a la copia (`fs::copy` lo hereda y un
     `.bak` de solo lectura hace fallar el siguiente guardado: pasó en el ejercicio);
  3. `rename(tmp → bóveda)`;
  4. solo si 3 salió bien, `rename(bak.tmp → bak)`.
  La bóveda nunca deja de existir, y si 3 falla el `.bak` anterior sigue ahí.
- Un temporal sobrante de un corte anterior no afecta la bóveda y el siguiente guardado lo reemplaza.

### Salida real (`cargo run --release --bin p3_atomic`)
```
== P3 guardado atomico con std (sin unsafe) ==
  tras guardar v2: boveda=v2 bak=v1
  tras guardar v3: boveda=v3 bak=v2
  archivos en la carpeta: ["cuentas.kdbx", "cuentas.kdbx.bak"]
== destino abierto por otro (handle sin FILE_SHARE_DELETE, como el Bloc de notas o un antivirus) ==
  guardar v4 -> Err((PermissionDenied, Some(5))) | boveda sigue = v3 | bak = v2
  temporal que queda: true
== destino abierto por otro CON FILE_SHARE_DELETE (como KeePass al leer) ==
  guardar v5 -> Ok(()) | boveda = v5
== carpeta de solo lectura simulada: destino con atributo de solo lectura ==
  guardar v6 -> Err((PermissionDenied, Some(5))) | boveda = v5
== corte entre escribir el temporal y renombrar (se simula no llamando a rename) ==
  boveda = v5 (el temporal sobrante no la afecta y el siguiente guardado lo pisa)
  tras guardar v7: boveda=v7 bak=v5
```

### Qué cambia en el diseño
`almarioks-core::storage` usa el orden de 4 pasos de arriba, limpia el temporal cuando el guardado falla y trata
`PermissionDenied` como `GuardadoFallido` con un mensaje que sugiere cerrar la otra app que tiene el archivo abierto.
No probado: unidades de red, FAT/exFAT y carpetas sincronizadas (OneDrive, Dropbox).

---

## P5. Fortaleza y tipos que se borran (RF-7, RF-70; ADR-0002)

**Fuente:** https://docs.rs/zxcvbn/3.1.1/zxcvbn/ (MIT, `cargo info zxcvbn@3.1.1`), https://docs.rs/secrecy/0.10.3/secrecy/
y https://docs.rs/zeroize/1.9.1/zeroize/. Hecho por el hilo principal.

### Respuestas
- `zxcvbn::zxcvbn(texto, &[])` devuelve `score()` de 0 a 4, convertible con `u8::from`. Sin `default-features` no trae
  dependencias de tiempo ni de WebAssembly. No es criptografía: es un estimador.
- Los umbrales son generosos: una contraseña de 11 caracteres con sustituciones conocidas ya da 4. El aviso de RF-7
  ("menor que 3") solo saltará con contraseñas muy malas. Se deja el puntaje tal cual, como dice la spec, y se anota
  para el dueño: si quiere un aviso más exigente, se cambia el umbral, no el estimador.
- `secrecy::SecretString` no imprime su contenido con `Debug` (`[REDACTED]`) y se borra al soltarse; `Zeroizing<T>`
  hace lo mismo con búferes. Un `String` que llega por `invoke` se envuelve de inmediato en `SecretString`.

### Salida real (`cargo run --release --bin p5_secrets`)
```
== P5 zxcvbn: puntaje 0-4 ==
  ""                               -> score 0 (log10 intentos -inf)
  "123456"                         -> score 0 (log10 intentos 0.3)
  "password"                       -> score 0 (log10 intentos 0.5)
  "Tr0ub4dor&3"                    -> score 4 (log10 intentos 11.0)
  "correct horse battery staple"   -> score 4 (log10 intentos 19.3)
  "lab-Maestra-2026"               -> score 4 (log10 intentos 14.0)
  "ñandú-€-2026"                   -> score 4 (log10 intentos 10.0)
  "kV7#pQ2!xW9@mL4$zR8&"           -> score 4 (log10 intentos 19.3)
== P5 secrecy / zeroize ==
  Debug de SecretString: SecretBox<str>([REDACTED])
  largo expuesto a proposito: 16
  Zeroizing<Vec<u8>> largo 8 (se borra al soltarse)
```

---

## P1. Portapapeles fuera de Win+V y de la nube (RF-71 a RF-75, RF-90)

**Fuente:** https://learn.microsoft.com/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats,
https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber y
https://learn.microsoft.com/uwp/api/windows.applicationmodel.datatransfer.clipboard. Ejercicio escrito por
`rust-engineer` y corrido por el hilo principal (pisa el portapapeles de quien lo corra; solo usa canarios falsos).

### Respuestas
- **Cómo se marca.** En la misma apertura del portapapeles, antes del texto, se ponen tres formatos registrados con
  `RegisterClipboardFormatW`: `ExcludeClipboardContentFromMonitorProcessing` (basta su presencia),
  `CanIncludeInClipboardHistory` = `DWORD` 0 y `CanUploadToCloudClipboard` = `DWORD` 0. Luego `CF_UNICODETEXT`.
- **"Sigue siendo lo nuestro" sin leer el secreto:** `GetClipboardSequenceNumber()` justo después de `CloseClipboard`.
  Una copia nuestra con 4 formatos sube el número en 8, así que no se predice: se lee. La limpieza abre el
  portapapeles, compara el número **con el portapapeles abierto** (nadie puede cambiarlo entre la comparación y
  `EmptyClipboard`) y solo entonces vacía.
- **Hace falta una ventana dueña.** `OpenClipboard(NULL)` + `EmptyClipboard` funcionó aquí aunque la documentación dice
  que `SetClipboardData` falla sin dueño; no se confía en eso: se usa una ventana oculta de solo mensajes
  (`HWND_MESSAGE`), que además sirve para P2.
- **El historial de Win+V sí se puede verificar por programa** con WinRT (`Clipboard::GetHistoryItemsAsync`): el
  canario marcado no aparece y el de control (sin marcas) sí. Con solo `CanIncludeInClipboardHistory = 0` tampoco
  aparece, pero se ponen los tres. La nube no se pudo verificar (la sincronización está apagada en este PC).
- Los búferes intermedios (UTF-16 y bytes) se borran con `zeroize` después de entregarlos.

### Salida real (`cargo run --release --bin p1_clipboard`)
```
== P1 portapapeles ==
  formatos registrados: Exclude...=0xc266 CanIncludeInClipboardHistory=0xc24e CanUploadToCloudClipboard=0xc24d

  [1] OpenClipboard(NULL) + EmptyClipboard + SetClipboardData (la doc dice que falla):
    resultado: Ok(860) | hay texto: true | dueño NULL: true

  [2] Copia marcada con dueño = ventana oculta de solo mensajes:
    secuencia antes 860 -> despues de CloseClipboard 868 (+8 por 1 copia con 4 formatos)
    dueño = nuestra ventana: true | hay texto: true

  [3] (a) nadie mas copia: limpieza condicional
    clear_if_ours -> Cleared | hay texto despues: false | secuencia ahora 869

  [4] (b) otra app copia antes de la limpieza
    otro proceso (cmd + clip.exe) copio "CANARIO-AJENO-hola": Ok(true)
    secuencia nuestra 877, ahora 882 | dueño = nuestra ventana: false
    clear_if_ours -> NotOurs { now: 882 }
    el portapapeles conserva lo ajeno: Some("CANARIO-AJENO-hola")

  [5] (c) copiamos otra contraseña antes de la limpieza: vale solo la ultima secuencia
    limpieza vieja (secuencia 890) -> NotOurs { now: 898 }
    limpieza nueva (secuencia 898) -> Cleared | hay texto: false

  [6] Historial de Win+V por programa (WinRT Clipboard):
    IsHistoryEnabled = Ok(true) | IsRoamingEnabled = Ok(false)
      (canario de control borrado del historial: Ok(true))
    control (sin marcas de historial) en el historial: Ok(true)
    marcado (3 formatos) en el historial:              Ok(false)
    solo CanIncludeInClipboardHistory=0:              Ok(false)
    limpieza del historial (restos del lab, incluido el canario ajeno): Ok(0) elementos

  portapapeles vaciado al salir: hay texto = false
```

### Qué cambia en el diseño
- `src-tauri::clipboard` (módulo `ffi`, único con `unsafe`): ventana oculta de solo mensajes como dueña, copia con los
  tres formatos, y limpieza por número de secuencia comparado con el portapapeles abierto. Un `trait` de portapapeles
  permite probar la lógica (ejemplo 1 de la spec) con uno falso y reloj inyectable; el real se prueba con este canario.
- RF-72 deja de ser solo manual: la prueba de la app puede consultar el historial por WinRT cuando está activado.
- Dependencia: `windows` =0.62.2 (MIT o Apache-2.0), solo en `src-tauri`.

---

## Pendiente de este laboratorio
P2 (sesión de Windows) y P6 (Tauri:
comandos asíncronos, instancia única, diálogos, recarga de la webview). Se retoman antes de las tareas T13 a T16.
