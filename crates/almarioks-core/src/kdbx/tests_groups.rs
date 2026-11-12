//! Moving and deleting as the file sees it: history untouched by a move, the recycle bin in the metadata, the
//! deleted objects with the injected time, and a bin inside another group (RF-46 to RF-48, RF-57, RF-58).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use keepass::db::{EntryId as KpEntryId, Icon, fields};

use super::{MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open};
use crate::{
    error::CoreError,
    vault::{Deletion, EntryId, EntryPatch, GroupId, NewEntry, Timestamp},
};

const PASSWORD: &str = "pruebas-grupos-2026";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn vault() -> OpenedVault {
    let bytes = create_vault(&key(), "Raíz", NewVaultKdf::with_iterations(2).unwrap()).unwrap();
    open::open(&bytes, &key()).unwrap()
}

fn reopen(vault: &OpenedVault) -> OpenedVault {
    open::open(&vault.to_bytes(&key()).unwrap(), &key()).unwrap()
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200 + seconds).unwrap()
}

fn naive(time: Timestamp) -> chrono::NaiveDateTime {
    time.datetime().naive_utc()
}

fn new_entry(vault: &mut OpenedVault, group: GroupId, title: &str) -> EntryId {
    let entry = NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    };
    vault.create_entry(group, entry, at(0)).unwrap()
}

fn history_len(vault: &OpenedVault, id: EntryId) -> usize {
    let entry = vault.db.entry(KpEntryId::from_uuid(id.uuid())).unwrap();
    entry.history.as_ref().map_or(0, |h| h.get_entries().len())
}

#[test]
fn rf_46_moving_an_entry_adds_no_history_and_records_the_move() {
    let mut vault = vault();
    let root = vault.root_id();
    let destino = vault.create_group(root, "Destino", at(0)).unwrap();
    let id = new_entry(&mut vault, root, "GitHub");
    let titled = EntryPatch {
        title: Some("GitHub 2".into()),
        ..EntryPatch::default()
    };
    vault.update_entry(id, titled, at(1)).unwrap();
    assert_eq!(history_len(&vault, id), 1);
    vault.move_entry(id, destino, at(5)).unwrap();
    let reopened = reopen(&vault);
    assert_eq!(history_len(&reopened, id), 1);
    let entry = reopened.db.entry(KpEntryId::from_uuid(id.uuid())).unwrap();
    assert_eq!(entry.times.location_changed, Some(naive(at(5))));
    assert_eq!(entry.times.last_modification, Some(naive(at(1))));
    assert_eq!(entry.parent().id().uuid(), destino.uuid());
    assert_eq!(
        entry.previous_parent().map(|g| g.id().uuid()),
        Some(root.uuid())
    );
}

#[test]
fn rf_47_the_first_delete_creates_and_enables_the_bin_like_keepass() {
    let mut vault = vault();
    let root = vault.root_id();
    let id = new_entry(&mut vault, root, "GitHub");
    vault.delete_entry(id, at(7)).unwrap();
    let reopened = reopen(&vault);
    let bin = reopened.db.recycle_bin().unwrap();
    assert_eq!(reopened.db.meta.recyclebin_enabled, Some(true));
    assert_eq!(reopened.db.meta.recyclebin_changed, Some(naive(at(7))));
    assert_eq!(bin.icon(), Some(&Icon::BuiltIn(43)));
    assert_eq!(bin.enable_searching, Some(false));
    assert_eq!(bin.enable_autotype, Some(false));
    assert_eq!(bin.parent().unwrap().id(), reopened.db.root().id());
    assert_eq!(history_len(&reopened, id), 0, "a delete is not an edit");
    assert!(reopened.db.deleted_objects.is_empty());
}

#[test]
fn rf_47_a_bin_the_metadata_disables_is_enabled_and_reused() {
    let mut vault = vault();
    let root = vault.root_id();
    let old_bin = vault.create_group(root, "Papelera vieja", at(0)).unwrap();
    vault.db.meta.recyclebin_uuid = Some(old_bin.uuid());
    vault.db.meta.recyclebin_enabled = Some(false);
    let id = new_entry(&mut vault, root, "GitHub");
    vault.delete_entry(id, at(1)).unwrap();
    assert_eq!(vault.recycle_bin_id(), Some(old_bin));
    assert_eq!(vault.db.meta.recyclebin_enabled, Some(true));
    assert_eq!(vault.entry(id).unwrap().group_id, old_bin);
}

