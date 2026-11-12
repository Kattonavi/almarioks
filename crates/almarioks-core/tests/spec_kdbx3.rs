//! Independent validation from the spec: KDBX 3.x opens read-only and converts to a new KDBX 4 file (RF-14,
//! RF-15), on the KeePass 2 fixture `keepass2/v3-aes.kdbx` (password from `docs/lab/kdbx-roundtrip/correr.ps1`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::{fs, path::PathBuf};

use almarioks_core::{
    CoreError,
    kdbx::{self, Argon2Variant, KdfParams, MasterPassword, VaultKey, read_header},
    session::Applied,
};
use spec_support::{Rig, TempDir, at, kdf, new_entry, status_open};

const FIXTURE_PASSWORD: &str = "lab-Maestra-2026";

fn fixture() -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keepass2/v3-aes.kdbx");
    fs::read(path).expect("fixture")
}

fn fixture_key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(FIXTURE_PASSWORD.to_owned())), None).expect("key")
}

#[test]
fn spec_rf_14_a_kdbx_3_vault_opens_read_only_and_every_change_is_solo_lectura() {
    let bytes = fixture();
    let mut vault = kdbx::open(&bytes, &fixture_key()).expect("RF-14: KDBX 3.1 opens");
    assert_eq!(vault.version().major, 3);
    assert!(vault.is_read_only());
    let root = vault.root_id();
    let any_entry = vault
        .list_entries(root)
        .expect("list")
        .first()
        .map(|e| e.id);
    assert_eq!(
        vault
            .create_entry(root, new_entry("Nueva"), at(0))
            .map(|_| ()),
        Err(CoreError::SoloLectura)
    );
    assert_eq!(
        vault.create_group(root, "Grupo", at(0)).map(|_| ()),
        Err(CoreError::SoloLectura)
    );
    assert_eq!(
        vault.rename_group(root, "Otro", at(0)),
        Err(CoreError::SoloLectura)
    );
    if let Some(id) = any_entry {
        assert_eq!(
            vault.delete_entry(id, at(0)).map(|_| ()),
            Err(CoreError::SoloLectura)
        );
    }
    assert_eq!(
        vault.to_bytes(&fixture_key()).map(|_| ()),
        Err(CoreError::SoloLectura)
    );
    // Definiciones: SoloLectura comes before NoEncontrado and before the domain rule.
    let missing = almarioks_core::vault::GroupId::from_uuid(uuid::Uuid::from_u128(1));
    assert_eq!(
        vault
            .create_entry(missing, new_entry("   "), at(0))
            .map(|_| ()),
        Err(CoreError::SoloLectura)
    );

    // Through the session: open, read-only, every change SoloLectura and the file untouched.
    let rig = Rig::closed("rf14");
    fs::write(&rig.path, &bytes).expect("write");
    let open = status_open(
        rig.session
            .unlock(
                &rig.path,
                Some(MasterPassword::new(FIXTURE_PASSWORD.to_owned())),
                None,
            )
            .expect("unlock"),
    );
    assert!(open.read_only);
    assert_eq!(
        rig.session.mutate(|v, now| {
            let root = v.root_id();
            v.create_entry(root, new_entry("x"), now)
                .map(Applied::Changed)
        }),
        Err(CoreError::SoloLectura)
    );
    assert_eq!(
        rig.session.mutate(|_, _| Ok(Applied::Unchanged(()))),
        Err(CoreError::SoloLectura)
    );
    assert_eq!(fs::read(&rig.path).expect("file"), bytes);
}

#[test]
fn spec_rf_15_converting_writes_a_new_kdbx_4_with_the_new_vault_kdf_and_leaves_the_original_identical()
 {
    let dir = TempDir::new("rf15");
    let source = dir.join("vieja.kdbx");
    let bytes = fixture();
    fs::write(&source, &bytes).expect("write");
    let original = kdbx::open(&bytes, &fixture_key()).expect("open");
    let target = dir.join("nueva.kdbx");
    let converted =
        kdbx::convert_vault_file(&bytes, &target, &fixture_key(), kdf()).expect("convert");
    assert_eq!(
        fs::read(&source).expect("source"),
        bytes,
        "RF-15: original byte for byte"
    );
    let written = fs::read(&target).expect("target");
    let header = read_header(&written).expect("header");
    assert_eq!(header.version.major, 4);
    assert!(
        matches!(header.kdf, KdfParams::Argon2 { variant: Argon2Variant::Id, memory_bytes, iterations, .. }
            if memory_bytes >= 64 << 20 && (2..=50).contains(&iterations)),
        "RF-2 KDF: {:?}",
        header.kdf
    );
    let reopened = kdbx::open(&written, &fixture_key()).expect("same key");
    assert!(!reopened.is_read_only());
    assert_eq!(reopened.entry_count(), original.entry_count());
    assert_eq!(reopened.group_count(), original.group_count());
    assert_eq!(converted.entry_count(), original.entry_count());
    // RF-15 "en una ruta nueva": never over an existing file.
    assert_eq!(
        kdbx::convert_vault_file(&bytes, &source, &fixture_key(), kdf()).map(|_| ()),
        Err(CoreError::ArchivoExiste)
    );
    assert_eq!(fs::read(&source).expect("source"), bytes);
}
