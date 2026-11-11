# Experimento: KDBX 4 desde Rust (crate `keepass` 0.17)

> 2026-11-11 · Hecho antes de planear AlmarioKS, para decidir el ADR-0001. Datos falsos.

## Pregunta
¿El crate `keepass` 0.17.0 con `save_kdbx4` lee y reescribe una bóveda KDBX 4 hecha por KeePass 2 sin perder nada, y
KeePass 2 abre lo que Rust escribe (contraseña, archivo llave, los dos)? Es el supuesto que puede tumbar el plan.

## Hipótesis
Lee bien; al escribir pierde algo (el README dice "experimental support for KDBX4.1 writing").

## Método
1. `correr.ps1` carga `KeePass.exe` 2.62 como ensamblado y usa **KeePassLib** (la implementación de referencia) para
   crear 4 bóvedas KDBX 4.1 con Argon2id: solo contraseña, solo archivo llave (`.keyx` v2), los dos con ChaCha20, y
   contraseña con `ñ`, `ú` y `€`. Cada una lleva: 3 grupos anidados, notas, etiquetas y datos propios de grupo, icono
   propio, entrada con campos propios (uno protegido), 2 adjuntos (uno protegido), 2 versiones de historial,
   etiquetas, vencimiento, colores, URL alterna, auto-escritura, datos propios de entrada, objeto borrado, datos
   propios de la base y datos públicos de cabecera.
2. `kdbx-lab rewrite` abre y guarda sin tocar. `edit` cambia una contraseña con historial y agrega una entrada con
   adjunto. `create` hace una bóveda nueva (ChaCha20 + Argon2id 64 MiB) y su archivo llave v2 desde Rust.
3. KeePassLib vuelve a abrir cada salida y se comparan resúmenes línea por línea (55 líneas por bóveda).

Reproducir: `cd kdbx-lab && cargo build --release` y luego
`powershell -NoProfile -ExecutionPolicy Bypass -File correr.ps1`.

## Resultado (salida real)
```
KeePassLib 2.62
== 1. Reescritura sin cambios: KeePass 2 -> Rust (open + save) -> KeePass 2 ==
  [pass] formato 4.1 -> 4.1 | version=KDB4(1) kdf=Argon2id { iterations: 2, memory: 67108864, parallelism: 2, version: Version13 } rust_reopen_equal=true
  [pass] IDENTICO (55 lineas)
  [key] IDENTICO (55 lineas)
  [both] IDENTICO (55 lineas)
  [unicode] IDENTICO (55 lineas)
== 2. Edicion en Rust -> KeePass 2 ==
  entrada nueva completa (campo protegido, adjunto, contraseña "pässwörd-日本語-🔑");
  GitHub: Password = cambiada-en-rust-ñ, historial 2 -> 3, todo lo demás igual
== 3. Boveda y archivo llave creados en Rust -> KeePass 2 ==
  abre con contraseña Unicode + nueva.keyx; sin archivo llave: rechazada (InvalidCompositeKeyException)
== Lineas distintas en la reescritura sin cambios: 0 ==
```
La hipótesis era falsa: **no se perdió nada de lo medido**.

## Lo que encontré y cambia el diseño
1. **El historial queda al revés.** `edit_tracking` inserta la versión nueva en la posición 0; KeePass 2 las guarda
   de la más vieja a la más nueva. Tras editar en Rust, KeePass 2 muestra la última versión de primera. El núcleo debe
   ordenar el historial por fecha antes de guardar y tener una prueba que lo fije.
2. **El KDF por defecto del crate es débil:** Argon2d con 1 MiB de memoria y 50 iteraciones. AlmarioKS fija el suyo
   (Argon2id, 64 MiB o más) al crear la bóveda; nunca `DatabaseConfig::default()` a secas.
3. **El crate no crea archivos llave** y al leer un `.keyx` v2 no valida su `Hash`. El generador es nuestro (32 bytes
   aleatorios en hex + 4 bytes de SHA-256; KeePass 2.62 lo acepta) y la validación del hash también.
4. `Database::save` solo escribe KDBX 4. Una bóveda 3.1 se abre pero no se puede guardar: hay que convertirla a 4
   con aviso, o abrirla en solo lectura.
5. `Value::Protected` usa `secrecy::SecretBox` y `DatabaseKey` se borra al soltarse (`ZeroizeOnDrop`): sirve para E3-3.

## Ampliación del mismo día: KeePassXC 2.8.0 (portable, suma SHA-256 verificada contra su `.DIGEST`)
- **Abre lo que Rust escribe:** `keepassxc-cli ls`, `show` y `db-info` leen las bóvedas reescritas y la editada en Rust
  (contraseña, solo archivo llave, ambos con ChaCha20 + Argon2id), con adjuntos, etiquetas y la contraseña con `ñ`.
- **Ida y vuelta de un archivo escrito por KeePassXC:** una bóveda KDBX 4 a la que KeePassXC le agregó una entrada, un
  adjunto y una versión de historial pasa por Rust (abrir + guardar) y `keepassxc-cli export -f xml` de antes y después
  da 571 líneas en ambos. Comparadas sin importar el orden queda **1 línea distinta** (`<Generator>`); con orden, 46:
  el crate guarda los datos propios en un `HashMap` y **no conserva el orden de los ítems de `CustomData`**. No se
  pierde contenido.
- **`keepassxc-cli` no crea KDBX 4:** `db-create` e `import` dan KDBX 3.1 con AES-KDF (2 intentos). El crate la abre
  (`KDB3(1)`) y al guardar devuelve `UnsupportedVersion`, como decía el punto 4. Para tener un KDBX 4 escrito por
  KeePassXC se parte de uno de KeePass 2 y se edita con `keepassxc-cli add/edit`.
- El `.keyx` que genera KeePassXC trae la llave en hex con espacios y saltos de línea: el crate lo lee bien.

## Límites (lo que NO probé)
- Elementos XML que KeePassLib no conoce (datos de plugins fuera de `CustomData`), bóvedas grandes y fuzzing (E3-4).
- KDBX 3.1 y bóvedas con AES-KDF. Entradas en la papelera.

## Entorno: dos trampas de Windows
- `cargo build` con el toolchain MSVC falla sin las C++ Build Tools: `linker link.exe not found`.
- El toolchain GNU (`stable-x86_64-pc-windows-gnu`, solo para este experimento) trae `ld` pero su `dlltool.exe` necesita
  `as.exe`, que no viene: falla en cualquier crate con `raw-dylib` (`getrandom` 0.4). Salida: copiar `llvm-ar.exe`
  (componente `llvm-tools`) como `llvm-dlltool.exe` y compilar con `RUSTFLAGS="-C dlltool=<ruta>"`. Es un atajo de
  laboratorio; el proyecto usa MSVC, que es lo que pide Tauri.

## Lección
La librería sirve para la 0.1 sin parches. Lo que hay que envolver en `almarioks-core`: orden del historial,
parámetros del KDF, archivo llave y política para KDBX 3.1.

## Destino
`docs/adr/0001-formato-kdbx4-crate-keepass.md` y la spec de la bóveda. En el H0 pasa a ser la prueba de ida y vuelta
de `almarioks-core`.
