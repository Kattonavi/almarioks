//! Opening vaults through the public API of `almarioks_core::kdbx` (RF-10, RF-11 with example 7, RF-12, RF-87).
//!
//! Fixtures (fake data) come from KeePass 2.62 and KeePassXC and are only read. The key of `llave.keyx` and its
//! base64 below were derived once, by hand, from the fixture (`Hash="663CE0D4"` checks against it).

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use almarioks_core::{
    CoreError,
    kdbx::{FormatVersion, KeyFile, MasterPassword, VaultKey, open},
};

const PASSWORD: &str = "lab-Maestra-2026";
const UNICODE_PASSWORD: &str = "ñandú-€-2026";
/// The `<Data>` of `keepass2/llave.keyx`, without its spaces and line breaks.
const KEY_HEX: &str = "F8174CA6DA806A61E0175312B95D903E40108AD0F07464A2D4234FD52AF0BEA4";
const KEY_BASE64: &str = "+BdMptqAamHgF1MSuV2QPkAQitDwdGSi1CNP1SrwvqQ=";

fn fixture(rel: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel);
    fs::read(path).expect("read fixture")
}

fn key(password: Option<&str>, key_file: Option<Vec<u8>>) -> VaultKey {
    let password = password.map(|p| MasterPassword::new(p.to_owned()));
    let key_file = key_file.map(|bytes| KeyFile::from_bytes(bytes).expect("valid key file"));
    VaultKey::new(password, key_file).expect("non-empty key")
}

fn keyx() -> Vec<u8> {
    fixture("keepass2/llave.keyx")
}

fn raw_key() -> Vec<u8> {
    (0..KEY_HEX.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(KEY_HEX.get(i..i + 2).unwrap(), 16).unwrap())
        .collect()
}

/// A `.keyx` version 2 for another vault: same format, other 32 bytes, correct hash of its own.
fn other_keyx() -> Vec<u8> {
    // SHA-256 of 32 zero bytes starts with 66687AAD.
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>2.0</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data Hash=\"66687AAD\">{}</Data>\n\t</Key>\n</KeyFile>\n",
        "00".repeat(32)
    );
    xml.into_bytes()
}

#[test]
fn rf_10_opens_every_kdbx4_fixture_of_keepass2_and_keepassxc_with_its_key() {
    let cases = [
        ("keepass2/f-pass.kdbx", key(Some(PASSWORD), None)),
        ("keepass2/f-key.kdbx", key(None, Some(keyx()))),
        ("keepass2/f-both.kdbx", key(Some(PASSWORD), Some(keyx()))),
        ("keepass2/f-unicode.kdbx", key(Some(UNICODE_PASSWORD), None)),
        ("keepassxc/xc-both4.kdbx", key(Some(PASSWORD), Some(keyx()))),
    ];
    for (rel, key) in cases {
        let opened = open(&fixture(rel), &key).unwrap_or_else(|e| panic!("{rel}: {}", e.code()));
        assert_eq!(
            opened.version(),
            FormatVersion { major: 4, minor: 1 },
            "{rel}"
        );
        assert!(!opened.is_read_only(), "{rel}");
        assert!(!opened.is_kdf_weak(), "{rel}: Argon2id 64 MiB");
        assert!(
            opened.entry_count() > 0 && opened.group_count() > 1,
            "{rel}: groups and entries"
        );
    }
}

#[test]
fn rf_11_example_7_every_wrong_part_gives_the_same_code_and_text() {
    let vault = fixture("keepass2/f-both.kdbx");
    let wrong = [
        key(Some("lab-Maestra-2025"), Some(keyx())), // bad password + good key file
        key(Some(PASSWORD), Some(other_keyx())),     // good password + key file of another vault
        key(Some(PASSWORD), None),                   // good password without key file
    ];
    let errors: Vec<CoreError> = wrong.iter().map(|k| open(&vault, k).unwrap_err()).collect();
    for error in &errors {
        assert_eq!(*error, CoreError::LlaveIncorrecta);
        assert_eq!(error.code(), "LlaveIncorrecta");
        assert_eq!(error.message(), errors[0].message());
    }
    // And the right key still opens it: the vault stays as it was.
    assert!(open(&vault, &key(Some(PASSWORD), Some(keyx()))).is_ok());
}

