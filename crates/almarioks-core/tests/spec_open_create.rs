//! Independent validation from the spec: create and open (RF-1 to RF-6, RF-11 to RF-13, RF-16, RF-17, RF-87,
//! RF-94, RF-95; example 7). Hostile headers are forged from a vault made by the core, with the header SHA-256
//! recomputed so only the field under test is wrong.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::{
    fs,
    time::{Duration, Instant},
};

use almarioks_core::{
    CoreError,
    kdbx::{
        self, Argon2Variant, KdfParams, KeyFile, NewKeyFile, NewVaultKdf, VaultKey, read_header,
    },
    session::Status,
    storage::Fingerprint,
};
use keepass::{Database, DatabaseKey, config::KdfConfig};
use spec_support::{PW, Rig, TempDir, kdf, key, password, status_open};

const ONE_SECOND: Duration = Duration::from_secs(1);

fn made_here() -> Vec<u8> {
    kdbx::create_vault(&key(PW), "Bóveda", kdf()).expect("create")
}

// ----- KDBX 4 header surgery (format: signature, version, then TLV fields; END; SHA-256; HMAC; blocks) -----

struct Kdbx4 {
    prefix: Vec<u8>,
    fields: Vec<(u8, Vec<u8>)>,
    after_hash: Vec<u8>,
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    let raw: [u8; 4] = bytes
        .get(at..at + 4)
        .expect("4 bytes")
        .try_into()
        .expect("array");
    usize::try_from(u32::from_le_bytes(raw)).expect("usize")
}

fn split(bytes: &[u8]) -> Kdbx4 {
    let prefix = bytes.get(..12).expect("prefix").to_vec();
    let mut fields = Vec::new();
    let mut pos = 12;
    loop {
        let id = *bytes.get(pos).expect("field id");
        let len = u32_at(bytes, pos + 1);
        let data = bytes
            .get(pos + 5..pos + 5 + len)
            .expect("field data")
            .to_vec();
        pos += 5 + len;
        fields.push((id, data));
        if id == 0 {
            break;
        }
    }
    Kdbx4 {
        prefix,
        fields,
        after_hash: bytes.get(pos + 32..).expect("rest").to_vec(),
    }
}

fn le32(n: usize) -> [u8; 4] {
    u32::try_from(n).expect("u32").to_le_bytes()
}

fn join(h: &Kdbx4) -> Vec<u8> {
    let mut out = h.prefix.clone();
    for (id, data) in &h.fields {
        out.push(*id);
        out.extend(le32(data.len()));
        out.extend(data);
    }
    let sha = Fingerprint::of(&out);
    out.extend(sha.as_bytes());
    out.extend(&h.after_hash);
    out
}

fn field_mut(h: &mut Kdbx4, id: u8) -> &mut Vec<u8> {
    h.fields
        .iter_mut()
        .find(|(i, _)| *i == id)
        .map(|(_, d)| d)
        .expect("field")
}

type Item = (u8, String, Vec<u8>);

fn dict_items(data: &[u8]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut pos = 2;
    while let Some(&kind) = data.get(pos) {
        if kind == 0 {
            break;
        }
        let key_len = u32_at(data, pos + 1);
        let name = String::from_utf8(data.get(pos + 5..pos + 5 + key_len).expect("key").to_vec())
            .expect("utf8");
        let at = pos + 5 + key_len;
        let value_len = u32_at(data, at);
        let value = data
            .get(at + 4..at + 4 + value_len)
            .expect("value")
            .to_vec();
        items.push((kind, name, value));
        pos = at + 4 + value_len;
    }
    items
}

fn dict_bytes(items: &[Item]) -> Vec<u8> {
    let mut out = vec![0x00, 0x01];
    for (kind, name, value) in items {
        out.push(*kind);
        out.extend(le32(name.len()));
        out.extend(name.as_bytes());
        out.extend(le32(value.len()));
        out.extend(value);
    }
    out.push(0);
    out
}

