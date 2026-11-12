//! RF-25 and constitution rule 5 after an EDIT: open each fixture of KeePass 2 and the one of KeePassXC, change the
//! title of one entry through `OpenedVault::update_entry`, save with `to_bytes`, reopen, and compare the whole
//! database with the original. The only differences allowed are the ones the edit makes (RF-26, RF-45): that
//! entry's title, its modification time and one more history version, the previous state. Everything else
//! (other entries, attachments with their bytes, history of the others, custom fields, icons, custom data of
//! entries, groups and the database, deleted objects, metadata, format version) must be equal, so any loss fails.
//!
//! All fixtures are KDBX 4.1: a 4.0 vault is not covered here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use keepass::{
    Database,
    db::{Entry, EntryId as KpEntryId, Group, GroupId as KpGroupId, fields},
};

use super::{FormatVersion, KeyFile, MasterPassword, VaultKey, open};
use crate::vault::{EntryId, EntryPatch, Timestamp};

const PASSWORD: &str = "lab-Maestra-2026";
const UNICODE_PASSWORD: &str = "ñandú-€-2026";
const NEW_TITLE: &str = "Título editado aquí";

fn fixture(rel: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(rel),
    )
    .unwrap()
}

fn key(password: Option<&str>, key_file: bool) -> VaultKey {
    VaultKey::new(
        password.map(|p| MasterPassword::new(p.to_owned())),
        key_file.then(|| KeyFile::from_bytes(fixture("keepass2/llave.keyx")).unwrap()),
    )
    .unwrap()
}

fn now() -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200).unwrap()
}

/// Every entry of the database (history versions not included), in a stable order.
fn all_entries(db: &Database) -> Vec<KpEntryId> {
    let mut entries = Vec::new();
    let mut pending = vec![db.root().id()];
    while let Some(id) = pending.pop() {
        let group = db.group(id).unwrap();
        entries.extend(group.entry_ids());
        pending.extend(group.group_ids());
    }
    entries
}

/// The entry to edit: the one with the most history versions (ties: the most attachments), so the edit lands where
/// the most can be lost.
fn richest_entry(db: &Database) -> KpEntryId {
    all_entries(db)
        .into_iter()
        .max_by_key(|id| {
            let entry = db.entry(*id).unwrap();
            let versions = entry.history.as_ref().map_or(0, |h| h.get_entries().len());
            (versions, entry.attachments().count())
        })
        .unwrap()
}

