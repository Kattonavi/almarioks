//! P4: what the `keepass` 0.17.0 API gives (and does not give) for RF-2, RF-11, RF-13, RF-14, RF-16, RF-46..RF-58.
//!
//! Fixtures are read from `crates/almarioks-core/tests/fixtures` (never modified) and copied into memory.
//! The KDBX 3.1 file comes from `generar-kdbx3.ps1` (KeePassLib 2.62) in `%TEMP%\almarioks-boveda-lab`.
//! Output never prints absolute paths: only file names.

use std::{
    collections::HashMap,
    fs,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    time::{Duration, Instant},
};

use keepass::{
    Database, DatabaseKey,
    config::{
        CompressionConfig, DatabaseConfig, DatabaseVersion, InnerCipherConfig, KdfConfig, OuterCipherConfig,
        VariantDictionary,
    },
    db::{DatabaseOpenError, GroupId, Times, fields},
    error::{DatabaseKeyError, DatabaseVersionParseError, KdfConfigError},
};
use sha2::{Digest, Sha256};

const PASSWORD: &str = "lab-Maestra-2026";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../crates/almarioks-core/tests/fixtures")
}

fn read_fixture(rel: &str) -> Vec<u8> {
    fs::read(fixtures().join(rel)).unwrap_or_else(|e| panic!("fixture {rel}: {}", e.kind()))
}

fn kdbx3_path() -> PathBuf {
    std::env::temp_dir().join("almarioks-boveda-lab").join("v3-aes.kdbx")
}

fn key(password: Option<&str>, keyfile: bool) -> DatabaseKey {
    let mut key = DatabaseKey::new();
    if let Some(p) = password {
        key = key.with_password(p);
    }
    if keyfile {
        let bytes = read_fixture("keepass2/llave.keyx");
        key = key.with_keyfile(&mut bytes.as_slice()).expect("key file");
    }
    key
}

// ---------------------------------------------------------------------------------------------------------
// Our own header pre-reader (RF-16). Not cryptography: it walks the TLV header and the KDF VariantDictionary.
// ---------------------------------------------------------------------------------------------------------

const KDF_ARGON2D: [u8; 16] = [
    0xef, 0x63, 0x6d, 0xdf, 0x8c, 0x29, 0x44, 0x4b, 0x91, 0xf7, 0xa9, 0xa4, 0x03, 0xe3, 0x0a, 0x0c,
];
const KDF_ARGON2ID: [u8; 16] = [
    0x9e, 0x29, 0x8b, 0x19, 0x56, 0xdb, 0x47, 0x73, 0xb2, 0x3d, 0xfc, 0x3e, 0xc6, 0xf0, 0xa1, 0xe6,
];
const KDF_AES_KDBX4: [u8; 16] = [
    0x7c, 0x02, 0xbb, 0x82, 0x79, 0xa7, 0x4a, 0xc0, 0x92, 0x7d, 0x11, 0x4a, 0x00, 0x64, 0x82, 0x38,
];
const KDF_AES_KDBX3: [u8; 16] = [
    0xc9, 0xd9, 0xf3, 0x9a, 0x62, 0x8a, 0x44, 0x60, 0xbf, 0x74, 0x0d, 0x08, 0xc1, 0x8a, 0x4f, 0xea,
];

const MAX_MEMORY: u64 = 1 << 30; // 1 GiB
const MAX_ITERATIONS: u64 = 100;
const MAX_PARALLELISM: u32 = 16;
const MAX_AES_ROUNDS: u64 = 100_000_000;

#[derive(Debug, Clone, PartialEq)]
enum Kdf {
    Argon2 { id: bool, memory: u64, iterations: u64, parallelism: u32 },
    Aes { rounds: u64 },
}

#[derive(Debug)]
#[allow(dead_code)] // fields are read through Debug in the output
enum PreReadError {
    NotKdbx,
    Unsupported(String),
    Truncated,
    Malformed(String),
    Excessive(String),
}

struct Header {
    major: u16,
    minor: u16,
    /// Offset just after the END field: the header SHA-256 (KDBX 4) starts here.
    end: usize,
    kdfs: Vec<Kdf>,
    /// Absolute offset of the value bytes of each VariantDictionary item, by key (KDBX 4 only).
    vd_offsets: HashMap<String, usize>,
}