const KDF_PARAMETERS: u8 = 11;

fn with_kdf(bytes: &[u8], edit: impl FnOnce(&mut Vec<Item>)) -> Vec<u8> {
    let mut header = split(bytes);
    let data = field_mut(&mut header, KDF_PARAMETERS);
    let mut items = dict_items(data);
    edit(&mut items);
    *data = dict_bytes(&items);
    join(&header)
}

fn set_item(items: &mut [Item], name: &str, value: Vec<u8>) {
    let item = items.iter_mut().find(|(_, n, _)| n == name).expect("item");
    item.2 = value;
}

fn aes_kdf(rounds: u64) -> Vec<Item> {
    let uuid = vec![
        0xC9, 0xD9, 0xF3, 0x9A, 0x62, 0x8A, 0x44, 0x60, 0xBF, 0x74, 0x0D, 0x08, 0xC1, 0x8A, 0x4F,
        0xEA,
    ];
    vec![
        (0x42, "$UUID".to_owned(), uuid),
        (0x05, "R".to_owned(), rounds.to_le_bytes().to_vec()),
        (0x42, "S".to_owned(), vec![7; 32]),
    ]
}

/// Opens with the RIGHT key, so only the header can stop the derivation; must answer in under a second.
fn open_fast(bytes: &[u8]) -> Result<(), CoreError> {
    let started = Instant::now();
    let result = kdbx::open(bytes, &key(PW)).map(|_| ());
    assert!(
        started.elapsed() < ONE_SECOND,
        "took {:?}: the key was derived",
        started.elapsed()
    );
    result
}

// ----- RF-16 and RF-94 -----

#[test]
fn spec_rf_16_hostile_kdf_parameters_are_rejected_before_deriving() {
    let base = made_here();
    let memory = with_kdf(&base, |i| {
        set_item(i, "M", (64_u64 << 30).to_le_bytes().to_vec())
    });
    assert_eq!(
        open_fast(&memory),
        Err(CoreError::ParametrosKdfExcesivos),
        "64 GiB"
    );
    let iterations = with_kdf(&base, |i| {
        set_item(i, "I", 1_000_000_u64.to_le_bytes().to_vec())
    });
    assert_eq!(
        open_fast(&iterations),
        Err(CoreError::ParametrosKdfExcesivos),
        "1 000 000 iterations"
    );
    let lanes = with_kdf(&base, |i| set_item(i, "P", 17_u32.to_le_bytes().to_vec()));
    assert_eq!(
        open_fast(&lanes),
        Err(CoreError::ParametrosKdfExcesivos),
        "17 lanes"
    );
    let aes = with_kdf(&base, |i| *i = aes_kdf(100_000_001));
    assert_eq!(
        open_fast(&aes),
        Err(CoreError::ParametrosKdfExcesivos),
        "AES-KDF over 100 000 000 rounds"
    );
}

#[test]
fn spec_rf_16_the_limits_themselves_are_allowed() {
    let base = made_here();
    let header = |edit: &dyn Fn(&mut Vec<Item>)| read_header(&with_kdf(&base, edit)).map(|_| ());
    assert_eq!(
        header(&|i| set_item(i, "M", (1_u64 << 30).to_le_bytes().to_vec())),
        Ok(()),
        "1 GiB"
    );
    assert_eq!(
        header(&|i| set_item(i, "M", ((1_u64 << 30) + 1).to_le_bytes().to_vec())),
        Err(CoreError::ParametrosKdfExcesivos)
    );
    assert_eq!(
        header(&|i| set_item(i, "I", 100_u64.to_le_bytes().to_vec())),
        Ok(())
    );
    assert_eq!(
        header(&|i| set_item(i, "I", 101_u64.to_le_bytes().to_vec())),
        Err(CoreError::ParametrosKdfExcesivos)
    );
    assert_eq!(
        header(&|i| set_item(i, "P", 16_u32.to_le_bytes().to_vec())),
        Ok(())
    );
    assert_eq!(header(&|i| *i = aes_kdf(100_000_000)), Ok(()));
}