fn edit_one_title_and_compare(rel: &str, key: &VaultKey) {
    let bytes = fixture(rel);
    let mut vault = open::open(&bytes, key).unwrap();
    let version = vault.version();
    assert_eq!(version, FormatVersion { major: 4, minor: 1 }, "{rel}");
    let original = vault.db.clone();
    let target = richest_entry(&original);
    let id = EntryId::from_uuid(target.uuid());

    let patch = EntryPatch {
        title: Some(NEW_TITLE.to_owned()),
        ..EntryPatch::default()
    };
    assert!(
        vault.update_entry(id, patch, now()).unwrap(),
        "{rel}: the edit changed nothing"
    );
    let reopened = open::open(&vault.to_bytes(key).unwrap(), key).unwrap();
    assert_eq!(reopened.version(), version, "{rel}: format version changed");

    let before: &Entry = &original.entry(target).unwrap();
    let after: Entry = Entry::clone(&reopened.db.entry(target).unwrap());

    // The edited entry: new title with the same protection flag, every other field as it was.
    let title = after.fields.get(fields::TITLE).unwrap();
    assert_eq!(title.get(), NEW_TITLE, "{rel}");
    assert_eq!(
        title.is_protected(),
        before
            .fields
            .get(fields::TITLE)
            .is_some_and(|v| v.is_protected()),
        "{rel}: title protection flag"
    );
    let without_title = |entry: &Entry| {
        let mut fields = entry.fields.clone();
        fields.remove(fields::TITLE);
        fields
    };
    assert_eq!(
        without_title(&after),
        without_title(before),
        "{rel}: other fields"
    );
    // Times: only the modification time is the edit's.
    let mut times = before.times.clone();
    times.last_modification = Some(now().datetime().naive_utc());
    assert_eq!(after.times, times, "{rel}: times");
    // History: the old versions, oldest first, then the previous state (RF-26).
    let old: Vec<Entry> = before
        .history
        .as_ref()
        .map(|h| h.get_entries().clone())
        .unwrap_or_default();
    let new: Vec<Entry> = after
        .history
        .as_ref()
        .map(|h| h.get_entries().clone())
        .unwrap_or_default();
    let mut previous = before.clone();
    previous.history = None;
    let mut expected_history = old;
    expected_history.push(previous);
    assert_eq!(new, expected_history, "{rel}: history of the edited entry");

    // Everything else: put the edit's legitimate changes into the original and compare the whole database.
    let mut expected = original.clone();
    {
        let mut entry = expected.entry_mut(target).unwrap();
        entry.fields.clone_from(&after.fields);
        entry.times = after.times.clone();
        entry.history.clone_from(&after.history);
    }
    // The attachment pool also records which entry versions use each attachment; that bookkeeping is rebuilt
    // when a file is read, not stored in it. So the expected database goes through the same save -> reopen (the
    // plain round trip, `tests/roundtrip.rs`, shows that this loses nothing) and is compared with what the app
    // wrote, whole. A version whose attachment is gone, or an attachment nobody uses, still fails.
    let mut canonical = Vec::new();
    expected
        .save(&mut canonical, key.to_database_key().unwrap())
        .unwrap();
    let expected = open::open(&canonical, key).unwrap().db;
    assert_eq!(
        reopened.db.clone().remove_unused_attachments(),
        0,
        "{rel}: an attachment nobody uses was written"
    );
    assert_same(&expected, &reopened.db, rel);
}

/// All groups of the database, root first.
fn all_groups(db: &Database) -> Vec<KpGroupId> {
    let mut groups = Vec::new();
    let mut pending = vec![db.root().id()];
    while let Some(id) = pending.pop() {
        groups.push(id);
        pending.extend(db.group(id).unwrap().group_ids());
    }
    groups
}

