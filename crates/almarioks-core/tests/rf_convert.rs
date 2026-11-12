//! KDBX 3.x: read only (RF-14) and conversion to KDBX 4 in a new file (RF-15, T4). The fixture is the 3.1 vault
//! of KeePass 2.62 (`docs/lab/boveda-0.1`, fake data); its password is the one of `tests/rf_open.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use almarioks_core::{
    CoreError,
    kdbx::{
        Argon2Variant, FormatVersion, KdfParams, MAX_NEW_VAULT_ITERATIONS,
        MIN_NEW_VAULT_ITERATIONS, MasterPassword, NEW_VAULT_MEMORY_BYTES, NewVaultKdf, OpenedVault,
        VaultKey, convert_to_kdbx4, convert_vault_file, open, read_header,
    },
    vault::{EntryId, EntryPatch, GroupId, GroupNode, NewEntry, Timestamp},
};

const PASSWORD: &str = "lab-Maestra-2026";

fn fixture_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn v3_bytes() -> Vec<u8> {
    fs::read(fixture_path("keepass2/v3-aes.kdbx")).unwrap()
}

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn kdf() -> NewVaultKdf {
    NewVaultKdf::with_iterations(2).unwrap()
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200 + seconds).unwrap()
}

/// A fresh folder under the system temp dir, removed at the end.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!(
            "almarioks-convert-test-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

/// Everything the read model shows of a vault: tree, and every group's entries with their detail.
fn content(vault: &OpenedVault) -> (GroupNode, Vec<String>) {
    fn walk(vault: &OpenedVault, node: &GroupNode, out: &mut Vec<String>) {
        for summary in vault.list_entries(node.id).unwrap() {
            out.push(format!("{:?}", vault.entry(summary.id).unwrap()));
            for field in vault.entry(summary.id).unwrap().protected_fields {
                // Protected values too, compared without printing them on failure.
                let value = vault.reveal_field(summary.id, &field).unwrap();
                out.push(format!(
                    "{field}:{}",
                    secrecy::ExposeSecret::expose_secret(&value).len()
                ));
            }
        }
        for child in &node.children {
            walk(vault, child, out);
        }
    }
    let tree = vault.group_tree();
    let mut entries = Vec::new();
    walk(vault, &tree, &mut entries);
    (tree, entries)
}

fn assert_new_vault_kdf(bytes: &[u8]) {
    let header = read_header(bytes).unwrap();
    assert_eq!(header.version.major, 4);
    match header.kdf {
        KdfParams::Argon2 {
            variant,
            memory_bytes,
            iterations,
            ..
        } => {
            assert_eq!(variant, Argon2Variant::Id);
            assert!(memory_bytes >= 64 << 20 && memory_bytes == NEW_VAULT_MEMORY_BYTES);
            assert!((MIN_NEW_VAULT_ITERATIONS..=MAX_NEW_VAULT_ITERATIONS).contains(&iterations));
        }
        KdfParams::AesKdf { .. } => panic!("AES-KDF after converting"),
    }
}

