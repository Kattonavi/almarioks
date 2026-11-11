//! Round trip of the `keepass` crate against vaults written by KeePass 2.62 (KeePassLib, KDBX 4.1).
//!
//! This is a test of the library, not of product code: open -> save to memory -> reopen must give the same
//! database, and what we read from the original must match what KeePass 2 wrote, so a loss while READING also
//! fails. Fixtures hold fake data only; they come from `docs/lab/kdbx-roundtrip`.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs::File, path::PathBuf};

use keepass::{
    Database, DatabaseKey,
    db::{DatabaseOpenError, fields},
    error::DatabaseKeyError,
};

const PASSWORD: &str = "lab-Maestra-2026";
const UNICODE_PASSWORD: &str = "ñandú-€-2026";
const KEYFILE: &str = "llave.keyx";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("keepass2")
        .join(name)
}

fn key(password: Option<&str>, keyfile: bool) -> DatabaseKey {
    let mut key = DatabaseKey::new();
    if let Some(password) = password {
        key = key.with_password(password);
    }
    if keyfile {
        let mut file = File::open(fixture(KEYFILE)).expect("open key file fixture");
        key = key.with_keyfile(&mut file).expect("read key file fixture");
    }
    key
}

fn open(name: &str, key: &DatabaseKey) -> Database {
    let mut file = File::open(fixture(name)).expect("open vault fixture");
    Database::open(&mut file, key.clone()).expect("decrypt vault fixture")
}

/// Content KeePass 2 wrote into every fixture (see the lab script). Checked on the original, before saving.
fn assert_known_content(db: &Database) {
    let root = db.root();
    let work = root.group_by_name("Trabajo").expect("group 'Trabajo'");
    assert!(
        work.group_by_name("Subgrupo").is_some(),
        "nested group 'Subgrupo'"
    );

    let github = work.entry_by_name("GitHub").expect("entry 'GitHub'");
    assert_eq!(github.get_username(), Some("sara@example.com"));
    assert_eq!(github.get_password(), Some("tercera-clave"));
    assert_eq!(github.get_url(), Some("https://github.com"));

    let history = github.history.as_ref().expect("history of 'GitHub'");
    assert_eq!(history.get_entries().len(), 2, "two history versions");
    let mut old_passwords: Vec<&str> = history
        .get_entries()
        .iter()
        .filter_map(|version| version.get(fields::PASSWORD))
        .collect();
    old_passwords.sort_unstable();
    assert_eq!(old_passwords, ["primera-clave", "segunda-clave"]);

    let protected = github
        .fields
        .get("Campo protegido")
        .expect("field 'Campo protegido'");
    assert!(
        protected.is_protected(),
        "'Campo protegido' keeps its protection flag"
    );
    assert_eq!(protected.get(), "secreto-ñ");
    let visible = github
        .fields
        .get("Campo propio")
        .expect("field 'Campo propio'");
    assert!(!visible.is_protected());

    assert_eq!(github.attachments().count(), 2, "two attachments");
    let text = github
        .attachment_by_name("adjunto.txt")
        .expect("attachment 'adjunto.txt'");
    assert!(!text.data.is_protected());
    assert_eq!(
        text.data.get().as_slice(),
        "contenido del adjunto".as_bytes()
    );
    let binary = github
        .attachment_by_name("protegido.bin")
        .expect("attachment 'protegido.bin'");
    assert!(
        binary.data.is_protected(),
        "'protegido.bin' keeps its protection flag"
    );
    let expected: Vec<u8> = (0..=255).collect();
    assert_eq!(binary.data.get(), &expected);
}

fn roundtrip(name: &str, key: &DatabaseKey) {
    let original = open(name, key);
    assert_known_content(&original);

    let mut saved = Vec::new();
    original
        .save(&mut saved, key.clone())
        .expect("save to memory");
    let reopened = Database::parse(&saved, key.clone()).expect("reopen saved vault");

    assert_eq!(
        original, reopened,
        "{name}: database changed after open -> save -> reopen"
    );
    assert_known_content(&reopened);
}

#[test]
fn roundtrip_keepass2_password() {
    roundtrip("f-pass.kdbx", &key(Some(PASSWORD), false));
}

#[test]
fn roundtrip_keepass2_keyfile() {
    roundtrip("f-key.kdbx", &key(None, true));
}

#[test]
fn roundtrip_keepass2_password_and_keyfile() {
    roundtrip("f-both.kdbx", &key(Some(PASSWORD), true));
}

#[test]
fn roundtrip_keepass2_unicode_password() {
    roundtrip("f-unicode.kdbx", &key(Some(UNICODE_PASSWORD), false));
}

#[test]
fn wrong_key_is_rejected() {
    let cases = [
        ("f-pass.kdbx", key(Some("lab-Maestra-2025"), false)),
        // Password and key file are both required: the password alone must not open it.
        ("f-both.kdbx", key(Some(PASSWORD), false)),
        ("f-key.kdbx", key(Some(PASSWORD), false)),
    ];
    for (name, wrong) in cases {
        let mut file = File::open(fixture(name)).expect("open vault fixture");
        let result = Database::open(&mut file, wrong);
        assert!(
            matches!(
                result,
                Err(DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey))
            ),
            "{name}: expected IncorrectKey, got {:?}",
            result.err()
        );
    }
}
