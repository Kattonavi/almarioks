//! Search over what the app cannot write: protected title and notes from another app, and a recycle bin inside
//! another group (RF-61, RF-62, spec "Campo protegido").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use keepass::{
    Database,
    db::{EntryId as KpEntryId, fields},
};
use uuid::Uuid;

use super::{MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open};
use crate::{
    search::{SearchHit, SearchResult},
    vault::EntryId,
};

const PASSWORD: &str = "pruebas-busqueda-2026";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn empty_db() -> Database {
    let bytes = create_vault(&key(), "Raíz", NewVaultKdf::with_iterations(2).unwrap()).unwrap();
    open::open(&bytes, &key()).unwrap().db
}

fn reopen(db: &Database) -> OpenedVault {
    let mut bytes = Vec::new();
    db.save(&mut bytes, key().to_database_key().unwrap())
        .unwrap();
    open::open(&bytes, &key()).unwrap()
}

fn id(n: u8) -> EntryId {
    EntryId::from_uuid(Uuid::from_bytes([n; 16]))
}

fn hits(vault: &OpenedVault, query: &str) -> Vec<SearchHit> {
    match vault.search(query) {
        SearchResult::Matches(hits) => hits,
        SearchResult::NoFilter => panic!("{query:?} should filter"),
    }
}

fn ids(hits: &[SearchHit]) -> Vec<EntryId> {
    hits.iter().map(|h| h.summary.id).collect()
}

#[test]
fn rf_61_protected_fields_are_never_searched() {
    let mut db = empty_db();
    let mut root = db.root_mut();
    root.add_entry_with_id(KpEntryId::from_uuid(Uuid::from_bytes([1; 16])))
        .unwrap()
        .edit(|e| {
            e.set_unprotected(fields::TITLE, "Banco");
            e.set_protected(fields::NOTES, "clave del cajero zafiro");
            e.set_protected(fields::PASSWORD, "rubi-secreto");
            e.set_protected("PIN", "esmeralda");
            e.set_unprotected("Propio", "turquesa");
        });
    root.add_entry_with_id(KpEntryId::from_uuid(Uuid::from_bytes([2; 16])))
        .unwrap()
        .edit(|e| {
            e.set_protected(fields::TITLE, "Zafiro oculto");
            e.set_unprotected(fields::USERNAME, "ana");
            e.set_protected(fields::URL, "https://zafiro.example");
        });
    let vault = reopen(&db);
    // Protected notes, password, title, URL and custom field: nothing found.
    for query in ["zafiro", "rubi", "esmeralda", "oculto"] {
        assert!(hits(&vault, query).is_empty(), "{query:?}");
    }
    // Custom fields are not among the searched fields (RF-59), protected or not.
    assert!(hits(&vault, "turquesa").is_empty());
    // The entry with a protected title is found by its visible user name, and its title stays hidden.
    let by_user = hits(&vault, "ana");
    assert_eq!(ids(&by_user), [id(2)]);
    assert_eq!(by_user[0].summary.title, None);
    assert_eq!(ids(&hits(&vault, "banco")), [id(1)]);
}

#[test]
fn rf_62_a_bin_inside_another_group_is_skipped_at_any_depth() {
    let mut db = empty_db();
    let mut root = db.root_mut();
    let archivo = root.add_group().edit(|g| g.name = "Archivo".into()).id();
    let mut archivo_group = db.group_mut(archivo).unwrap();
    archivo_group
        .add_entry_with_id(KpEntryId::from_uuid(Uuid::from_bytes([3; 16])))
        .unwrap()
        .edit(|e| e.set_unprotected(fields::TITLE, "Cuenta viva"));
    let bin = archivo_group
        .add_group()
        .edit(|g| g.name = "Papelera".into())
        .id();
    let mut bin_group = db.group_mut(bin).unwrap();
    let deep = bin_group.add_group().edit(|g| g.name = "Vieja".into()).id();
    bin_group
        .add_entry_with_id(KpEntryId::from_uuid(Uuid::from_bytes([4; 16])))
        .unwrap()
        .edit(|e| e.set_unprotected(fields::TITLE, "Cuenta borrada"));
    db.group_mut(deep)
        .unwrap()
        .add_entry_with_id(KpEntryId::from_uuid(Uuid::from_bytes([5; 16])))
        .unwrap()
        .edit(|e| e.set_unprotected(fields::TITLE, "Cuenta muy borrada"));
    db.meta.recyclebin_uuid = Some(bin.uuid());
    db.meta.recyclebin_enabled = Some(true);
    let vault = reopen(&db);
    let found = hits(&vault, "cuenta");
    assert_eq!(ids(&found), [id(3)]);
    assert_eq!(found[0].group_path, ["Raíz", "Archivo"]);
}