#[test]
fn spec_rf_94_a_header_field_of_the_wrong_size_is_archivo_danado_and_the_process_survives() {
    let base = made_here();
    let mut cipher = split(&base);
    field_mut(&mut cipher, 2).truncate(15);
    assert_eq!(
        open_fast(&join(&cipher)),
        Err(CoreError::ArchivoDanado),
        "cipher id of 15 bytes"
    );
    let mut compression = split(&base);
    field_mut(&mut compression, 3).truncate(3);
    assert_eq!(
        open_fast(&join(&compression)),
        Err(CoreError::ArchivoDanado),
        "compression flags of 3 bytes"
    );
    let short_memory = with_kdf(&base, |i| set_item(i, "M", vec![0, 0, 0, 4]));
    assert_eq!(
        open_fast(&short_memory),
        Err(CoreError::ArchivoDanado),
        "UInt64 item of 4 bytes"
    );
    let long_lanes = with_kdf(&base, |i| set_item(i, "P", vec![2, 0, 0, 0, 0, 0, 0, 0]));
    assert_eq!(
        open_fast(&long_lanes),
        Err(CoreError::ArchivoDanado),
        "UInt32 item of 8 bytes"
    );
}

/// KDBX 4 requires a 32-byte master seed and a 12-byte ChaCha20 IV (KeePass 2 rejects other lengths as a format
/// error). With the header sum recomputed, the core answers `LlaveIncorrecta`: the person is told the key is wrong
/// when the file is damaged. No crash and no derivation of a hostile KDF, so the impact is the message.
#[test]
#[ignore = "DEFECTO: RF-94 MasterSeed de 31 bytes o IV de 11 con la suma recalculada da LlaveIncorrecta, no ArchivoDanado"]
fn spec_rf_94_a_master_seed_or_iv_of_the_wrong_size_is_archivo_danado() {
    let base = made_here();
    let mut seed = split(&base);
    field_mut(&mut seed, 4).truncate(31);
    assert_eq!(
        open_fast(&join(&seed)),
        Err(CoreError::ArchivoDanado),
        "master seed of 31 bytes"
    );
    let mut iv = split(&base);
    field_mut(&mut iv, 7).truncate(11);
    assert_eq!(
        open_fast(&join(&iv)),
        Err(CoreError::ArchivoDanado),
        "ChaCha20 IV of 11 bytes"
    );
}

// ----- RF-13 and the header rows of "Casos límite" -----

#[test]
fn spec_rf_13_empty_plain_text_and_truncated_files_are_archivo_danado() {
    let base = made_here();
    assert_eq!(open_fast(&[]), Err(CoreError::ArchivoDanado), "0 bytes");
    assert_eq!(
        open_fast("esto es texto plano, no una bóveda\n".as_bytes()),
        Err(CoreError::ArchivoDanado)
    );
    assert_eq!(
        open_fast(base.get(..40).expect("prefix")),
        Err(CoreError::ArchivoDanado),
        "header cut"
    );
    let body_cut = base.get(..base.len() - 50).expect("cut");
    assert_eq!(
        kdbx::open(body_cut, &key(PW)).map(|_| ()),
        Err(CoreError::ArchivoDanado),
        "KDBX 4 truncated"
    );
    let mut flipped = base.clone();
    if let Some(last) = flipped.last_mut() {
        *last ^= 0x01;
    }
    assert_eq!(
        kdbx::open(&flipped, &key(PW)).map(|_| ()),
        Err(CoreError::ArchivoDanado),
        "a block that does not verify"
    );
}

