//! Module tests of `kdbx` that need the library or forge headers (RF-10, RF-11, RF-13, RF-14, RF-16, RF-17,
//! RF-94). Expected values come from the spec and from the lab table (`docs/lab/boveda-0.1/EXPERIMENTO.md`, P4).
//! Fixtures hold fake data only and are never modified: they are read into memory.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this module).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs, panic,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use keepass::{
    Database, DatabaseKey,
    config::{
        CompressionConfig, DatabaseConfig, DatabaseVersion, InnerCipherConfig, KdfConfig,
        OuterCipherConfig, VariantDictionary,
    },
};
use sha2::{Digest, Sha256};

use super::{
    Argon2Variant, FormatVersion, KdfParams, KeyFile, MasterPassword, VaultKey,
    header::{self, KDF_ARGON2D, KDF_ARGON2ID},
    open::{self, parse_guarded},
    read_header,
};
use crate::error::CoreError;

const PASSWORD: &str = "lab-Maestra-2026";
const KDBX4_1: FormatVersion = FormatVersion { major: 4, minor: 1 };

fn fixture(rel: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel);
    fs::read(path).expect("read fixture")
}

fn password_key(password: &str) -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(password.to_owned())), None).unwrap()
}

fn both_key() -> VaultKey {
    let file = KeyFile::from_bytes(fixture("keepass2/llave.keyx")).unwrap();
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), Some(file)).unwrap()
}

fn open_err(bytes: &[u8]) -> CoreError {
    open::open(bytes, &password_key(PASSWORD)).unwrap_err()
}

/// Overwrites a u64 KDF item and recomputes the KDBX 4 header SHA-256, as an attacker could.
fn tamper_u64(data: &mut [u8], item: &str, value: u64) {
    let parsed = header::parse(data).expect("pre-read before tampering");
    let off = parsed.kdf_item_offsets[item];
    data[off..off + 8].copy_from_slice(&value.to_le_bytes());
    rehash(data, parsed.end);
}

fn tamper_u32(data: &mut [u8], item: &str, value: u32) {
    let parsed = header::parse(data).expect("pre-read before tampering");
    let off = parsed.kdf_item_offsets[item];
    data[off..off + 4].copy_from_slice(&value.to_le_bytes());
    rehash(data, parsed.end);
}

fn rehash(data: &mut [u8], end: usize) {
    let hash = Sha256::digest(&data[..end]);
    data[end..end + 32].copy_from_slice(&hash);
}

fn argon2(uuid: [u8; 16], memory: u64, iterations: u64, parallelism: u32) -> KdfConfig {
    let mut vd = VariantDictionary::new();
    vd.set("$UUID", uuid.to_vec());
    vd.set("M", memory);
    vd.set("I", iterations);
    vd.set("P", parallelism);
    vd.set("V", 0x13u32);
    vd.set("S", vec![0u8; 32]);
    let (kdf, _salt): (KdfConfig, Vec<u8>) = vd.try_into().expect("argon2 kdf");
    kdf
}

/// A KDBX 4.1 vault with the given KDF, saved by the library with [`PASSWORD`].
fn vault_with(kdf: KdfConfig) -> Vec<u8> {
    let mut config = DatabaseConfig::default();
    config.version = DatabaseVersion::KDB4(1);
    config.outer_cipher_config = OuterCipherConfig::ChaCha20;
    config.compression_config = CompressionConfig::GZip;
    config.inner_cipher_config = InnerCipherConfig::ChaCha20;
    config.kdf_config = kdf;
    config.public_custom_data = None;
    let mut out = Vec::new();
    Database::with_config(config)
        .save(&mut out, DatabaseKey::new().with_password(PASSWORD))
        .expect("save");
    out
}

/// Offset of the value of the first KDBX 3 header field of this type.
fn kdbx3_field_offset(data: &[u8], wanted: u8) -> usize {
    let mut pos = 12;
    loop {
        let ty = data[pos];
        let len = usize::from(u16::from_le_bytes([data[pos + 1], data[pos + 2]]));
        if ty == wanted {
            return pos + 3;
        }
        assert_ne!(ty, 0, "field {wanted} not found");
        pos += 3 + len;
    }
}