fn take(data: &[u8], pos: usize, len: usize) -> Result<&[u8], PreReadError> {
    let end = pos.checked_add(len).ok_or(PreReadError::Truncated)?;
    data.get(pos..end).ok_or(PreReadError::Truncated)
}

fn u16_at(data: &[u8], pos: usize) -> Result<u16, PreReadError> {
    let b = take(data, pos, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], pos: usize) -> Result<u32, PreReadError> {
    let b = take(data, pos, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_of(b: &[u8]) -> Result<u64, PreReadError> {
    let arr: [u8; 8] = b.try_into().map_err(|_| PreReadError::Malformed("u64 field is not 8 bytes".into()))?;
    Ok(u64::from_le_bytes(arr))
}

fn u32_of(b: &[u8]) -> Result<u32, PreReadError> {
    let arr: [u8; 4] = b.try_into().map_err(|_| PreReadError::Malformed("u32 field is not 4 bytes".into()))?;
    Ok(u32::from_le_bytes(arr))
}

fn pre_read(data: &[u8]) -> Result<Header, PreReadError> {
    if data.len() < 12 {
        return Err(PreReadError::Truncated);
    }
    if data[0..4] != [0x03, 0xd9, 0xa2, 0x9a] {
        return Err(PreReadError::NotKdbx);
    }
    let sig2 = u32_at(data, 4)?;
    let minor = u16_at(data, 8)?;
    let major = u16_at(data, 10)?;
    match (sig2, major) {
        (0xb54b_fb67, 3 | 4) => {}
        (0xb54b_fb65, _) => return Err(PreReadError::Unsupported("KDB (KeePass 1)".into())),
        (0xb54b_fb66, _) => return Err(PreReadError::Unsupported("KDBX 2 pre-release".into())),
        _ => return Err(PreReadError::Unsupported(format!("signature {sig2:#x}, version {major}.{minor}"))),
    }
    let len_size = if major == 4 { 4 } else { 2 };
    let mut pos = 12;
    let mut kdfs = Vec::new();
    let mut vd_offsets = HashMap::new();
    loop {
        let ty = *data.get(pos).ok_or(PreReadError::Truncated)?;
        let len = if len_size == 4 { u32_at(data, pos + 1)? as usize } else { usize::from(u16_at(data, pos + 1)?) };
        let value_pos = pos + 1 + len_size;
        let value = take(data, value_pos, len)?;
        pos = value_pos + len;
        match (major, ty) {
            (_, 0) => break,
            // Fixed-size fields the crate reads with byteorder (which panics on short input): validate the size.
            (_, 3) | (3, 10) if len != 4 => return Err(PreReadError::Malformed(format!("field {ty} len {len} != 4"))),
            (3, 6) => kdfs.push(Kdf::Aes { rounds: u64_of(value)? }),
            (4, 11) => kdfs.push(parse_kdf_dictionary(value, value_pos, &mut vd_offsets)?),
            // Public custom data is a VariantDictionary too, and the crate parses it with the same reader.
            (4, 12) => {
                walk_vd(value, value_pos, &mut HashMap::new())?;
            }
            _ => {}
        }
    }
    if kdfs.is_empty() {
        return Err(PreReadError::Malformed("no KDF parameters".into()));
    }
    // Every occurrence is checked: the crate keeps the last one if a field repeats.
    for kdf in &kdfs {
        check_limits(kdf)?;
    }
    Ok(Header { major, minor, end: pos, kdfs, vd_offsets })
}

fn parse_kdf_dictionary(
    vd: &[u8],
    base: usize,
    offsets: &mut HashMap<String, usize>,
) -> Result<Kdf, PreReadError> {
    let items = walk_vd(vd, base, offsets)?;
    let get = |k: &str| items.get(k).ok_or_else(|| PreReadError::Malformed(format!("KDF item {k} missing")));
    let uuid = get("$UUID")?;
    if uuid.as_slice() == KDF_ARGON2D || uuid.as_slice() == KDF_ARGON2ID {
        Ok(Kdf::Argon2 {
            id: uuid.as_slice() == KDF_ARGON2ID,
            memory: u64_of(get("M")?)?,
            iterations: u64_of(get("I")?)?,
            parallelism: u32_of(get("P")?)?,
        })
    } else if uuid.as_slice() == KDF_AES_KDBX4 || uuid.as_slice() == KDF_AES_KDBX3 {
        Ok(Kdf::Aes { rounds: u64_of(get("R")?)? })
    } else {
        Err(PreReadError::Unsupported("unknown KDF UUID".into()))
    }
}

/// Walks a VariantDictionary and checks the size of every typed value: the crate reads them with byteorder,
/// which panics when the buffer is shorter than the type.
fn walk_vd(
    vd: &[u8],
    base: usize,
    offsets: &mut HashMap<String, usize>,
) -> Result<HashMap<String, Vec<u8>>, PreReadError> {
    let version = u16_at(vd, 0)?;
    if version & 0xff00 != 0x0100 {
        return Err(PreReadError::Malformed(format!("VariantDictionary version {version:#x}")));
    }
    let mut pos = 2;
    let mut items: HashMap<String, Vec<u8>> = HashMap::new();
    loop {
        let ty = *vd.get(pos).ok_or(PreReadError::Truncated)?;
        if ty == 0 {
            break;
        }
        let klen = u32_at(vd, pos + 1)? as usize;
        let k = take(vd, pos + 5, klen)?;
        let vlen_pos = pos + 5 + klen;
        let vlen = u32_at(vd, vlen_pos)? as usize;
        let v = take(vd, vlen_pos + 4, vlen)?;
        let expected = match ty {
            0x04 | 0x0c => Some(4),
            0x05 | 0x0d => Some(8),
            0x08 => Some(1),
            0x18 | 0x42 => None,
            _ => return Err(PreReadError::Malformed(format!("VariantDictionary type {ty:#x}"))),
        };
        if expected.is_some_and(|n| n != vlen) {
            return Err(PreReadError::Malformed(format!("VariantDictionary item of type {ty:#x} with {vlen} bytes")));
        }
        let name = String::from_utf8_lossy(k).into_owned();
        offsets.insert(name.clone(), base + vlen_pos + 4);
        items.insert(name, v.to_vec());
        pos = vlen_pos + 4 + vlen;
    }
    Ok(items)
}

fn check_limits(kdf: &Kdf) -> Result<(), PreReadError> {
    match *kdf {
        Kdf::Argon2 { memory, iterations, parallelism, .. } => {
            if memory > MAX_MEMORY || iterations > MAX_ITERATIONS || parallelism > MAX_PARALLELISM {
                return Err(PreReadError::Excessive(format!("M={memory} I={iterations} P={parallelism}")));
            }
        }
        Kdf::Aes { rounds } => {
            if rounds > MAX_AES_ROUNDS {
                return Err(PreReadError::Excessive(format!("R={rounds}")));
            }
        }
    }
    Ok(())
}

/// Overwrites a u64 KDF item and recomputes the KDBX 4 header SHA-256, as an attacker could.
fn tamper_u64(data: &mut [u8], item: &str, value: u64) {
    let header = pre_read(data).expect("pre-read");
    let off = header.vd_offsets[item];
    data[off..off + 8].copy_from_slice(&value.to_le_bytes());
    let hash = Sha256::digest(&data[..header.end]);
    data[header.end..header.end + 32].copy_from_slice(&hash);
}

// ---------------------------------------------------------------------------------------------------------
// Mapping crate errors to the spec's codes (RF-11, RF-13). The enums are #[non_exhaustive]: the wildcard is
// mandatory and must land on ArchivoDanado.
// ---------------------------------------------------------------------------------------------------------

fn code(err: &DatabaseOpenError) -> &'static str {
    code_for(None, err)
}

/// KDBX 3.1 has no header HMAC: a wrong key shows up as a padding error of the last block, the same error a
/// truncated or damaged file gives. Those are mapped to LlaveIncorrecta (the spec cannot tell them apart).
fn code_for(version: Option<&DatabaseVersion>, err: &DatabaseOpenError) -> &'static str {
    if let (Some(DatabaseVersion::KDB3(_)), DatabaseOpenError::Cryptography(_)) = (version, err) {
        return "LlaveIncorrecta";
    }
    match err {
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey) => "LlaveIncorrecta",
        DatabaseOpenError::Key(DatabaseKeyError::EmptyKey) => "LlaveVacia",
        DatabaseOpenError::UnsupportedVersion => "FormatoNoSoportado",
        DatabaseOpenError::VersionParse(DatabaseVersionParseError::InvalidKDBXVersion { .. }) => "FormatoNoSoportado",
        _ => "ArchivoDanado",
    }
}

