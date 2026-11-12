//! Groups, moving, deleting and the recycle bin through the public API (RF-46 to RF-48, RF-51 to RF-58, RF-14,
//! RF-93, examples 3 and 4). Expected values come from the spec.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use almarioks_core::{
    CoreError,
    kdbx::{
        MasterPassword, NewVaultKdf, OpenedVault, RECYCLE_BIN_NAME, VaultKey, create_vault, open,
    },
    vault::{DeletePreview, Deletion, EntryId, GroupId, GroupKind, GroupNode, NewEntry, Timestamp},
};

const PASSWORD: &str = "Maestra-grupos-2026";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn vault() -> OpenedVault {
    let bytes = create_vault(&key(), "Raíz", NewVaultKdf::with_iterations(2).unwrap()).unwrap();
    open(&bytes, &key()).unwrap()
}

fn reopen(vault: &OpenedVault) -> OpenedVault {
    open(&vault.to_bytes(&key()).unwrap(), &key()).unwrap()
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200 + seconds).unwrap()
}

fn entry(vault: &mut OpenedVault, group: GroupId, title: &str) -> EntryId {
    let new = NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    };
    vault.create_entry(group, new, at(0)).unwrap()
}

fn find(node: &GroupNode, id: GroupId) -> Option<&GroupNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, id))
}

fn names(node: &GroupNode) -> Vec<&str> {
    node.children.iter().map(|c| c.name.as_str()).collect()
}

/// Raíz › Trabajo (3 entries) › Clientes (2 entries).
struct Work {
    vault: OpenedVault,
    trabajo: GroupId,
    clientes: GroupId,
}

fn work() -> Work {
    let mut vault = vault();
    let root = vault.root_id();
    let trabajo = vault.create_group(root, "Trabajo", at(0)).unwrap();
    let clientes = vault.create_group(trabajo, "Clientes", at(0)).unwrap();
    for title in ["Correo", "Nómina", "VPN"] {
        entry(&mut vault, trabajo, title);
    }
    for title in ["Cliente A", "Cliente B"] {
        entry(&mut vault, clientes, title);
    }
    Work {
        vault,
        trabajo,
        clientes,
    }
}

#[test]
fn rf_51_rf_52_rf_53_create_and_rename_groups_with_name_rules() {
    let mut vault = vault();
    let root = vault.root_id();
    let id = vault.create_group(root, "  Personal  ", at(0)).unwrap();
    assert_eq!(names(&vault.group_tree()), ["Personal"]);
    assert_eq!(
        find(&vault.group_tree(), id).unwrap().kind,
        GroupKind::Normal
    );
    assert_eq!(
        vault.create_group(root, " \t ", at(0)).unwrap_err(),
        CoreError::NombreVacio
    );
    assert_eq!(
        vault
            .create_group(root, &"g".repeat(256), at(0))
            .unwrap_err(),
        CoreError::CampoMuyLargo
    );
    vault.create_group(root, &"ñ".repeat(255), at(0)).unwrap();
    vault.rename_group(id, "  Casa  ", at(1)).unwrap();
    assert_eq!(find(&vault.group_tree(), id).unwrap().name, "Casa");
    assert_eq!(
        vault.rename_group(id, "   ", at(2)).unwrap_err(),
        CoreError::NombreVacio
    );
    assert_eq!(
        vault.rename_group(id, &"g".repeat(256), at(2)).unwrap_err(),
        CoreError::CampoMuyLargo
    );
    let reopened = reopen(&vault);
    assert_eq!(reopened.group_tree(), vault.group_tree());
}

#[test]
fn rf_55_rf_54_example_3_move_groups() {
    let mut w = work();
    let root = w.vault.root_id();
    // Moving "Trabajo" inside "Clientes" (its descendant) is rejected, and into itself too.
    assert_eq!(
        w.vault
            .move_group(w.trabajo, w.clientes, at(1))
            .unwrap_err(),
        CoreError::MovimientoInvalido
    );
    assert_eq!(
        w.vault.move_group(w.trabajo, w.trabajo, at(1)).unwrap_err(),
        CoreError::MovimientoInvalido
    );
    // Moving "Clientes" to the root: Raíz › Trabajo and Raíz › Clientes, with all its content.
    w.vault.move_group(w.clientes, root, at(1)).unwrap();
    let tree = w.vault.group_tree();
    assert_eq!(names(&tree), ["Trabajo", "Clientes"]);
    assert!(find(&tree, w.trabajo).unwrap().children.is_empty());
    assert_eq!(find(&tree, w.clientes).unwrap().entry_count, 2);
    assert_eq!(reopen(&w.vault).group_tree(), tree);
}