/// `assert_eq!` on two databases that names the part that differs (the whole `Debug` prints hash maps in an
/// arbitrary order, so a line diff is useless). The fixtures hold fake data only.
fn assert_same(expected: &Database, actual: &Database, rel: &str) {
    if expected == actual {
        return;
    }
    let mut parts = Vec::new();
    if expected.config != actual.config {
        parts.push("config".to_owned());
    }
    if expected.meta != actual.meta {
        parts.push(format!(
            "meta:
{:?}
{:?}",
            expected.meta, actual.meta
        ));
    }
    if expected.deleted_objects != actual.deleted_objects {
        parts.push("deleted objects".to_owned());
    }
    for id in all_entries(expected) {
        let (a, b) = (expected.entry(id), actual.entry(id));
        match (a, b) {
            (Some(a), Some(b)) if Entry::clone(&a) == Entry::clone(&b) => {}
            (Some(a), Some(b)) => parts.push(format!(
                "entry {id:?}:
{:?}
{:?}",
                Entry::clone(&a),
                Entry::clone(&b)
            )),
            _ => parts.push(format!("entry {id:?} missing")),
        }
    }
    for id in all_groups(expected) {
        let (a, b) = (expected.group(id), actual.group(id));
        match (a, b) {
            (Some(a), Some(b)) if Group::clone(&a) == Group::clone(&b) => {}
            (Some(a), Some(b)) => parts.push(format!(
                "group {id:?}:
{:?}
{:?}",
                Group::clone(&a),
                Group::clone(&b)
            )),
            _ => parts.push(format!("group {id:?} missing")),
        }
    }
    if all_entries(expected).len() != all_entries(actual).len()
        || all_groups(expected).len() != all_groups(actual).len()
    {
        parts.push("number of entries or groups".to_owned());
    }
    let pool = |db: &Database| {
        let mut out: Vec<String> = all_entries(db)
            .into_iter()
            .flat_map(|id| {
                let entry = db.entry(id).unwrap();
                entry
                    .attachments_named()
                    .map(|(name, a)| {
                        format!(
                            "{name} id={:?} protected={} len={} full={:?}",
                            a.id(),
                            a.data.is_protected(),
                            a.data.get().len(),
                            {
                                let mut lines: Vec<String> = format!("{:?}", *a)
                                    .split("), (")
                                    .map(str::to_owned)
                                    .collect();
                                lines.sort();
                                lines.len()
                            }
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        out.sort();
        out
    };
    if pool(expected) != pool(actual) {
        parts.push(format!(
            "attachments:
{:#?}
{:#?}",
            pool(expected),
            pool(actual)
        ));
    }
    if expected.num_attachments() != actual.num_attachments() {
        parts.push("number of attachments".to_owned());
    }
    panic!(
        "{rel}: something else changed after edit -> save -> reopen: {}",
        if parts.is_empty() {
            "attachments or custom icons".to_owned()
        } else {
            parts.join(
                "
",
            )
        }
    );
}

#[test]
fn rf_25_roundtrip_after_edit_keepass2_password() {
    edit_one_title_and_compare("keepass2/f-pass.kdbx", &key(Some(PASSWORD), false));
}

#[test]
fn rf_25_roundtrip_after_edit_keepass2_keyfile() {
    edit_one_title_and_compare("keepass2/f-key.kdbx", &key(None, true));
}

#[test]
fn rf_25_roundtrip_after_edit_keepass2_password_and_keyfile() {
    edit_one_title_and_compare("keepass2/f-both.kdbx", &key(Some(PASSWORD), true));
}

#[test]
fn rf_25_roundtrip_after_edit_keepass2_unicode_password() {
    edit_one_title_and_compare(
        "keepass2/f-unicode.kdbx",
        &key(Some(UNICODE_PASSWORD), false),
    );
}

#[test]
fn rf_25_roundtrip_after_edit_keepassxc() {
    edit_one_title_and_compare("keepassxc/xc-both4.kdbx", &key(Some(PASSWORD), true));
}

#[test]
fn rf_25_the_edited_entry_of_keepass2_keeps_its_attachments_and_custom_fields() {
    // The KeePass 2 corpus (see `tests/roundtrip.rs`): "GitHub" has 2 history versions, 2 attachments and 2 custom
    // fields; the edit lands on it and they survive with their bytes and flags.
    let key = key(Some(PASSWORD), false);
    let mut vault = open::open(&fixture("keepass2/f-pass.kdbx"), &key).unwrap();
    let target = richest_entry(&vault.db);
    assert_eq!(
        vault.db.entry(target).unwrap().get(fields::TITLE),
        Some("GitHub")
    );
    let patch = EntryPatch {
        title: Some(NEW_TITLE.to_owned()),
        ..EntryPatch::default()
    };
    vault
        .update_entry(EntryId::from_uuid(target.uuid()), patch, now())
        .unwrap();
    let reopened = open::open(&vault.to_bytes(&key).unwrap(), &key).unwrap();
    let entry = reopened.db.entry(target).unwrap();
    assert_eq!(
        entry
            .attachment_by_name("adjunto.txt")
            .unwrap()
            .data
            .get()
            .as_slice(),
        "contenido del adjunto".as_bytes()
    );
    let binary = entry.attachment_by_name("protegido.bin").unwrap();
    assert!(binary.data.is_protected());
    assert_eq!(binary.data.get(), &(0..=255).collect::<Vec<u8>>());
    let protected = entry.fields.get("Campo protegido").unwrap();
    assert!(protected.is_protected());
    assert_eq!(protected.get(), "secreto-ñ");
    // The two old versions plus the one the edit added.
    assert_eq!(entry.history.as_ref().unwrap().get_entries().len(), 3);
}