#[test]
fn rf_15_convert_gives_a_kdbx4_with_the_kdf_of_rf_2_and_the_same_content() {
    let original_bytes = v3_bytes();
    let original = open(&original_bytes, &key()).unwrap();
    assert_eq!(original.version(), FormatVersion { major: 3, minor: 1 });
    let converted_bytes = convert_to_kdbx4(&original_bytes, &key(), kdf()).unwrap();
    assert_new_vault_kdf(&converted_bytes);
    // Reopens with the same key, writable, with the same groups and entries.
    let converted = open(&converted_bytes, &key()).unwrap();
    assert!(!converted.is_read_only());
    assert!(!converted.is_kdf_weak());
    assert_eq!(content(&converted), content(&original));
    // A wrong key opens neither.
    let wrong = VaultKey::new(Some(MasterPassword::new("otra".into())), None).unwrap();
    assert_eq!(
        open(&converted_bytes, &wrong).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
    assert_eq!(
        convert_to_kdbx4(&original_bytes, &wrong, kdf()).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
}

#[test]
fn rf_15_convert_to_a_new_file_leaves_the_original_byte_for_byte() {
    let dir = TempDir::new();
    let source = dir.join("viejo.kdbx");
    fs::copy(fixture_path("keepass2/v3-aes.kdbx"), &source).unwrap();
    let before = fs::read(&source).unwrap();
    let target = dir.join("nuevo.kdbx");
    let opened = convert_vault_file(&before, &target, &key(), kdf()).unwrap();
    assert!(!opened.is_read_only());
    assert_eq!(fs::read(&source).unwrap(), before, "the original changed");
    assert_new_vault_kdf(&fs::read(&target).unwrap());
    let reopened = open(&fs::read(&target).unwrap(), &key()).unwrap();
    assert_eq!(content(&reopened), content(&open(&before, &key()).unwrap()));
}

#[test]
fn rf_15_rf_4_convert_never_writes_over_an_existing_file() {
    let dir = TempDir::new();
    let source = dir.join("viejo.kdbx");
    fs::copy(fixture_path("keepass2/v3-aes.kdbx"), &source).unwrap();
    let before = fs::read(&source).unwrap();
    // Over itself, and over another file: both untouched.
    assert_eq!(
        convert_vault_file(&before, &source, &key(), kdf()).unwrap_err(),
        CoreError::ArchivoExiste
    );
    let other = dir.join("otro.kdbx");
    fs::write(&other, b"no tocar").unwrap();
    assert_eq!(
        convert_vault_file(&before, &other, &key(), kdf()).unwrap_err(),
        CoreError::ArchivoExiste
    );
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(fs::read(&other).unwrap(), b"no tocar");
    // A failed conversion (wrong key) leaves no file behind.
    let target = dir.join("nuevo.kdbx");
    let wrong = VaultKey::new(Some(MasterPassword::new("otra".into())), None).unwrap();
    assert_eq!(
        convert_vault_file(&before, &target, &wrong, kdf()).unwrap_err(),
        CoreError::LlaveIncorrecta
    );
    assert!(!Path::new(&target).exists());
}

#[test]
fn rf_15_a_kdbx4_vault_is_not_converted() {
    let bytes = fs::read(fixture_path("keepass2/f-pass.kdbx")).unwrap();
    assert_eq!(
        convert_to_kdbx4(&bytes, &key(), kdf()).unwrap_err(),
        CoreError::ArgumentoInvalido
    );
}

#[test]
fn rf_14_every_change_to_a_kdbx3_vault_is_solo_lectura() {
    let mut vault = open(&v3_bytes(), &key()).unwrap();
    assert!(vault.is_read_only());
    let root = vault.root_id();
    let entry = vault.list_entries(root).unwrap()[0].id;
    let ghost_group = GroupId::parse("00000000-0000-4000-8000-0000000000dd").unwrap();
    let ghost_entry = EntryId::parse("00000000-0000-4000-8000-0000000000ee").unwrap();
    let before = content(&vault);
    let titled = || NewEntry {
        title: "t".into(),
        ..NewEntry::default()
    };
    let retitle = || EntryPatch {
        title: Some("Otro".into()),
        ..EntryPatch::default()
    };
    // Existing ids, unknown ids and invalid values: SoloLectura comes first (spec "Códigos de error").
    let results = [
        vault.create_entry(root, titled(), at(0)).map(|_| ()),
        vault
            .create_entry(ghost_group, NewEntry::default(), at(0))
            .map(|_| ()),
        vault.update_entry(entry, retitle(), at(0)).map(|_| ()),
        vault
            .update_entry(ghost_entry, retitle(), at(0))
            .map(|_| ()),
        vault.move_entry(entry, root, at(0)),
        vault.move_entry(ghost_entry, ghost_group, at(0)),
        vault.delete_entry(entry, at(0)).map(|_| ()),
        vault.delete_entry(ghost_entry, at(0)).map(|_| ()),
        vault.create_group(root, "Nuevo", at(0)).map(|_| ()),
        vault.create_group(ghost_group, "", at(0)).map(|_| ()),
        vault.rename_group(root, "Otra", at(0)),
        vault.rename_group(ghost_group, "Otra", at(0)),
        vault.move_group(root, root, at(0)),
        vault.move_group(ghost_group, root, at(0)),
        vault.delete_group(root, at(0)).map(|_| ()),
        vault.delete_group(ghost_group, at(0)).map(|_| ()),
        vault.empty_recycle_bin(at(0)).map(|_| ()),
        vault.to_bytes(&key()).map(|_| ()),
    ];
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap_err(), CoreError::SoloLectura, "operation {i}");
    }
    // Nothing changed in memory either.
    assert_eq!(content(&vault), before);
}