/// The three headers that made `keepass` 0.17.0 panic in the lab (P4 f).
fn panicking_headers() -> [(&'static str, Vec<u8>); 3] {
    let base = fixture("keepass2/f-pass.kdbx");
    let mut compression = base[..12].to_vec();
    compression.extend([3u8, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    let mut custom_data = base[..12].to_vec();
    let vd: [u8; 15] = [
        0x00, 0x01, 0x04, 1, 0, 0, 0, b'x', 1, 0, 0, 0, 0x07, 0x00, 0x00,
    ];
    custom_data.push(12);
    custom_data.extend(u32::try_from(vd.len()).unwrap().to_le_bytes());
    custom_data.extend(vd);
    custom_data.extend([0u8, 0, 0, 0, 0]);
    let v3 = fixture("keepass2/v3-aes.kdbx");
    let mut rounds = v3[..12].to_vec();
    rounds.extend([6u8, 1, 0, 9, 0, 0, 0]);
    [
        ("compression field of 1 byte", compression),
        ("public custom data with a UInt32 of 1 byte", custom_data),
        ("KDBX 3.1 transform rounds of 1 byte", rounds),
    ]
}

#[test]
fn rf_16_header_of_every_fixture_gives_the_lab_values() {
    let lab = KdfParams::Argon2 {
        variant: Argon2Variant::Id,
        memory_bytes: 67_108_864,
        iterations: 2,
        parallelism: 2,
    };
    for rel in [
        "keepass2/f-pass.kdbx",
        "keepass2/f-key.kdbx",
        "keepass2/f-both.kdbx",
        "keepass2/f-unicode.kdbx",
        "keepassxc/xc-both4.kdbx",
    ] {
        let info = read_header(&fixture(rel)).expect(rel);
        assert_eq!(info.version, KDBX4_1, "{rel}");
        assert_eq!(info.kdf, lab, "{rel}");
    }
    let v3 = read_header(&fixture("keepass2/v3-aes.kdbx")).unwrap();
    assert_eq!(v3.version, FormatVersion { major: 3, minor: 1 });
    assert_eq!(v3.kdf, KdfParams::AesKdf { rounds: 60_000 });
}

#[test]
fn rf_10_opens_keepass2_and_keepassxc_vaults_with_their_content() {
    let opened = open::open(&fixture("keepass2/f-pass.kdbx"), &password_key(PASSWORD)).unwrap();
    assert_eq!(opened.version(), KDBX4_1);
    let github = opened
        .db
        .root()
        .group_by_name("Trabajo")
        .and_then(|g| {
            g.entry_by_name("GitHub").map(|e| {
                (
                    e.get_username().map(str::to_owned),
                    e.get_password().map(str::to_owned),
                )
            })
        })
        .expect("Trabajo/GitHub");
    assert_eq!(
        github,
        (
            Some("sara@example.com".to_owned()),
            Some("tercera-clave".to_owned())
        )
    );

    let xc = open::open(&fixture("keepassxc/xc-both4.kdbx"), &both_key()).unwrap();
    assert_eq!(xc.version(), KDBX4_1);
    assert!(xc.entry_count() > 0);
}

#[test]
fn rf_13_not_kdbx_truncated_or_altered_is_archivo_danado() {
    let base = fixture("keepass2/f-pass.kdbx");
    let end = header::parse(&base).unwrap().end;
    let mut content_bit = base.clone();
    let i = content_bit.len() - 200;
    content_bit[i] ^= 0x01;
    let mut header_bit = base.clone();
    header_bit[end - 10] ^= 0x01;
    let mut salt_bit = base.clone();
    let salt = header::parse(&base).unwrap().kdf_item_offsets["S"];
    salt_bit[salt] ^= 0x01;
    let cases: [(&str, &[u8]); 8] = [
        ("0 bytes", &[]),
        ("plain text", b"hola, esto no es una boveda\r\n"),
        ("truncated in the header (100 B)", &base[..100]),
        ("truncated between hash and HMAC", &base[..end + 40]),
        ("truncated in the block stream", &base[..base.len() - 100]),
        ("1 bit changed in the encrypted content", &content_bit),
        ("1 bit changed in the header, no rehash", &header_bit),
        ("1 bit changed in the KDF salt, no rehash", &salt_bit),
    ];
    for (label, bytes) in cases {
        assert_eq!(open_err(bytes), CoreError::ArchivoDanado, "{label}");
    }
}

#[test]
fn rf_13_kdb_kdbx2_and_kdbx5_are_formato_no_soportado() {
    let base = fixture("keepass2/f-pass.kdbx");
    let mut v5 = base.clone();
    v5[10..12].copy_from_slice(&5u16.to_le_bytes());
    let mut v2 = base.clone();
    v2[10..12].copy_from_slice(&2u16.to_le_bytes());
    let mut kdb1 = vec![0x03, 0xd9, 0xa2, 0x9a, 0x65, 0xfb, 0x4b, 0xb5];
    kdb1.extend(std::iter::repeat_n(0u8, 200));
    let mut prerelease = base.clone();
    prerelease[4..8].copy_from_slice(&0xb54b_fb66u32.to_le_bytes());
    for (label, bytes) in [
        ("KDBX 5.1", v5),
        ("KDBX 2.1", v2),
        ("KDB (KeePass 1)", kdb1),
        ("KDBX 2 pre-release", prerelease),
    ] {
        assert_eq!(open_err(&bytes), CoreError::FormatoNoSoportado, "{label}");
    }
    let mut other_signature = base;
    other_signature[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    assert_eq!(open_err(&other_signature), CoreError::ArchivoDanado);
}

#[test]
fn rf_16_kdf_limits_are_checked_before_deriving_even_with_a_valid_header_hash() {
    let base = fixture("keepass2/f-pass.kdbx");
    let cases: [(&str, u64); 5] = [
        ("M", 64 << 30),               // 64 GiB: the library would try to reserve it
        ("M", (1 << 30) + 1),          // 1 GiB + 1 byte
        ("M", (1 << 42) + (64 << 20)), // 4 TiB + 64 MiB: `(M / 1024) as u32` would read 64 MiB
        ("I", 101),
        ("I", (1 << 32) + 2), // `as u32` would read 2
    ];
    for (item, value) in cases {
        let mut bytes = base.clone();
        tamper_u64(&mut bytes, item, value);
        assert_eq!(
            open_err(&bytes),
            CoreError::ParametrosKdfExcesivos,
            "{item}={value}"
        );
    }
    let mut lanes = base.clone();
    tamper_u32(&mut lanes, "P", 17);
    assert_eq!(open_err(&lanes), CoreError::ParametrosKdfExcesivos, "P=17");

    // The limits themselves are allowed (read the header only: deriving 100 passes takes seconds).
    for (item, value) in [("M", 1u64 << 30), ("I", 100)] {
        let mut bytes = base.clone();
        tamper_u64(&mut bytes, item, value);
        assert!(
            read_header(&bytes).is_ok(),
            "{item}={value} is within the limit"
        );
    }
    let mut lanes = base;
    tamper_u32(&mut lanes, "P", 16);
    assert!(read_header(&lanes).is_ok(), "P=16 is within the limit");
}

#[test]
fn rf_16_every_occurrence_of_the_kdf_field_is_checked() {
    // The library keeps the LAST KDF field: a hostile first occurrence must not slip through.
    let base = fixture("keepass2/f-pass.kdbx");
    let mut hostile = base.clone();
    tamper_u64(&mut hostile, "M", 64 << 30);
    let field = |data: &[u8]| -> Vec<u8> {
        let mut pos = 12;
        loop {
            let len = usize::try_from(u32::from_le_bytes(
                data[pos + 1..pos + 5].try_into().unwrap(),
            ))
            .unwrap();
            if data[pos] == 11 {
                return data[pos..pos + 5 + len].to_vec();
            }
            pos += 5 + len;
        }
    };
    let mut doubled = base[..12].to_vec();
    doubled.extend(field(&hostile));
    doubled.extend(&base[12..]);
    assert_eq!(
        read_header(&doubled).unwrap_err(),
        CoreError::ParametrosKdfExcesivos
    );
}

#[test]
fn rf_16_aes_kdf_rounds_limit_in_kdbx3_and_kdbx4() {
    let v3 = fixture("keepass2/v3-aes.kdbx");
    let off = kdbx3_field_offset(&v3, 6);
    let mut over = v3.clone();
    over[off..off + 8].copy_from_slice(&100_000_001u64.to_le_bytes());
    assert_eq!(
        read_header(&over).unwrap_err(),
        CoreError::ParametrosKdfExcesivos
    );
    let mut limit = v3;
    limit[off..off + 8].copy_from_slice(&100_000_000u64.to_le_bytes());
    assert_eq!(
        read_header(&limit).unwrap().kdf,
        KdfParams::AesKdf {
            rounds: 100_000_000
        }
    );

    let mut v4 = vault_with(KdfConfig::Aes { rounds: 1_000 });
    assert_eq!(
        read_header(&v4).unwrap().kdf,
        KdfParams::AesKdf { rounds: 1_000 }
    );
    tamper_u64(&mut v4, "R", 100_000_001);
    assert_eq!(open_err(&v4), CoreError::ParametrosKdfExcesivos);
}

#[test]
fn rf_94_headers_that_panic_the_library_are_archivo_danado_and_the_app_goes_on() {
    for (label, bytes) in panicking_headers() {
        assert_eq!(open_err(&bytes), CoreError::ArchivoDanado, "{label}");
    }
    // A UInt64 of 1 byte inside the KDF parameters (field 11) is caught too.
    let base = fixture("keepass2/f-pass.kdbx");
    let mut kdf_short = base[..12].to_vec();
    let vd: [u8; 15] = [
        0x00, 0x01, 0x05, 1, 0, 0, 0, b'M', 1, 0, 0, 0, 0x07, 0x00, 0x00,
    ];
    kdf_short.push(11);
    kdf_short.extend(u32::try_from(vd.len()).unwrap().to_le_bytes());
    kdf_short.extend(vd);
    kdf_short.extend([0u8, 0, 0, 0, 0]);
    assert_eq!(open_err(&kdf_short), CoreError::ArchivoDanado);
    // The process is still here and still opens vaults.
    assert!(open::open(&base, &password_key(PASSWORD)).is_ok());
}

#[test]
fn rf_94_catch_unwind_is_the_second_barrier() {
    // Bypass our header reader to prove the library's panic is contained. The default hook would print the
    // panic location (a path of the machine): silence it for this test, which runs in its own process.
    static PANICS: AtomicUsize = AtomicUsize::new(0);
    panic::set_hook(Box::new(|_| {
        PANICS.fetch_add(1, Ordering::SeqCst);
    }));
    for (label, bytes) in panicking_headers() {
        let version = if label.starts_with("KDBX 3") {
            FormatVersion { major: 3, minor: 1 }
        } else {
            KDBX4_1
        };
        let key = DatabaseKey::new().with_password(PASSWORD);
        let before = PANICS.load(Ordering::SeqCst);
        assert_eq!(
            parse_guarded(&bytes, key, version).unwrap_err(),
            CoreError::ArchivoDanado,
            "{label}"
        );
        // The library really panicked (as in the lab) and the panic did not cross `parse_guarded`.
        assert_eq!(PANICS.load(Ordering::SeqCst), before + 1, "{label}");
    }
    drop(panic::take_hook());
}

#[test]
fn rf_11_kdbx3_wrong_password_is_llave_incorrecta() {
    let v3 = fixture("keepass2/v3-aes.kdbx");
    assert_eq!(
        open::open(&v3, &password_key("otra")).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
    let mut truncated = v3.clone();
    truncated.truncate(v3.len() - 50);
    // Spec "Casos límite": a truncated 3.x cannot be told apart from a wrong key.
    assert_eq!(
        open::open(&truncated, &password_key(PASSWORD)).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
}

#[test]
fn rf_14_kdbx3_opens_read_only_and_kdbx4_does_not() {
    let v3 = open::open(&fixture("keepass2/v3-aes.kdbx"), &password_key(PASSWORD)).unwrap();
    assert_eq!(v3.version(), FormatVersion { major: 3, minor: 1 });
    assert!(v3.is_read_only());
    assert_eq!(v3.entry_count(), 1);
    let v4 = open::open(&fixture("keepass2/f-pass.kdbx"), &password_key(PASSWORD)).unwrap();
    assert!(!v4.is_read_only());
}

#[test]
fn rf_17_weak_kdf_opens_with_a_warning() {
    let strong = open::open(&fixture("keepass2/f-pass.kdbx"), &password_key(PASSWORD)).unwrap();
    assert!(!strong.is_kdf_weak(), "Argon2id with 64 MiB is not weak");
    let v3 = open::open(&fixture("keepass2/v3-aes.kdbx"), &password_key(PASSWORD)).unwrap();
    assert!(v3.is_kdf_weak(), "AES-KDF is weak");
    for (label, kdf) in [
        ("Argon2d with 64 MiB", argon2(KDF_ARGON2D, 64 << 20, 2, 2)),
        ("Argon2id with 32 MiB", argon2(KDF_ARGON2ID, 32 << 20, 2, 2)),
        ("AES-KDF in KDBX 4", KdfConfig::Aes { rounds: 1_000 }),
    ] {
        let opened = open::open(&vault_with(kdf), &password_key(PASSWORD)).expect(label);
        assert!(opened.is_kdf_weak(), "{label}");
    }
}