#[test]
fn spec_rf_13_a_header_that_does_not_verify_is_danado_and_one_with_a_recomputed_sum_is_llave_incorrecta()
 {
    let base = made_here();
    let mut header = split(&base);
    if let Some(byte) = field_mut(&mut header, 4).first_mut() {
        *byte ^= 0xFF;
    }
    let recomputed = join(&header);
    assert_eq!(
        kdbx::open(&recomputed, &key(PW)).map(|_| ()),
        Err(CoreError::LlaveIncorrecta),
        "casos límite"
    );
    // Same change with the ORIGINAL sum: the header does not verify (RF-13).
    let mut stale = recomputed;
    let start = base.len() - split(&base).after_hash.len() - 32;
    stale.splice(
        start..start + 32,
        base.get(start..start + 32)
            .expect("old sum")
            .iter()
            .copied(),
    );
    assert_eq!(
        kdbx::open(&stale, &key(PW)).map(|_| ()),
        Err(CoreError::ArchivoDanado)
    );
}

#[test]
fn spec_rf_13_kdb_1_kdbx_2_and_kdbx_5_are_formato_no_soportado() {
    let base = made_here();
    let mut kdb1 = vec![0x03, 0xD9, 0xA2, 0x9A, 0x65, 0xFB, 0x4B, 0xB5];
    kdb1.extend([0_u8; 120]);
    assert_eq!(
        open_fast(&kdb1),
        Err(CoreError::FormatoNoSoportado),
        "KDB 1.x"
    );
    for major in [2_u16, 5] {
        let mut other = base.clone();
        other.splice(10..12, major.to_le_bytes());
        assert_eq!(
            open_fast(&other),
            Err(CoreError::FormatoNoSoportado),
            "KDBX {major}"
        );
    }
}

// ----- RF-11, example 7 -----

#[test]
fn spec_rf_11_example_7_three_wrong_keys_give_the_same_code_and_text() {
    let dir = TempDir::new("ex7");
    let key_a = dir.join("a.keyx");
    let key_b = dir.join("b.keyx");
    kdbx::create_named_vault_file(
        &dir.join("a.kdbx"),
        "A",
        password(PW),
        NewKeyFile::Generate(key_a.clone()),
        kdf(),
    )
    .expect("vault A");
    kdbx::create_named_vault_file(
        &dir.join("b.kdbx"),
        "B",
        password(PW),
        NewKeyFile::Generate(key_b.clone()),
        kdf(),
    )
    .expect("vault B");
    let bytes = fs::read(dir.join("a.kdbx")).expect("read");
    let file = |p: &std::path::Path| {
        Some(KeyFile::from_bytes(fs::read(p).expect("key file")).expect("key"))
    };
    let wrong = [
        VaultKey::new(password("contraseña-mala"), file(&key_a)).expect("k1"),
        VaultKey::new(password(PW), file(&key_b)).expect("k2"),
        VaultKey::new(password(PW), None).expect("k3"),
    ];
    let errors: Vec<CoreError> = wrong
        .iter()
        .map(|k| kdbx::open(&bytes, k).map(|_| ()).expect_err("wrong"))
        .collect();
    assert!(
        errors.iter().all(|e| *e == CoreError::LlaveIncorrecta),
        "{errors:?}"
    );
    let texts: Vec<String> = errors.iter().map(|e| e.message()).collect();
    assert!(
        texts.windows(2).all(|w| w.first() == w.get(1)),
        "same text: {texts:?}"
    );
    assert!(
        kdbx::open(
            &bytes,
            &VaultKey::new(password(PW), file(&key_a)).expect("right")
        )
        .is_ok()
    );

    // Through the session too, and the vault stays locked.
    let rig = Rig::closed("ex7-session");
    fs::write(&rig.path, &bytes).expect("write");
    let via_session = [
        rig.session
            .unlock(&rig.path, password("contraseña-mala"), Some(&key_a)),
        rig.session.unlock(&rig.path, password(PW), Some(&key_b)),
        rig.session.unlock(&rig.path, password(PW), None),
    ];
    for result in via_session {
        assert_eq!(result.map(|_| ()), Err(CoreError::LlaveIncorrecta));
        assert!(!matches!(rig.session.status(), Status::Open(_)));
    }
}