#[test]
fn rf_48_rf_58_permanent_deletes_are_written_down_with_the_injected_time() {
    let mut vault = vault();
    let root = vault.root_id();
    let trabajo = vault.create_group(root, "Trabajo", at(0)).unwrap();
    let clientes = vault.create_group(trabajo, "Clientes", at(0)).unwrap();
    let a = new_entry(&mut vault, trabajo, "A");
    let b = new_entry(&mut vault, clientes, "B");
    let suelta = new_entry(&mut vault, root, "Suelta");
    let otra = new_entry(&mut vault, root, "Otra");

    vault.delete_entry(suelta, at(1)).unwrap();
    assert_eq!(
        vault.delete_entry(suelta, at(2)).unwrap(),
        Deletion::Permanent
    );
    vault.delete_group(trabajo, at(3)).unwrap();
    assert_eq!(
        vault.delete_group(trabajo, at(4)).unwrap(),
        Deletion::Permanent
    );
    vault.delete_entry(otra, at(5)).unwrap();
    vault.empty_recycle_bin(at(6)).unwrap();

    let reopened = reopen(&vault);
    let deleted = &reopened.db.deleted_objects;
    assert_eq!(deleted.get(&suelta.uuid()), Some(&Some(naive(at(2)))));
    for uuid in [trabajo.uuid(), clientes.uuid(), a.uuid(), b.uuid()] {
        assert_eq!(deleted.get(&uuid), Some(&Some(naive(at(4)))));
    }
    assert_eq!(deleted.get(&otra.uuid()), Some(&Some(naive(at(6)))));
    assert_eq!(deleted.len(), 6);
    // The bin itself is never removed and the metadata still points at it.
    let bin = reopened.db.recycle_bin().unwrap();
    assert!(!deleted.contains_key(&bin.id().uuid()));
}

#[test]
fn rf_57_a_group_that_holds_the_bin_cannot_be_deleted_but_can_be_renamed_and_moved() {
    let mut vault = vault();
    let root = vault.root_id();
    let archivo = vault.create_group(root, "Archivo", at(0)).unwrap();
    let otro = vault.create_group(root, "Otro", at(0)).unwrap();
    // A KeePass vault whose bin is inside another group.
    let bin = vault.create_group(archivo, "Papelera", at(0)).unwrap();
    vault.db.meta.recyclebin_uuid = Some(bin.uuid());
    vault.db.meta.recyclebin_enabled = Some(true);
    // It cannot be deleted, and the preview says so (plan "Precisiones" 17).
    assert_eq!(
        vault.delete_group(archivo, at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        vault.delete_preview(archivo).unwrap_err(),
        CoreError::GrupoProtegido
    );
    // Moving it inside its own bin is still a move into a descendant (RF-55).
    assert_eq!(
        vault.move_group(archivo, bin, at(1)).unwrap_err(),
        CoreError::MovimientoInvalido
    );
    // Renaming and moving it are allowed; the bin goes along and stays the bin.
    vault.rename_group(archivo, "Viejo", at(1)).unwrap();
    vault.move_group(archivo, otro, at(1)).unwrap();
    let moved = vault.db.group(super::read::kp_group(archivo)).unwrap();
    assert_eq!(moved.name, "Viejo");
    assert_eq!(
        moved.parent().map(|p| p.id()),
        Some(super::read::kp_group(otro))
    );
    assert_eq!(vault.recycle_bin_id(), Some(bin));
    // The bin itself still cannot be moved, renamed or deleted.
    assert_eq!(
        vault.move_group(bin, root, at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        vault.rename_group(bin, "Basura", at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    assert_eq!(
        vault.delete_group(bin, at(1)).unwrap_err(),
        CoreError::GrupoProtegido
    );
    // Deleting an entry uses that bin; no second bin is created.
    let id = new_entry(&mut vault, otro, "x");
    vault.delete_entry(id, at(2)).unwrap();
    assert_eq!(vault.entry(id).unwrap().group_id, bin);
    // Root, Archivo, Otro and the bin: no second bin.
    assert_eq!(vault.group_count(), 4);
}

#[test]
fn rf_47_rf_25_keepass_fixture_without_bin_keeps_its_deleted_objects() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keepass2/f-pass.kdbx");
    let key = VaultKey::new(Some(MasterPassword::new("lab-Maestra-2026".into())), None).unwrap();
    let mut vault = open::open(&fs::read(path).unwrap(), &key).unwrap();
    assert_eq!(
        vault.recycle_bin_id(),
        None,
        "lab P4 c: RecycleBinUUID is zero"
    );
    let before = vault.db.deleted_objects.clone();
    let id = vault
        .db
        .iter_all_entries()
        .find(|e| e.get(fields::TITLE).is_some())
        .map(|e| EntryId::from_uuid(e.id().uuid()))
        .unwrap();
    let history = history_len(&vault, id);
    assert_eq!(
        vault.delete_entry(id, at(1)).unwrap(),
        Deletion::MovedToRecycleBin
    );
    let reopened = open::open(&vault.to_bytes(&key).unwrap(), &key).unwrap();
    assert_eq!(reopened.db.deleted_objects, before);
    assert_eq!(history_len(&reopened, id), history);
    let bin = reopened.recycle_bin_id().unwrap();
    assert_eq!(reopened.entry(id).unwrap().group_id, bin);
}
