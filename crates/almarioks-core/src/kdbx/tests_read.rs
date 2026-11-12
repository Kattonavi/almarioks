//! Read model tests that need a vault the app cannot make: protected notes and title, custom fields, attachments
//! and fixed UUIDs (RF-42 to RF-44, RF-50, RF-93, spec "Campo protegido"). The vault is built with the library,
//! saved to bytes and opened through our `open`, as a KeePass file would be.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use keepass::{
    Database,
    db::{EntryId as KpEntryId, Value, fields},
};
use secrecy::ExposeSecret;
use uuid::Uuid;

use super::{
    MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault,
    open::{self},
};
use crate::{
    error::CoreError,
    vault::{EntryId, GroupId, GroupKind, GroupNode},
};

const PASSWORD: &str = "pruebas-lectura-2026";
/// Appears only in protected values: no `Debug`, summary or detail may contain it.
const CANARY: &str = "CANARIO-7f3a9c";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

/// A database with the KDF of a new vault (2 iterations, fast), ready to fill with the library.
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

fn uuid(n: u8) -> Uuid {
    Uuid::from_bytes([n; 16])
}

/// Root: "Trabajo" (entries: one with protected notes, one with everything protected), "Vacío", and a recycle bin.
struct Fixture {
    vault: OpenedVault,
    trabajo: GroupId,
    vacio: GroupId,
    bin: GroupId,
    notes_entry: EntryId,
    all_protected: EntryId,
}

fn fixture() -> Fixture {
    let mut db = empty_db();
    let mut root = db.root_mut();
    let trabajo = root.add_group().edit(|g| g.name = "Trabajo".into()).id();
    let vacio = root.add_group().edit(|g| g.name = "Vacío".into()).id();
    let bin = root.add_group().edit(|g| g.name = "Papelera".into()).id();
    let mut group = db.group_mut(trabajo).unwrap();
    let notes_entry = group
        .add_entry_with_id(KpEntryId::from_uuid(uuid(1)))
        .unwrap()
        .edit(|e| {
            e.set_unprotected(fields::TITLE, "Banco");
            e.set_unprotected(fields::USERNAME, "ana");
            e.set_protected(fields::PASSWORD, format!("{CANARY}-password"));
            e.set_unprotected(fields::URL, "https://banco.example");
            e.set_protected(fields::NOTES, format!("{CANARY}-notes"));
            e.set_unprotected("Número de cliente", "12345");
            e.set_protected("PIN", format!("{CANARY}-pin"));
            e.set_protected("Vacío protegido", "");
            e.tags = vec!["finanzas".into(), "casa".into()];
            e.add_attachment("contrato.pdf", Value::unprotected(vec![7u8; 42]));
        })
        .id();
    let all_protected = group
        .add_entry_with_id(KpEntryId::from_uuid(uuid(2)))
        .unwrap()
        .edit(|e| {
            for name in [
                fields::TITLE,
                fields::USERNAME,
                fields::PASSWORD,
                fields::URL,
                fields::NOTES,
            ] {
                e.set_protected(name, format!("{CANARY}-{name}"));
            }
        })
        .id();
    db.meta.recyclebin_uuid = Some(bin.uuid());
    db.meta.recyclebin_enabled = Some(true);
    Fixture {
        vault: reopen(&db),
        trabajo: GroupId::from_uuid(trabajo.uuid()),
        vacio: GroupId::from_uuid(vacio.uuid()),
        bin: GroupId::from_uuid(bin.uuid()),
        notes_entry: EntryId::from_uuid(notes_entry.uuid()),
        all_protected: EntryId::from_uuid(all_protected.uuid()),
    }
}

fn find(node: &GroupNode, id: GroupId) -> Option<&GroupNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, id))
}

#[test]
fn rf_43_detail_shows_plain_fields_and_names_but_no_protected_value() {
    let f = fixture();
    let detail = f.vault.entry(f.notes_entry).unwrap();
    assert_eq!(detail.summary.title.as_deref(), Some("Banco"));
    assert_eq!(detail.summary.username.as_deref(), Some("ana"));
    assert_eq!(detail.summary.url.as_deref(), Some("https://banco.example"));
    assert_eq!(detail.summary.tags, ["finanzas", "casa"]);
    assert_eq!(detail.group_id, f.trabajo);
    // Notes protected in the file: they do not travel (spec "Casos límite", D8).
    assert_eq!(detail.notes, None);
    // Which protected fields have a value: standard first, then custom; the empty protected one is not listed.
    assert_eq!(detail.protected_fields, ["Password", "Notes", "PIN"]);
    let custom: Vec<(&str, bool, Option<&str>)> = detail
        .custom_fields
        .iter()
        .map(|c| (c.name.as_str(), c.protected, c.value.as_deref()))
        .collect();
    assert_eq!(
        custom,
        [
            ("Número de cliente", false, Some("12345")),
            ("PIN", true, None),
            ("Vacío protegido", true, None),
        ]
    );
    assert_eq!(detail.attachments.len(), 1);
    assert_eq!(detail.attachments[0].name, "contrato.pdf");
    assert_eq!(detail.attachments[0].size, 42);
}

