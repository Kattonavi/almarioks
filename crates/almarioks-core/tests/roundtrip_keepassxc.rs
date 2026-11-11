//! Round trip of the `keepass` crate against a vault written by KeePassXC 2.8.0 (KDBX 4, password + key file).
//!
//! Same contract as `roundtrip.rs`: open -> save to memory -> reopen gives the same database, and content
//! KeePassXC wrote is read from the original. Fixture holds fake data only (lab 19).

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs::File, path::PathBuf};

use keepass::{Database, DatabaseKey, db::GroupRef};

const PASSWORD: &str = "lab-Maestra-2026";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn key() -> DatabaseKey {
    // KeePassXC reused the KeePass 2 key file of the corpus.
    let mut keyfile =
        File::open(fixtures().join("keepass2").join("llave.keyx")).expect("open key file fixture");
    DatabaseKey::new()
        .with_password(PASSWORD)
        .with_keyfile(&mut keyfile)
        .expect("read key file fixture")
}

/// Searches the whole tree and returns the data of `attachment` in the entry titled `title`.
fn attachment_of(group: &GroupRef<'_>, title: &str, attachment: &str) -> Option<Vec<u8>> {
    if let Some(entry) = group.entry_by_name(title) {
        return entry
            .attachment_by_name(attachment)
            .map(|found| found.data.get().clone());
    }
    group
        .groups()
        .find_map(|child| attachment_of(&child, title, attachment))
}

#[test]
fn roundtrip_keepassxc_both() {
    let key = key();
    let mut file =
        File::open(fixtures().join("keepassxc").join("xc-both4.kdbx")).expect("open vault fixture");
    let original = Database::open(&mut file, key.clone()).expect("decrypt vault fixture");

    let data = attachment_of(&original.root(), "Hecha en KeePassXC", "xc.txt")
        .expect("entry 'Hecha en KeePassXC' with attachment 'xc.txt'");
    assert!(!data.is_empty(), "'xc.txt' has content");

    let mut saved = Vec::new();
    original
        .save(&mut saved, key.clone())
        .expect("save to memory");
    let reopened = Database::parse(&saved, key).expect("reopen saved vault");

    assert_eq!(
        original, reopened,
        "database changed after open -> save -> reopen"
    );
    assert_eq!(
        attachment_of(&reopened.root(), "Hecha en KeePassXC", "xc.txt"),
        Some(data)
    );
}