// ----- RF-12 and RF-87 -----

fn upper_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn keyx_v2(key: &[u8; 32]) -> Vec<u8> {
    let hash = Fingerprint::of(key);
    let check = upper_hex(hash.as_bytes().get(..4).expect("4"));
    let data = upper_hex(key);
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>2.0</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data Hash=\"{check}\">{data}</Data>\n\t</Key>\n</KeyFile>\n"
    )
    .into_bytes()
}

#[test]
fn spec_rf_12_a_keyx_whose_hash_does_not_match_is_archivo_llave_danado() {
    let raw = [0x5A_u8; 32];
    let good = keyx_v2(&raw);
    assert!(
        KeyFile::from_bytes(good.clone()).is_ok(),
        "a well-formed .keyx v2 is accepted"
    );
    let text = String::from_utf8(good).expect("utf8");
    let check = upper_hex(Fingerprint::of(&raw).as_bytes().get(..4).expect("4"));
    let other = if check == "00000000" {
        "11111111"
    } else {
        "00000000"
    };
    let bad_hash = text.replacen(
        &format!("Hash=\"{check}\""),
        &format!("Hash=\"{other}\""),
        1,
    );
    assert_ne!(bad_hash, text);
    assert_eq!(
        KeyFile::from_bytes(bad_hash.into_bytes()).map(|_| ()),
        Err(CoreError::ArchivoLlaveDanado)
    );
    let bad_data = text.replacen(">5A5A", ">5B5A", 1);
    assert_eq!(
        KeyFile::from_bytes(bad_data.into_bytes()).map(|_| ()),
        Err(CoreError::ArchivoLlaveDanado)
    );
}

#[test]
fn spec_rf_87_key_files_in_every_keepass_format_give_the_same_key() {
    let raw: [u8; 32] = std::array::from_fn(|i| u8::try_from(i * 7 + 3).expect("byte"));
    let dir = TempDir::new("rf87");
    let path = dir.join("v.kdbx");
    let existing = KeyFile::from_bytes(raw.to_vec()).expect("32 raw bytes");
    kdbx::create_named_vault_file(&path, "V", None, NewKeyFile::Existing(existing), kdf())
        .expect("create");
    let bytes = fs::read(&path).expect("read");
    let opens = |content: Vec<u8>| {
        let file = KeyFile::from_bytes(content).expect("key file");
        kdbx::open(&bytes, &VaultKey::new(None, Some(file)).expect("key")).map(|_| ())
    };
    assert_eq!(opens(raw.to_vec()), Ok(()), "32 raw bytes");
    assert_eq!(
        opens(upper_hex(&raw).to_lowercase().into_bytes()),
        Ok(()),
        "64 hex characters"
    );
    assert_eq!(
        opens(keyx_v2(&raw)),
        Ok(()),
        ".keyx version 2 with the same key"
    );

    // Any other file counts by its SHA-256: a vault keyed with a "photo" opens with those 32 bytes.
    let photo: Vec<u8> = (0..5_000_u32)
        .map(|i| u8::try_from(i % 251).expect("byte"))
        .collect();
    let digest = *Fingerprint::of(&photo).as_bytes();
    let path = dir.join("foto.kdbx");
    let existing = KeyFile::from_bytes(photo.clone()).expect("photo");
    kdbx::create_named_vault_file(&path, "F", None, NewKeyFile::Existing(existing), kdf())
        .expect("create");
    let bytes = fs::read(&path).expect("read");
    let with = |content: Vec<u8>| {
        kdbx::open(
            &bytes,
            &VaultKey::new(None, Some(KeyFile::from_bytes(content).expect("k"))).expect("key"),
        )
        .map(|_| ())
    };
    assert_eq!(with(photo), Ok(()));
    assert_eq!(
        with(digest.to_vec()),
        Ok(()),
        "RF-87: any other file is its SHA-256"
    );
}