#[test]
fn rf_56_example_4_preview_counts_every_level_and_the_group_goes_to_the_bin() {
    let mut w = work();
    assert_eq!(
        w.vault.delete_preview(w.trabajo).unwrap(),
        DeletePreview {
            entries: 5,
            subgroups: 1
        }
    );
    assert_eq!(
        w.vault.delete_group(w.trabajo, at(1)).unwrap(),
        Deletion::MovedToRecycleBin
    );
    let tree = w.vault.group_tree();
    let bin = w.vault.recycle_bin_id().unwrap();
    let bin_node = find(&tree, bin).unwrap();
    assert_eq!(bin_node.kind, GroupKind::RecycleBin);
    assert_eq!(bin_node.name, RECYCLE_BIN_NAME);
    assert_eq!(names(bin_node), ["Trabajo"]);
    let trabajo = find(bin_node, w.trabajo).unwrap();
    assert_eq!(trabajo.entry_count, 3);
    assert_eq!(find(trabajo, w.clientes).unwrap().entry_count, 2);
    assert_eq!(names(&tree), [RECYCLE_BIN_NAME]);
    assert_eq!(reopen(&w.vault).group_tree(), tree);
}

#[test]
fn rf_58_deleting_a_group_in_the_bin_or_emptying_the_bin_is_for_good() {
    let mut w = work();
    let root = w.vault.root_id();
    let otro = w.vault.create_group(root, "Otro", at(0)).unwrap();
    let suelta = entry(&mut w.vault, root, "Suelta");
    w.vault.delete_group(w.trabajo, at(1)).unwrap();
    w.vault.delete_group(otro, at(1)).unwrap();
    w.vault.delete_entry(suelta, at(1)).unwrap();
    // A group already in the bin: deleting it removes it with its content.
    assert_eq!(
        w.vault.delete_group(w.clientes, at(2)).unwrap(),
        Deletion::Permanent
    );
    assert!(find(&w.vault.group_tree(), w.clientes).is_none());
    assert_eq!(
        w.vault.list_entries(w.clientes).unwrap_err(),
        CoreError::NoEncontrado
    );
    // Emptying: Trabajo (3 entries), Otro (0) and the loose entry.
    assert_eq!(
        w.vault.empty_recycle_bin(at(3)).unwrap(),
        DeletePreview {
            entries: 4,
            subgroups: 2
        }
    );
    let bin = w.vault.recycle_bin_id().unwrap();
    let tree = w.vault.group_tree();
    let bin_node = find(&tree, bin).unwrap();
    assert!(bin_node.children.is_empty());
    assert_eq!(bin_node.entry_count, 0);
    assert_eq!(w.vault.entry_count(), 0);
    assert_eq!(w.vault.entry(suelta).unwrap_err(), CoreError::NoEncontrado);
    assert_eq!(reopen(&w.vault).recycle_bin_id(), Some(bin));
}