fn short(s: String) -> String {
    if s.chars().count() > 110 { format!("{}...", s.chars().take(110).collect::<String>()) } else { s }
}

fn quiet_panics() {
    // The default hook prints the source location (a path under the user's profile): print only the message.
    panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        eprintln!("      (panic: {})", short(msg));
    }));
}

fn try_open(label: &str, data: &[u8], key: DatabaseKey) {
    let pre = match pre_read(data) {
        Ok(h) => format!("pre-lector OK v{}.{}", h.major, h.minor),
        Err(e) => format!("pre-lector {e:?}"),
    };
    let start = Instant::now();
    let result = panic::catch_unwind(AssertUnwindSafe(|| Database::parse(data, key)));
    let ms = start.elapsed().as_millis();
    let outcome = match result {
        Ok(Ok(db)) => format!("ABRE ({})", db.config.version),
        Ok(Err(e)) => {
            let version = Database::get_version(&mut &data[..]).ok();
            format!("{} <- {}", code_for(version.as_ref(), &e), short(format!("{e:?}")))
        }
        Err(_) => "PANIC (el proceso caeria sin catch_unwind)".to_owned(),
    };
    println!("  {label:<44} {ms:>5} ms | {outcome}");
    println!("  {:<44}          | {}", "", short(pre));
}