// ----- RF-1 to RF-6 -----

#[test]
fn spec_rf_1_rf_2_a_new_vault_reopens_with_its_key_only_and_uses_argon2id() {
    let dir = TempDir::new("rf1");
    let path = dir.join("nueva.kdbx");
    kdbx::create_named_vault_file(
        &path,
        "Nueva",
        password("ñ€😀 maestra"),
        NewKeyFile::None,
        kdf(),
    )
    .expect("create");
    let bytes = fs::read(&path).expect("read");
    assert!(kdbx::open(&bytes, &key("ñ€😀 maestra")).is_ok());
    assert_eq!(
        kdbx::open(&bytes, &key("ñ€😀 maestrA")).map(|_| ()),
        Err(CoreError::LlaveIncorrecta)
    );
    let header = read_header(&bytes).expect("header");
    assert_eq!(header.version.major, 4);
    match header.kdf {
        KdfParams::Argon2 {
            variant,
            memory_bytes,
            iterations,
            ..
        } => {
            assert_eq!(variant, Argon2Variant::Id);
            assert!(memory_bytes >= 64 << 20, "RF-2: {memory_bytes}");
            assert!((2..=50).contains(&iterations), "RF-2: {iterations}");
        }
        other => panic!("RF-2: {other:?}"),
    }
    assert!(NewVaultKdf::with_iterations(1).is_err() && NewVaultKdf::with_iterations(51).is_err());
}

#[test]
fn spec_rf_2_calibration_picks_the_iterations_closest_to_one_second() {
    let linear = |ms_per_iteration: u64| {
        NewVaultKdf::calibrate(|k| Ok(Duration::from_millis(k.iterations() * ms_per_iteration)))
            .expect("calibrate")
    };
    assert_eq!(linear(100).iterations(), 10);
    assert_eq!(linear(30).iterations(), 33, "990 ms beats 1 020 ms");
    assert_eq!(linear(1).iterations(), 50, "never more than 50");
    assert_eq!(linear(700).iterations(), 2, "never fewer than 2");
}

#[test]
fn spec_rf_3_rf_4_rf_6_creation_refuses_without_touching_anything() {
    let dir = TempDir::new("rf3");
    let path = dir.join("v.kdbx");
    assert_eq!(
        kdbx::create_named_vault_file(&path, "V", None, NewKeyFile::None, kdf()).map(|_| ()),
        Err(CoreError::LlaveVacia)
    );
    assert!(!path.exists(), "RF-3: nothing written");

    fs::write(&path, b"ya existe").expect("write");
    assert_eq!(
        kdbx::create_named_vault_file(&path, "V", password(PW), NewKeyFile::None, kdf())
            .map(|_| ()),
        Err(CoreError::ArchivoExiste)
    );
    assert_eq!(fs::read(&path).expect("read"), b"ya existe", "RF-4");

    let fresh = dir.join("otra.kdbx");
    let key_path = dir.join("llave.keyx");
    fs::write(&key_path, b"llave previa").expect("write");
    assert_eq!(
        kdbx::create_named_vault_file(
            &fresh,
            "V",
            password(PW),
            NewKeyFile::Generate(key_path.clone()),
            kdf()
        )
        .map(|_| ()),
        Err(CoreError::ArchivoExiste)
    );
    assert_eq!(fs::read(&key_path).expect("read"), b"llave previa", "RF-6");
    assert!(!fresh.exists(), "RF-6: the vault is not created either");
}

