//! Entry edits that need the library's view: history order in the saved file, history limits, protection kept
//! on edit (RF-26, RF-38, RF-45, example 8, spec "Campo protegido").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use keepass::db::{EntryId as KpEntryId, fields};
use secrecy::{ExposeSecret, SecretString};

use super::{MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open};
use crate::vault::{EntryId, EntryPatch, NewEntry, Timestamp};

const PASSWORD: &str = "pruebas-entradas-2026";

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

fn secret(text: &str) -> Option<SecretString> {
    Some(SecretString::from(text.to_owned()))
}

fn titled(title: &str) -> EntryPatch {
    EntryPatch {
        title: Some(title.to_owned()),
        ..EntryPatch::default()
    }
}

fn history_titles(vault: &OpenedVault, id: EntryId) -> Vec<String> {
    let entry = vault.db.entry(KpEntryId::from_uuid(id.uuid())).unwrap();
    entry
        .history
        .as_ref()
        .map(|h| {
            h.get_entries()
                .iter()
                .map(|e| e.get_title().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn github(vault: &mut OpenedVault) -> EntryId {
    let root = vault.root_id();
    vault
        .create_entry(
            root,
            NewEntry {
                title: "GitHub".into(),
                username: "ana".into(),
                password: secret("tercera-clave"),
                ..NewEntry::default()
            },
            at(0),
        )
        .unwrap()
}

#[test]
fn rf_45_example_8_title_only_edit_keeps_the_password_and_adds_one_version() {
    let mut vault = vault();
    let id = github(&mut vault);
    assert!(
        vault
            .update_entry(id, titled("GitHub personal"), at(10))
            .unwrap()
    );
    assert_eq!(
        vault.reveal_field(id, "Password").unwrap().expose_secret(),
        "tercera-clave"
    );
    let detail = vault.entry(id).unwrap();
    assert_eq!(detail.summary.title.as_deref(), Some("GitHub personal"));
    assert_eq!(detail.summary.modified, at(10));
    assert_eq!(history_titles(&vault, id), ["GitHub"]);

    // Confirm again without changing anything: neither the date nor the history change.
    assert!(
        !vault
            .update_entry(id, titled("GitHub personal"), at(20))
            .unwrap()
    );
    assert!(
        !vault
            .update_entry(id, EntryPatch::default(), at(30))
            .unwrap()
    );
    assert_eq!(vault.entry(id).unwrap().summary.modified, at(10));
    assert_eq!(history_titles(&vault, id), ["GitHub"]);
}

#[test]
fn rf_26_history_is_saved_oldest_to_newest_and_reopens_in_that_order() {
    let mut vault = vault();
    let id = github(&mut vault);
    for (n, title) in ["B", "C", "D"].into_iter().enumerate() {
        let seconds = i64::try_from(n).unwrap() + 1;
        assert!(vault.update_entry(id, titled(title), at(seconds)).unwrap());
    }
    assert_eq!(history_titles(&vault, id), ["GitHub", "B", "C"]);
    let reopened = reopen(&vault);
    assert_eq!(history_titles(&reopened, id), ["GitHub", "B", "C"]);
    assert_eq!(
        reopened.entry(id).unwrap().summary.title.as_deref(),
        Some("D")
    );
}

#[test]
fn rf_26_history_max_items_drops_the_oldest_versions() {
    let mut vault = vault();
    vault.db.meta.history_max_items = Some(2);
    vault.db.meta.history_max_size = Some(-1);
    let id = github(&mut vault);
    for (n, title) in ["B", "C", "D", "E"].into_iter().enumerate() {
        let seconds = i64::try_from(n).unwrap() + 1;
        vault.update_entry(id, titled(title), at(seconds)).unwrap();
    }
    assert_eq!(history_titles(&vault, id), ["C", "D"]);
    assert_eq!(history_titles(&reopen(&vault), id), ["C", "D"]);
}

#[test]
fn rf_26_history_max_size_drops_the_oldest_versions() {
    let mut vault = vault();
    vault.db.meta.history_max_items = Some(-1);
    // Any version is larger than 0 bytes: with this limit no version is kept.
    vault.db.meta.history_max_size = Some(0);
    let id = github(&mut vault);
    vault.update_entry(id, titled("B"), at(1)).unwrap();
    assert!(history_titles(&vault, id).is_empty());
    // Without limits in the vault, nothing is dropped.
    vault.db.meta.history_max_items = None;
    vault.db.meta.history_max_size = None;
    vault.update_entry(id, titled("C"), at(2)).unwrap();
    vault.update_entry(id, titled("D"), at(3)).unwrap();
    assert_eq!(history_titles(&vault, id), ["B", "C"]);
}

#[test]
fn rf_26_new_vaults_carry_the_keepass_history_limits() {
    let vault = vault();
    assert_eq!(vault.db.meta.history_max_items, Some(10));
    assert_eq!(vault.db.meta.history_max_size, Some(6 * 1024 * 1024));
}

#[test]
fn rf_38_only_the_password_is_protected_in_a_new_entry() {
    let mut vault = vault();
    let id = github(&mut vault);
    let entry = vault.db.entry(KpEntryId::from_uuid(id.uuid())).unwrap();
    for name in [fields::TITLE, fields::USERNAME, fields::URL, fields::NOTES] {
        assert!(!entry.fields[name].is_protected(), "{name}");
    }
    assert!(entry.fields[fields::PASSWORD].is_protected());
    assert_eq!(entry.times.creation, entry.times.last_modification);
    assert_eq!(
        entry.times.creation.unwrap().and_utc().timestamp(),
        at(0).unix_seconds()
    );
}

#[test]
fn rf_45_an_edit_keeps_the_protection_of_each_field() {
    let mut vault = vault();
    let id = github(&mut vault);
    // A KeePass vault that protects the notes.
    vault
        .db
        .entry_mut(KpEntryId::from_uuid(id.uuid()))
        .unwrap()
        .set_protected(fields::NOTES, "nota vieja");
    let patch = EntryPatch {
        notes: Some("nota nueva".into()),
        password: secret("otra-clave"),
        ..EntryPatch::default()
    };
    assert!(vault.update_entry(id, patch, at(5)).unwrap());
    let reopened = reopen(&vault);
    let entry = reopened.db.entry(KpEntryId::from_uuid(id.uuid())).unwrap();
    assert!(entry.fields[fields::NOTES].is_protected());
    assert!(entry.fields[fields::PASSWORD].is_protected());
    assert_eq!(reopened.entry(id).unwrap().notes, None);
    assert_eq!(
        reopened.reveal_field(id, "Notes").unwrap().expose_secret(),
        "nota nueva"
    );
    // The previous version keeps the old protected values.
    let old = &entry.history.as_ref().unwrap().get_entries()[0];
    assert_eq!(old.get(fields::NOTES), Some("nota vieja"));
    assert!(old.fields[fields::NOTES].is_protected());
    assert_eq!(old.get_password(), Some("tercera-clave"));
}