fn part_a_and_f() {
    println!("== P4 (e) version sin derivar la llave: Database::get_version ==");
    for rel in ["keepass2/f-pass.kdbx", "keepassxc/xc-both4.kdbx"] {
        let v = Database::get_version(&mut read_fixture(rel).as_slice()).expect("version");
        println!("  {rel:<30} -> {v:?} ({v})");
    }
    let v3 = fs::read(kdbx3_path()).ok();
    match &v3 {
        Some(b) => println!("  {:<30} -> {:?}", "%TEMP%/v3-aes.kdbx", Database::get_version(&mut b.as_slice())),
        None => println!("  v3-aes.kdbx no existe: correr generar-kdbx3.ps1"),
    }

    println!("\n== P4 (a) parametros KDF de la cabecera ANTES de derivar (pre-lector propio) ==");
    for rel in [
        "keepass2/f-pass.kdbx",
        "keepass2/f-key.kdbx",
        "keepass2/f-both.kdbx",
        "keepass2/f-unicode.kdbx",
        "keepassxc/xc-both4.kdbx",
    ] {
        let data = read_fixture(rel);
        let t = Instant::now();
        let h = pre_read(&data).expect("pre-read fixture");
        println!("  {rel:<26} v{}.{} {:?} en {} us", h.major, h.minor, h.kdfs, t.elapsed().as_micros());
    }
    if let Some(b) = &v3 {
        let h = pre_read(b).expect("pre-read v3");
        println!("  {:<26} v{}.{} {:?}", "%TEMP%/v3-aes.kdbx", h.major, h.minor, h.kdfs);
    }

    let base = read_fixture("keepass2/f-pass.kdbx");
    println!("\n  Cabecera manipulada (SHA-256 de cabecera recalculado, como haria un atacante):");
    let mut it4 = base.clone();
    tamper_u64(&mut it4, "I", 4);
    try_open("I=4 (era 2): el crate deriva y luego falla", &it4, key(Some(PASSWORD), false));
    let mut huge = base.clone();
    tamper_u64(&mut huge, "M", 64 << 30);
    let t = Instant::now();
    let r = pre_read(&huge);
    println!(
        "  M=64 GiB: pre-lector -> {:?} en {} us (el crate NO se llama: intentaria reservar 64 GiB)",
        r.err(),
        t.elapsed().as_micros()
    );
    let mut iters = base.clone();
    tamper_u64(&mut iters, "I", 1_000_000);
    println!("  I=1e6: pre-lector -> {:?}", pre_read(&iters).err());

    println!("\n== P4 (f) que error devuelve el crate en cada caso (y el codigo de la spec que le damos) ==");
    let header_end = pre_read(&base).expect("pre-read").end;
    let pass = || key(Some(PASSWORD), false);
    try_open("0 bytes", &[], pass());
    try_open("texto plano", b"hola, esto no es una boveda\r\n", pass());
    try_open("truncado en la cabecera (100 B)", &base[..100], pass());
    try_open("truncado entre hash y HMAC", &base[..header_end + 40], pass());
    try_open("truncado en el flujo de bloques", &base[..base.len() - 100], pass());
    let mut flip = base.clone();
    let i = flip.len() - 200;
    flip[i] ^= 0x01;
    try_open("1 bit cambiado en el contenido cifrado", &flip, pass());
    let mut nohash = base.clone();
    nohash[header_end - 10] ^= 0x01;
    try_open("1 bit cambiado en la cabecera (sin rehash)", &nohash, pass());
    let mut salt = base.clone();
    let off = pre_read(&base).expect("pre-read").vd_offsets["S"];
    salt[off] ^= 0x01;
    try_open("1 bit cambiado en la sal del KDF (sin rehash)", &salt, pass());
    try_open("contrasena incorrecta", &base, key(Some("otra"), false));
    try_open("contrasena buena + archivo llave sobrante", &base, key(Some(PASSWORD), true));
    try_open("llave sin componentes", &base, DatabaseKey::new());
    let both = read_fixture("keepass2/f-both.kdbx");
    try_open("f-both: contrasena buena sin archivo llave", &both, pass());
    try_open("f-both: archivo llave sin contrasena", &both, key(None, true));
    try_open("f-both: llave completa", &both, key(Some(PASSWORD), true));
    let mut v5 = base.clone();
    v5[10..12].copy_from_slice(&5u16.to_le_bytes());
    try_open("version mayor 5", &v5, pass());
    let mut kdb1 = vec![0x03, 0xd9, 0xa2, 0x9a, 0x65, 0xfb, 0x4b, 0xb5];
    kdb1.extend(std::iter::repeat_n(0u8, 200));
    try_open("firma KDB (KeePass 1) con basura", &kdb1, pass());
    // Compression field of 1 byte instead of 4: the crate calls LittleEndian::read_u32 on it.
    let mut crafted = base[..12].to_vec();
    crafted.extend([3u8, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    try_open("cabecera con campo de compresion de 1 byte", &crafted, pass());
    // Public custom data (field 12) holding a UInt32 item of 1 byte.
    let mut crafted_vd = base[..12].to_vec();
    let vd: [u8; 15] = [0x00, 0x01, 0x04, 1, 0, 0, 0, b'x', 1, 0, 0, 0, 0x07, 0x00, 0x00];
    crafted_vd.push(12);
    crafted_vd.extend((vd.len() as u32).to_le_bytes());
    crafted_vd.extend(vd);
    crafted_vd.extend([0u8, 0, 0, 0, 0]);
    try_open("datos publicos con un UInt32 de 1 byte", &crafted_vd, pass());
    if let Some(b) = &v3 {
        try_open("KDBX 3.1: llave correcta", b, pass());
        try_open("KDBX 3.1: contrasena incorrecta", b, key(Some("otra"), false));
        let mut f3 = b.clone();
        let j = f3.len() - 40;
        f3[j] ^= 0x01;
        try_open("KDBX 3.1: 1 bit cambiado al final", &f3, pass());
        try_open("KDBX 3.1: truncado", &b[..b.len() - 50], pass());
        let mut crafted3 = b[..12].to_vec();
        crafted3.extend([6u8, 1, 0, 9, 0, 0, 0]); // TRANSFORMROUNDS with 1 byte
        try_open("KDBX 3.1: campo de rondas de 1 byte", &crafted3, pass());
    }
}

// ---------------------------------------------------------------------------------------------------------
// (b) creation with our own Argon2id parameters and calibration
// ---------------------------------------------------------------------------------------------------------

/// Builds `KdfConfig::Argon2id` through the public `VariantDictionary` route, so the core does not need
/// `rust-argon2` as a direct dependency just to name `argon2::Version`.
fn argon2id(memory: u64, iterations: u64, parallelism: u32) -> Result<KdfConfig, KdfConfigError> {
    let mut vd = VariantDictionary::new();
    vd.set("$UUID", KDF_ARGON2ID.to_vec());
    vd.set("M", memory);
    vd.set("I", iterations);
    vd.set("P", parallelism);
    vd.set("V", 0x13u32);
    vd.set("S", vec![0u8; 32]); // ignored: save() draws a fresh seed
    let (kdf, _salt): (KdfConfig, Vec<u8>) = vd.try_into()?;
    Ok(kdf)
}

fn config(kdf: KdfConfig) -> DatabaseConfig {
    // `DatabaseConfig` is #[non_exhaustive]: start from default() and overwrite EVERY public field.
    let mut c = DatabaseConfig::default();
    c.version = DatabaseVersion::KDB4(1);
    c.outer_cipher_config = OuterCipherConfig::ChaCha20;
    c.compression_config = CompressionConfig::GZip;
    c.inner_cipher_config = InnerCipherConfig::ChaCha20;
    c.kdf_config = kdf;
    c.public_custom_data = None;
    c
}

fn time_unlock(memory: u64, iterations: u64, parallelism: u32) -> (Duration, Vec<u8>) {
    let db = Database::with_config(config(argon2id(memory, iterations, parallelism).expect("kdf")));
    let mut buf = Vec::new();
    db.save(&mut buf, key(Some(PASSWORD), false)).expect("save");
    let t = Instant::now();
    Database::parse(&buf, key(Some(PASSWORD), false)).expect("reopen");
    (t.elapsed(), buf)
}

fn part_b() {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    println!("\n== P4 (b) crear con Argon2id propio y calibrar (perfil {profile}, {cpus} hilos logicos) ==");
    let default = DatabaseConfig::default().kdf_config;
    println!("  DatabaseConfig::default().kdf_config = {default:?}");
    let memory = 64 << 20;
    let parallelism = 2u32;
    let mut iterations = 2u64;
    let mut last;
    loop {
        let (d, _) = time_unlock(memory, iterations, parallelism);
        last = d;
        println!("  M=64 MiB P={parallelism} I={iterations:<3} -> desbloqueo {} ms", d.as_millis());
        if d >= Duration::from_millis(500) || iterations >= 64 {
            break;
        }
        iterations *= 2;
    }
    if last < Duration::from_millis(900) && last.as_millis() > 0 {
        let scaled = (iterations as u128 * 1000 / last.as_millis()).max(2) as u64;
        let (d, buf) = time_unlock(memory, scaled, parallelism);
        println!("  calibrado a ~1 s: I={scaled} -> {} ms", d.as_millis());
        let h = pre_read(&buf).expect("pre-read new");
        let reopened = Database::parse(&buf, key(Some(PASSWORD), false)).expect("reopen");
        println!("  cabecera escrita: v{}.{} {:?}", h.major, h.minor, h.kdfs);
        println!("  config al reabrir: {:?}", reopened.config.kdf_config);
        let wrong = Database::parse(&buf, key(Some("otra"), false)).map(|_| ()).map_err(|e| code(&e));
        println!("  otra llave -> {wrong:?}");
    }
    let (d, _) = time_unlock(256 << 20, 2, 4);
    println!("  referencia M=256 MiB P=4 I=2 -> {} ms", d.as_millis());
}

// ---------------------------------------------------------------------------------------------------------
// (c) recycle bin and deleted objects, (d) moving entries and groups
// ---------------------------------------------------------------------------------------------------------

fn roundtrip(db: &Database) -> Database {
    let mut buf = Vec::new();
    db.save(&mut buf, key(Some(PASSWORD), false)).expect("save");
    Database::parse(&buf, key(Some(PASSWORD), false)).expect("reopen")
}

fn ensure_recycle_bin(db: &mut Database) -> (GroupId, bool) {
    if db.meta.recyclebin_enabled != Some(false) {
        if let Some(bin) = db.recycle_bin() {
            return (bin.id(), false);
        }
    }
    let id = {
        let mut root = db.root_mut();
        let mut bin = root.add_group();
        bin.name = "Papelera".into();
        bin.set_icon_builtin(43); // KeePass' recycle bin icon
        bin.enable_autotype = Some(false);
        bin.enable_searching = Some(false);
        bin.id()
    };
    db.meta.recyclebin_uuid = Some(id.uuid());
    db.meta.recyclebin_enabled = Some(true);
    db.meta.recyclebin_changed = Some(Times::now());
    (id, true)
}

fn part_c_and_d() {
    println!("\n== P4 (c)(d) papelera, objetos borrados y mover ==");
    let mut db = Database::parse(&read_fixture("keepass2/f-pass.kdbx"), key(Some(PASSWORD), false)).expect("open");
    println!(
        "  fixture f-pass: RecycleBinEnabled={:?} RecycleBinUUID={:?} recycle_bin()={} DeletedObjects={}",
        db.meta.recyclebin_enabled,
        db.meta.recyclebin_uuid,
        db.recycle_bin().map(|g| g.name.clone()).unwrap_or_else(|| "None".into()),
        db.deleted_objects.len()
    );
    let work_id = db.root().group_by_name("Trabajo").expect("Trabajo").id();
    let sub_id = db.root().group_by_name("Trabajo").and_then(|g| g.group_by_name("Subgrupo").map(|s| s.id())).expect("Sub");
    let (entry_id, before) = {
        let e = db.root().group_by_name("Trabajo").and_then(|g| g.entry_by_name("GitHub").map(|e| (e.id(), e.times.clone()))).expect("GitHub");
        e
    };
    let history_before = db.entry(entry_id).and_then(|e| e.history.as_ref().map(|h| h.get_entries().len()));

    let (bin_id, created) = ensure_recycle_bin(&mut db);
    println!("  papelera creada por nosotros: {created}");
    // EntryTrack (track_changes) pushes a history snapshot when dropped, so moving with it adds a version.
    // KeePass only updates LocationChanged on a move: EntryMut::move_to + the timestamp by hand.
    let mut probe = db.clone();
    probe.entry_mut(entry_id).expect("entry").track_changes().move_to(bin_id).expect("move");
    println!(
        "  EntryTrack::move_to agrega historial: {:?} -> {:?}",
        history_before,
        probe.entry(entry_id).and_then(|e| e.history.as_ref().map(|h| h.get_entries().len()))
    );
    {
        let mut entry = db.entry_mut(entry_id).expect("entry");
        entry.move_to(bin_id).expect("move to bin");
        entry.times.location_changed = Some(Times::now());
    }
    let db2 = roundtrip(&db);
    let e = db2.entry(entry_id).expect("same UUID after save+reopen");
    println!("  RF-47 mover a papelera -> guardar -> reabrir:");
    println!("    misma UUID: {} | padre = papelera: {} | grupo anterior = Trabajo: {}",
        e.id() == entry_id,
        e.parent().id() == bin_id,
        e.previous_parent().map(|g| g.id()) == Some(work_id));
    println!("    creation igual: {} | last_modification igual: {} | location_changed {:?} -> {:?}",
        e.times.creation == before.creation,
        e.times.last_modification == before.last_modification,
        before.location_changed, e.times.location_changed);
    println!("    historial: {:?} -> {:?} | contrasena intacta: {}", history_before,
        e.history.as_ref().map(|h| h.get_entries().len()), e.get_password() == Some("tercera-clave"));
    println!("    meta al reabrir: RecycleBinEnabled={:?} RecycleBinUUID==papelera: {}",
        db2.meta.recyclebin_enabled, db2.meta.recyclebin_uuid == Some(bin_id.uuid()));

    // (d) groups
    let cycle = db.group_mut(work_id).expect("work").track_changes().move_to(sub_id);
    println!("  RF-55 mover Trabajo dentro de su hijo Subgrupo -> {:?}", cycle.err());
    let root_id = db.root().id();
    let root_move = db.root_mut().move_to(sub_id);
    println!("  RF-57 mover la raiz -> {:?}", root_move.err());
    let sub_before = db.group(sub_id).expect("sub").times.clone();
    db.group_mut(sub_id).expect("sub").track_changes().move_to(root_id).expect("move sub");
    let db3 = roundtrip(&db);
    let s = db3.group(sub_id).expect("sub after reopen");
    println!("  RF-54 mover Subgrupo a la raiz: misma UUID {} | padre raiz {} | creation igual {} | location_changed cambio {}",
        s.id() == sub_id,
        s.parent().map(|p| p.id()) == Some(root_id),
        s.times.creation == sub_before.creation,
        s.times.location_changed != sub_before.location_changed);

    // RF-48: definitive delete of an entry already in the bin
    let deleted_before = db.deleted_objects.len();
    db.entry_mut(entry_id).expect("entry").track_changes().remove();
    let db4 = roundtrip(&db);
    println!("  RF-48 EntryTrack::remove en la papelera: DeletedObjects {} -> {} (tras reabrir: {}, contiene UUID: {})",
        deleted_before, db.deleted_objects.len(), db4.deleted_objects.len(),
        db4.deleted_objects.contains_key(&entry_id.uuid()));

    // EntryMut::remove (without track_changes) does not record it
    let other = db.iter_all_entries().find(|e| e.parent().id() != bin_id).map(|e| e.id()).expect("other entry");
    let n = db.deleted_objects.len();
    db.entry_mut(other).expect("other").remove();
    println!("  EntryMut::remove (sin track_changes): DeletedObjects {} -> {} (no lo anota)", n, db.deleted_objects.len());

    // RF-56 + RF-58: a group with content to the bin, then empty the bin (fresh copy of the fixture)
    let mut db = Database::parse(&read_fixture("keepass2/f-pass.kdbx"), key(Some(PASSWORD), false)).expect("open");
    let (bin_id, _) = ensure_recycle_bin(&mut db);
    let subgroups_in_work = db.group(work_id).map(|g| g.group_ids().count()).unwrap_or(0);
    let entries_in_work = db.group(work_id).map(|g| count_entries(&db, g.id())).unwrap_or(0);
    db.group_mut(work_id).expect("work").track_changes().move_to(bin_id).expect("work to bin");
    let in_bin = count_entries(&db, bin_id);
    println!(
        "  RF-56 Trabajo ({entries_in_work} entradas, {subgroups_in_work} subgrupos) a la papelera: la papelera tiene {in_bin} entradas"
    );
    let groups_before = db.num_groups();
    let n = db.deleted_objects.len();
    let children: Vec<GroupId> = db.group(bin_id).expect("bin").group_ids().collect();
    let entries: Vec<_> = db.group(bin_id).expect("bin").entry_ids().collect();
    for g in children {
        db.group_mut(g).expect("g").track_changes().remove().expect("remove group");
    }
    for e in entries {
        db.entry_mut(e).expect("e").track_changes().remove();
    }
    let db5 = roundtrip(&db);
    println!("  RF-58 vaciar papelera: DeletedObjects {} -> {} | grupos {} -> {} | papelera sigue: {} | entradas en papelera {}",
        n, db5.deleted_objects.len(), groups_before, db5.num_groups(), db5.recycle_bin().is_some(), count_entries(&db5, bin_id));
}

fn count_entries(db: &Database, group: GroupId) -> usize {
    let g = db.group(group).expect("group");
    g.entries().count() + g.group_ids().map(|c| count_entries(db, c)).sum::<usize>()
}

// ---------------------------------------------------------------------------------------------------------
// (e) KDBX 3.1: open, refuse to save, convert to a new KDBX 4 (RF-14, RF-15)
// ---------------------------------------------------------------------------------------------------------

fn part_e() {
    println!("\n== P4 (e) KDBX 3.1: abrir, guardar y convertir ==");
    let Ok(bytes) = fs::read(kdbx3_path()) else {
        println!("  sin v3-aes.kdbx: correr generar-kdbx3.ps1");
        return;
    };
    let db = Database::parse(&bytes, key(Some(PASSWORD), false)).expect("open v3");
    println!("  config.version = {:?} kdf = {:?}", db.config.version, db.config.kdf_config);
    let e = db.root().entries().next().map(|e| (e.get_title().map(str::to_owned), e.get(fields::USERNAME).map(str::to_owned)));
    println!("  primera entrada: {e:?}");
    let mut sink = Vec::new();
    println!("  save() del 3.1 tal cual -> {:?}", db.save(&mut sink, key(Some(PASSWORD), false)).err());
    let mut converted = db.clone();
    converted.config = config(argon2id(64 << 20, 2, 2).expect("kdf"));
    let mut out = Vec::new();
    converted.save(&mut out, key(Some(PASSWORD), false)).expect("save converted");
    let back = Database::parse(&out, key(Some(PASSWORD), false)).expect("reopen converted");
    println!("  convertido: {} con {:?}, misma raiz: {}, entradas {} -> {}",
        back.config.version, back.config.kdf_config, back.root().id() == db.root().id(),
        db.num_entries(), back.num_entries());
    let after = fs::read(kdbx3_path()).expect("reread");
    println!("  original identico byte a byte: {}", after == bytes);
}

fn main() {
    quiet_panics();
    part_a_and_f();
    part_b();
    part_c_and_d();
    part_e();
}
