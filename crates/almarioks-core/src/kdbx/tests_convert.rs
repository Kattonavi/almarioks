//! KDBX 3.x conversion as the file sees it (RF-15): the converted database is the original one with only the format
//! changed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use super::{MasterPassword, NewVaultKdf, VaultKey, convert_to_kdbx4, open};

fn v3() -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keepass2/v3-aes.kdbx"))
        .unwrap()
}

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new("lab-Maestra-2026".into())), None).unwrap()
}

#[test]
fn rf_15_the_converted_database_differs_only_in_its_format() {
    let original = open::open(&v3(), &key()).unwrap();
    let kdf = NewVaultKdf::with_iterations(2).unwrap();
    let converted = open::open(&convert_to_kdbx4(&v3(), &key(), kdf).unwrap(), &key()).unwrap();
    let mut expected = original.db.clone();
    expected.config = converted.db.config.clone();
    assert_eq!(expected, converted.db);
}