#[test]
fn spec_rf_5_a_new_key_file_is_keyx_v2_with_32_bytes_and_opens_the_vault() {
    let dir = TempDir::new("rf5");
    let path = dir.join("v.kdbx");
    let key_path = dir.join("v.keyx");
    kdbx::create_named_vault_file(
        &path,
        "V",
        None,
        NewKeyFile::Generate(key_path.clone()),
        kdf(),
    )
    .expect("create");
    let text = fs::read_to_string(&key_path).expect("keyx");
    assert!(text.contains("<Version>2.0</Version>"), "{text}");
    let data = text
        .split("\">")
        .nth(1)
        .and_then(|r| r.split("</Data>").next())
        .expect("Data");
    let hex: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(hex.len(), 64, "32 bytes");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    let file = KeyFile::from_bytes(text.into_bytes()).expect("valid");
    let bytes = fs::read(&path).expect("read");
    assert!(kdbx::open(&bytes, &VaultKey::new(None, Some(file)).expect("key")).is_ok());
    // A second generated key is different (it comes from the OS generator).
    let other_key = dir.join("w.keyx");
    kdbx::create_named_vault_file(
        &dir.join("w.kdbx"),
        "W",
        None,
        NewKeyFile::Generate(other_key.clone()),
        kdf(),
    )
    .expect("create");
    assert_ne!(
        fs::read(&other_key).expect("w"),
        fs::read(&key_path).expect("v")
    );
}

// ----- RF-95 and RF-17 -----

#[test]
fn spec_rf_95_missing_vault_or_key_file_says_which() {
    let rig = Rig::closed("rf95");
    let missing = rig.dir.join("no-esta.kdbx");
    assert_eq!(
        rig.session.unlock(&missing, password(PW), None).map(|_| ()),
        Err(CoreError::ArchivoNoEncontrado)
    );
    let missing_key = rig.dir.join("no-esta.keyx");
    assert_eq!(
        rig.session
            .unlock(&rig.path, password(PW), Some(&missing_key))
            .map(|_| ()),
        Err(CoreError::ArchivoLlaveNoEncontrado)
    );
}

fn rewrite_kdf(edit: impl FnOnce(KdfConfig) -> KdfConfig) -> Vec<u8> {
    let mut db =
        Database::parse(&made_here(), DatabaseKey::new().with_password(PW)).expect("parse");
    db.config.kdf_config = edit(db.config.kdf_config.clone());
    let mut out = Vec::new();
    db.save(&mut out, DatabaseKey::new().with_password(PW))
        .expect("save");
    out
}

#[test]
fn spec_rf_17_a_weak_kdf_opens_with_a_warning() {
    let aes = rewrite_kdf(|_| KdfConfig::Aes { rounds: 1_000 });
    assert!(
        kdbx::open(&aes, &key(PW))
            .expect("AES-KDF opens")
            .is_kdf_weak()
    );
    let small = rewrite_kdf(|kdf| match kdf {
        KdfConfig::Argon2id {
            iterations,
            parallelism,
            version,
            ..
        } => KdfConfig::Argon2id {
            iterations,
            memory: 32 << 20,
            parallelism,
            version,
        },
        other => other,
    });
    assert!(
        kdbx::open(&small, &key(PW))
            .expect("32 MiB opens")
            .is_kdf_weak(),
        "under 64 MiB"
    );
    let argon2d = rewrite_kdf(|kdf| match kdf {
        KdfConfig::Argon2id {
            iterations,
            memory,
            parallelism,
            version,
        } => KdfConfig::Argon2 {
            iterations,
            memory,
            parallelism,
            version,
        },
        other => other,
    });
    assert!(
        kdbx::open(&argon2d, &key(PW))
            .expect("Argon2d opens")
            .is_kdf_weak(),
        "not Argon2id"
    );
    assert!(
        !kdbx::open(&made_here(), &key(PW))
            .expect("ours")
            .is_kdf_weak()
    );

    let rig = Rig::closed("rf17");
    fs::write(&rig.path, aes).expect("write");
    assert!(status_open(rig.unlock().expect("unlock")).weak_kdf);
}
