//! Read model through the public API (RF-42 to RF-44, RF-50, RF-93) against the fixtures of KeePass 2 and
//! KeePassXC and a vault created in the test. Fixtures hold fake data only and are read into memory.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use almarioks_core::{
    CoreError,
    kdbx::{KeyFile, MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open},
    text,
    vault::{EntryId, GroupId, GroupKind, GroupNode},
};
use secrecy::ExposeSecret;

const PASSWORD: &str = "lab-Maestra-2026";
const UNICODE_PASSWORD: &str = "ñandú-€-2026";

fn fixture(rel: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel);
    fs::read(path).expect("read fixture")
}

fn key(password: Option<&str>, key_file: bool) -> VaultKey {
    VaultKey::new(
        password.map(|p| MasterPassword::new(p.to_owned())),
        key_file.then(|| KeyFile::from_bytes(fixture("keepass2/llave.keyx")).unwrap()),
    )
    .unwrap()
}

fn fixtures() -> Vec<(&'static str, OpenedVault)> {
    [
        ("keepass2/f-pass.kdbx", key(Some(PASSWORD), false)),
        ("keepass2/f-key.kdbx", key(None, true)),
        ("keepass2/f-both.kdbx", key(Some(PASSWORD), true)),
        (
            "keepass2/f-unicode.kdbx",
            key(Some(UNICODE_PASSWORD), false),
        ),
        ("keepassxc/xc-both4.kdbx", key(Some(PASSWORD), true)),
    ]
    .into_iter()
    .map(|(rel, key)| (rel, open(&fixture(rel), &key).unwrap()))
    .collect()
}

fn groups(node: &GroupNode, out: &mut Vec<GroupId>) {
    out.push(node.id);
    for child in &node.children {
        groups(child, out);
    }
}

fn all_groups(vault: &OpenedVault) -> Vec<GroupId> {
    let mut out = Vec::new();
    groups(&vault.group_tree(), &mut out);
    out
}

fn total_entries(node: &GroupNode) -> usize {
    node.entry_count + node.children.iter().map(total_entries).sum::<usize>()
}

#[test]
fn rf_50_tree_of_every_fixture_has_the_root_and_counts_all_entries() {
    for (rel, vault) in fixtures() {
        let tree = vault.group_tree();
        assert_eq!(tree.kind, GroupKind::Root, "{rel}");
        assert_eq!(tree.id, vault.root_id(), "{rel}");
        assert_eq!(tree.name, vault.name(), "{rel}");
        assert_eq!(total_entries(&tree), vault.entry_count(), "{rel}");
        assert_eq!(all_groups(&vault).len(), vault.group_count(), "{rel}");
    }
}

#[test]
fn rf_42_every_fixture_group_lists_its_entries_in_title_order() {
    for (rel, vault) in fixtures() {
        for group in all_groups(&vault) {
            let list = vault.list_entries(group).unwrap();
            let keys: Vec<(String, EntryId)> = list
                .iter()
                .map(|e| (text::fold(e.title.as_deref().unwrap_or("")), e.id))
                .collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "{rel}");
        }
    }
}

#[test]
fn rf_43_rf_44_fixture_details_never_carry_a_value_that_reveal_gives() {
    let mut checked = 0;
    for (rel, vault) in fixtures() {
        for group in all_groups(&vault) {
            for summary in vault.list_entries(group).unwrap() {
                let detail = vault.entry(summary.id).unwrap();
                assert_eq!(detail.summary, summary, "{rel}");
                assert_eq!(detail.group_id, group, "{rel}");
                let shown = format!("{detail:?}");
                for field in &detail.protected_fields {
                    let value = vault.reveal_field(summary.id, field).unwrap();
                    let value = value.expose_secret();
                    assert!(!value.is_empty(), "{rel}: {field} listed without a value");
                    // Short values could appear by chance in a date or a UUID.
                    if text::char_count(value) >= 6 {
                        assert!(!shown.contains(value), "{rel}: {field} leaked");
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "the fixtures have protected passwords");
}

#[test]
fn rf_93_unknown_ids_answer_no_encontrado_on_a_fixture() {
    let (_, vault) = fixtures().remove(0);
    let ghost = "00000000-0000-4000-8000-00000000abcd";
    assert_eq!(
        vault.entry(EntryId::parse(ghost).unwrap()).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        vault
            .list_entries(GroupId::parse(ghost).unwrap())
            .unwrap_err(),
        CoreError::NoEncontrado
    );
}

#[test]
fn created_vault_has_its_name_on_the_root_and_nothing_else() {
    let key = key(Some("Maestra-nueva-2026"), false);
    let bytes = create_vault(
        &key,
        "  Mis cuentas  ",
        NewVaultKdf::with_iterations(2).unwrap(),
    )
    .unwrap();
    let vault = open(&bytes, &key).unwrap();
    assert_eq!(vault.name(), "Mis cuentas");
    let tree = vault.group_tree();
    assert_eq!(tree.name, "Mis cuentas");
    assert_eq!(tree.entry_count, 0);
    assert!(tree.children.is_empty());
    assert_eq!(vault.recycle_bin_id(), None);
    assert!(vault.list_entries(vault.root_id()).unwrap().is_empty());
    assert_eq!(
        create_vault(&key, "   ", NewVaultKdf::with_iterations(2).unwrap()).unwrap_err(),
        CoreError::NombreVacio
    );
}