#[test]
fn rf_46_rf_47_rf_48_move_an_entry_then_delete_it_twice() {
    let mut w = work();
    let root = w.vault.root_id();
    let id = w.vault.list_entries(w.trabajo).unwrap()[0].id;
    w.vault.move_entry(id, root, at(1)).unwrap();
    assert_eq!(w.vault.list_entries(w.trabajo).unwrap().len(), 2);
    let in_root: Vec<EntryId> = w
        .vault
        .list_entries(root)
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(in_root, [id]);
    assert_eq!(w.vault.entry(id).unwrap().group_id, root);
    // Moving does not change the modification date (it is not an edit).
    assert_eq!(w.vault.entry(id).unwrap().summary.modified, at(0));

    assert_eq!(w.vault.recycle_bin_id(), None);
    assert_eq!(
        w.vault.delete_entry(id, at(2)).unwrap(),
        Deletion::MovedToRecycleBin
    );
    let bin = w.vault.recycle_bin_id().unwrap();
    assert_eq!(w.vault.entry(id).unwrap().group_id, bin);
    assert!(w.vault.list_entries(root).unwrap().is_empty());
    // The same bin is used again.
    let other = w.vault.list_entries(w.trabajo).unwrap()[0].id;
    w.vault.delete_entry(other, at(2)).unwrap();
    assert_eq!(w.vault.recycle_bin_id(), Some(bin));
    assert_eq!(w.vault.list_entries(bin).unwrap().len(), 2);

    assert_eq!(
        w.vault.delete_entry(id, at(3)).unwrap(),
        Deletion::Permanent
    );
    assert_eq!(w.vault.entry(id).unwrap_err(), CoreError::NoEncontrado);
    assert_eq!(w.vault.list_entries(bin).unwrap().len(), 1);
}

#[test]
fn rf_57_root_and_recycle_bin_are_protected() {
    let mut w = work();
    let root = w.vault.root_id();
    assert_eq!(
        w.vault.move_group(root, w.trabajo, at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        w.vault.delete_group(root, at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        w.vault.delete_preview(root).unwrap_err(),
        CoreError::GrupoProtegido
    );
    let suelta = entry(&mut w.vault, root, "Suelta");
    w.vault.delete_entry(suelta, at(1)).unwrap();
    let bin = w.vault.recycle_bin_id().unwrap();
    assert_eq!(
        w.vault.move_group(bin, w.trabajo, at(2)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        w.vault.rename_group(bin, "Basura", at(2)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        w.vault.delete_group(bin, at(2)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    // The root holds the bin: by the letter of RF-57 it cannot be renamed while a bin exists (decision pending).
    assert_eq!(
        w.vault.rename_group(root, "Otra", at(2)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    // Groups that do not hold the bin are free.
    w.vault.rename_group(w.trabajo, "Oficina", at(2)).unwrap();
}

#[test]
fn rf_93_unknown_groups_and_entries_answer_no_encontrado() {
    let mut w = work();
    let ghost = GroupId::parse("00000000-0000-4000-8000-0000000000aa").unwrap();
    let ghost_entry = EntryId::parse("00000000-0000-4000-8000-0000000000bb").unwrap();
    let root = w.vault.root_id();
    assert_eq!(
        w.vault.create_group(ghost, "   ", at(0)).unwrap_err(),
        CoreError::NoEncontrado,
        "NoEncontrado before NombreVacio"
    );
    assert_eq!(
        w.vault.rename_group(ghost, "x", at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.move_group(ghost, root, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.move_group(w.trabajo, ghost, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.delete_preview(ghost).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.delete_group(ghost, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.move_entry(ghost_entry, root, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    let real = w.vault.list_entries(w.trabajo).unwrap()[0].id;
    assert_eq!(
        w.vault.move_entry(real, ghost, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        w.vault.delete_entry(ghost_entry, at(0)).unwrap_err(),
        CoreError::NoEncontrado
    );
}

#[test]
fn rf_14_group_and_delete_operations_on_kdbx3_are_solo_lectura() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keepass2/v3-aes.kdbx");
    let key = VaultKey::new(Some(MasterPassword::new("lab-Maestra-2026".into())), None).unwrap();
    let mut vault = open(&fs::read(path).unwrap(), &key).unwrap();
    let root = vault.root_id();
    let id = vault.list_entries(root).unwrap()[0].id;
    let ghost = GroupId::parse("00000000-0000-4000-8000-0000000000cc").unwrap();
    let results = [
        vault.create_group(root, "x", at(0)).map(|_| ()),
        vault.rename_group(root, "x", at(0)),
        vault.move_group(ghost, root, at(0)),
        vault.delete_group(ghost, at(0)).map(|_| ()),
        vault.move_entry(id, root, at(0)),
        vault.delete_entry(id, at(0)).map(|_| ()),
        vault.empty_recycle_bin(at(0)).map(|_| ()),
    ];
    for result in results {
        assert_eq!(result.unwrap_err(), CoreError::SoloLectura);
    }
}