#[test]
fn rf_43_a_protected_standard_field_is_none_in_summary_and_detail() {
    let f = fixture();
    let detail = f.vault.entry(f.all_protected).unwrap();
    assert_eq!(detail.summary.title, None);
    assert_eq!(detail.summary.username, None);
    assert_eq!(detail.summary.url, None);
    assert_eq!(detail.notes, None);
    assert_eq!(
        detail.protected_fields,
        ["Title", "UserName", "Password", "URL", "Notes"]
    );
    let list = f.vault.list_entries(f.trabajo).unwrap();
    let summary = list.iter().find(|s| s.id == f.all_protected).unwrap();
    assert_eq!(summary.title, None);
}

#[test]
fn rf_43_canary_no_debug_summary_or_detail_contains_a_protected_value() {
    let f = fixture();
    let mut seen = vec![
        format!("{:?}", f.vault),
        format!("{:?}", f.vault.group_tree()),
    ];
    for group in [f.trabajo, f.vacio, f.bin, f.vault.root_id()] {
        seen.push(format!("{:?}", f.vault.list_entries(group).unwrap()));
    }
    for entry in [f.notes_entry, f.all_protected] {
        seen.push(format!("{:?}", f.vault.entry(entry).unwrap()));
        seen.push(format!(
            "{:?}",
            f.vault.reveal_field(entry, "Password").unwrap()
        ));
    }
    for text in seen {
        assert!(!text.contains(CANARY), "a protected value leaked: {text}");
    }
}

#[test]
fn rf_44_reveal_gives_only_that_field_of_that_entry() {
    let f = fixture();
    let notes = f.vault.reveal_field(f.notes_entry, "Notes").unwrap();
    assert_eq!(notes.expose_secret(), format!("{CANARY}-notes"));
    let pin = f.vault.reveal_field(f.notes_entry, "PIN").unwrap();
    assert_eq!(pin.expose_secret(), format!("{CANARY}-pin"));
    let title = f.vault.reveal_field(f.all_protected, "Title").unwrap();
    assert_eq!(title.expose_secret(), format!("{CANARY}-Title"));
    assert_eq!(
        f.vault
            .reveal_field(f.notes_entry, "No existe")
            .unwrap_err(),
        CoreError::NoEncontrado
    );
}

#[test]
fn rf_50_tree_counts_direct_entries_and_marks_root_and_recycle_bin() {
    let f = fixture();
    let tree = f.vault.group_tree();
    assert_eq!(tree.kind, GroupKind::Root);
    assert_eq!(tree.name, "Raíz");
    assert_eq!(tree.entry_count, 0);
    let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Trabajo", "Vacío", "Papelera"]);
    assert_eq!(find(&tree, f.trabajo).unwrap().entry_count, 2);
    assert_eq!(find(&tree, f.trabajo).unwrap().kind, GroupKind::Normal);
    assert_eq!(find(&tree, f.vacio).unwrap().entry_count, 0);
    assert_eq!(find(&tree, f.bin).unwrap().kind, GroupKind::RecycleBin);
    assert_eq!(f.vault.recycle_bin_id(), Some(f.bin));
}

#[test]
fn rf_50_entry_count_does_not_include_subgroups() {
    let mut db = empty_db();
    let mut root = db.root_mut();
    root.add_entry();
    let parent = root
        .add_group()
        .edit(|g| g.name = "Padre".into())
        .add_group()
        .edit(|g| g.name = "Hijo".into())
        .add_entry()
        .parent_mut()
        .parent_mut()
        .unwrap()
        .id();
    let vault = reopen(&db);
    let tree = vault.group_tree();
    assert_eq!(tree.entry_count, 1);
    let padre = find(&tree, GroupId::from_uuid(parent.uuid())).unwrap();
    assert_eq!(padre.entry_count, 0);
    assert_eq!(padre.children[0].entry_count, 1);
}

#[test]
fn rf_42_list_is_in_title_order_folded_with_uuid_tie_break() {
    let mut db = empty_db();
    let mut root = db.root_mut();
    // Inserted out of order; two titles fold the same and the UUID decides between them.
    for (n, title) in [
        (9, "cancion"),
        (5, "banco"),
        (3, "Canción"),
        (7, "Árbol"),
        (1, "Zeta"),
    ] {
        root.add_entry_with_id(KpEntryId::from_uuid(uuid(n)))
            .unwrap()
            .set_unprotected(fields::TITLE, title);
    }
    let vault = reopen(&db);
    let titles: Vec<String> = vault
        .list_entries(vault.root_id())
        .unwrap()
        .into_iter()
        .map(|s| s.title.unwrap())
        .collect();
    assert_eq!(titles, ["Árbol", "banco", "Canción", "cancion", "Zeta"]);
}