#[test]
fn rf_11_wrong_password_or_extra_key_file_on_a_password_vault() {
    let vault = fixture("keepass2/f-pass.kdbx");
    assert_eq!(
        open(&vault, &key(Some("otra"), None)).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
    assert_eq!(
        open(&vault, &key(Some(PASSWORD), Some(keyx()))).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
    assert_eq!(
        open(
            &fixture("keepass2/f-key.kdbx"),
            &key(None, Some(other_keyx()))
        )
        .unwrap_err(),
        CoreError::LlaveIncorrecta
    );
}

#[test]
fn rf_03_no_password_and_no_key_file_is_llave_vacia() {
    assert_eq!(
        VaultKey::new(None, None).unwrap_err(),
        CoreError::LlaveVacia
    );
    let empty = MasterPassword::new(String::new());
    assert_eq!(
        VaultKey::new(Some(empty), None).unwrap_err(),
        CoreError::LlaveVacia
    );
}

#[test]
fn rf_12_keyx_v2_with_a_hash_that_does_not_match_is_archivo_llave_danado() {
    let pristine = String::from_utf8(keyx()).unwrap();
    assert!(KeyFile::from_bytes(pristine.clone().into_bytes()).is_ok());
    // One digit of the key changed (F8174CA6 → F8174CA7): the hash no longer matches.
    let data_changed = pristine.replacen("F8174CA6", "F8174CA7", 1);
    assert_ne!(data_changed, pristine);
    assert_eq!(
        KeyFile::from_bytes(data_changed.into_bytes()).unwrap_err(),
        CoreError::ArchivoLlaveDanado
    );
    // One digit of the hash changed.
    let hash_changed = pristine.replacen("663CE0D4", "663CE0D5", 1);
    assert_ne!(hash_changed, pristine);
    assert_eq!(
        KeyFile::from_bytes(hash_changed.into_bytes()).unwrap_err(),
        CoreError::ArchivoLlaveDanado
    );
    // Key data that is not hexadecimal in a version 2 file.
    let not_hex = pristine.replacen("F8174CA6", "Z8174CA6", 1);
    assert_eq!(
        KeyFile::from_bytes(not_hex.into_bytes()).unwrap_err(),
        CoreError::ArchivoLlaveDanado
    );
}

#[test]
fn rf_87_key_file_formats_of_keepass2_open_the_same_vault() {
    let vault = fixture("keepass2/f-key.kdbx");
    let xml_v1 = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile>\n\t<Meta>\n\t\t<Version>1.00</Version>\n\t</Meta>\n\t<Key>\n\t\t<Data>{KEY_BASE64}</Data>\n\t</Key>\n</KeyFile>\n"
    );
    let formats = [
        (".keyx version 2 (KeePass 2.62)", keyx()),
        ("XML version 1", xml_v1.into_bytes()),
        ("32 raw bytes", raw_key()),
        ("64 hexadecimal characters", KEY_HEX.as_bytes().to_vec()),
    ];
    for (label, bytes) in formats {
        let opened = open(&vault, &key(None, Some(bytes)));
        assert!(
            opened.is_ok(),
            "{label}: {:?}",
            opened.err().map(CoreError::code)
        );
    }
}

#[test]
fn rf_87_any_other_file_is_a_key_by_its_sha256() {
    // A photo or an old `.key`: any bytes are accepted as a key file and never as the key of another file.
    let photo: Vec<u8> = (0u16..1000)
        .map(|i| u8::try_from(i % 251).unwrap())
        .collect();
    assert!(KeyFile::from_bytes(photo.clone()).is_ok());
    let opened = open(&fixture("keepass2/f-key.kdbx"), &key(None, Some(photo)));
    assert_eq!(opened.unwrap_err(), CoreError::LlaveIncorrecta);
}