#[test]
fn rf_93_unknown_ids_answer_no_encontrado() {
    let f = fixture();
    let ghost_entry = EntryId::from_uuid(uuid(200));
    let ghost_group = GroupId::from_uuid(uuid(201));
    assert_eq!(
        f.vault.entry(ghost_entry).unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        f.vault.reveal_field(ghost_entry, "Password").unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        f.vault.list_entries(ghost_group).unwrap_err(),
        CoreError::NoEncontrado
    );
    // An entry id is not a group id, nor the other way round.
    assert_eq!(
        f.vault
            .list_entries(GroupId::from_uuid(f.notes_entry.uuid()))
            .unwrap_err(),
        CoreError::NoEncontrado
    );
    assert_eq!(
        f.vault
            .entry(EntryId::from_uuid(f.trabajo.uuid()))
            .unwrap_err(),
        CoreError::NoEncontrado
    );
}

// ---------- RF-105: the list learns whether there is a password, and nothing else ----------

/// One entry per way a password can be in a file, all in the root group; the title says which.
fn password_cases() -> OpenedVault {
    let mut db = empty_db();
    let mut root = db.root_mut();
    let mut add = |n: u8, title: &str, password: Option<Value<String>>| {
        root.add_entry_with_id(KpEntryId::from_uuid(uuid(n)))
            .unwrap()
            .edit(|e| {
                e.set_unprotected(fields::TITLE, title);
                e.set_unprotected(fields::USERNAME, "ana");
                if let Some(password) = password {
                    e.fields.insert(fields::PASSWORD.to_owned(), password);
                }
            });
    };
    let secret = |text: &str| Some(Value::protected(text.to_owned()));
    let plain = |text: &str| Some(Value::unprotected(text.to_owned()));
    add(1, "caso protegida", secret(&format!("{CANARY}-corta")));
    add(
        2,
        "caso protegida larga",
        secret(&format!("{CANARY}-una-mucho-mas-larga-que-la-otra")),
    );
    add(3, "caso sin proteger", plain(&format!("{CANARY}-llana")));
    add(4, "caso solo espacios", secret(" "));
    add(5, "caso vacia protegida", secret(""));
    add(6, "caso vacia sin proteger", plain(""));
    add(7, "caso sin el campo", None);
    reopen(&db)
}

const WITH_PASSWORD: [&str; 4] = [
    "caso protegida",
    "caso protegida larga",
    "caso sin proteger",
    "caso solo espacios",
];
const WITHOUT_PASSWORD: [&str; 3] = [
    "caso vacia protegida",
    "caso vacia sin proteger",
    "caso sin el campo",
];

#[test]
fn rf_105_the_summary_says_whether_the_entry_has_a_password() {
    let vault = password_cases();
    let list = vault.list_entries(vault.root_id()).unwrap();
    assert_eq!(list.len(), WITH_PASSWORD.len() + WITHOUT_PASSWORD.len());
    for summary in &list {
        let title = summary.title.as_deref().unwrap();
        assert_eq!(
            summary.has_password,
            WITH_PASSWORD.contains(&title),
            "{title}"
        );
        assert_eq!(WITHOUT_PASSWORD.contains(&title), !summary.has_password);
        // The detail carries the same summary.
        assert_eq!(&vault.entry(summary.id).unwrap().summary, summary);
    }
}

#[test]
fn rf_105_search_results_say_it_too() {
    let vault = password_cases();
    let crate::search::SearchResult::Matches(hits) = vault.search("caso") else {
        panic!("the query filters");
    };
    assert_eq!(hits.len(), WITH_PASSWORD.len() + WITHOUT_PASSWORD.len());
    for hit in &hits {
        let title = hit.summary.title.as_deref().unwrap();
        assert_eq!(
            hit.summary.has_password,
            WITH_PASSWORD.contains(&title),
            "{title}"
        );
    }
}

#[test]
fn rf_105_the_summary_carries_nothing_taken_from_the_password() {
    let vault = password_cases();
    let list = vault.list_entries(vault.root_id()).unwrap();
    let crate::search::SearchResult::Matches(hits) = vault.search("caso") else {
        panic!("the query filters");
    };
    let seen = format!("{list:?} {hits:?}");
    assert!(!seen.contains(CANARY), "a password leaked: {seen}");

    // Two entries that differ only in their passwords (one much longer) give the same summary but for what
    // names the entry: no length, no hash, nothing derived.
    let by_id = |n: u8| {
        let id = EntryId::from_uuid(uuid(n));
        list.iter().find(|summary| summary.id == id).unwrap()
    };
    let (short, long) = (by_id(1), by_id(2));
    let mut renamed = short.clone();
    renamed.id = long.id;
    renamed.title.clone_from(&long.title);
    renamed.modified = long.modified;
    assert_eq!(&renamed, long);
}
