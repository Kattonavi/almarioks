//! Creating and editing entries through the public API (RF-14, RF-38 to RF-41, RF-45, RF-91, RF-93, examples 8
//! and 9). Expected values come from the spec. Vaults are created in memory with 2 iterations.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::PathBuf};

use almarioks_core::{
    CoreError,
    kdbx::{MasterPassword, NewVaultKdf, OpenedVault, VaultKey, create_vault, open},
    vault::{EntryId, EntryPatch, GroupId, NewEntry, Timestamp},
};
use secrecy::{ExposeSecret, SecretString};

const PASSWORD: &str = "Maestra-entradas-2026";

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn vault() -> OpenedVault {
    let bytes = create_vault(
        &key(),
        "Mis cuentas",
        NewVaultKdf::with_iterations(2).unwrap(),
    )
    .unwrap();
    open(&bytes, &key()).unwrap()
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(1_794_391_200 + seconds).unwrap()
}

fn secret(text: &str) -> Option<SecretString> {
    Some(SecretString::from(text.to_owned()))
}

fn titled(title: &str) -> NewEntry {
    NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    }
}

fn create(vault: &mut OpenedVault, entry: NewEntry) -> Result<EntryId, CoreError> {
    let root = vault.root_id();
    vault.create_entry(root, entry, at(0))
}

#[test]
fn rf_38_creates_the_entry_with_all_its_fields_in_the_chosen_group() {
    let mut vault = vault();
    let id = create(
        &mut vault,
        NewEntry {
            title: "GitHub".into(),
            username: "ana@example.com".into(),
            password: secret("tercera-clave"),
            url: "https://github.com/login".into(),
            notes: "línea 1\nlínea 2\tcon tab".into(),
            tags: vec![" trabajo ".into(), "código".into()],
        },
    )
    .unwrap();
    let detail = vault.entry(id).unwrap();
    assert_eq!(detail.group_id, vault.root_id());
    assert_eq!(detail.summary.title.as_deref(), Some("GitHub"));
    assert_eq!(detail.summary.username.as_deref(), Some("ana@example.com"));
    assert_eq!(
        detail.summary.url.as_deref(),
        Some("https://github.com/login")
    );
    assert_eq!(detail.notes.as_deref(), Some("línea 1\nlínea 2\tcon tab"));
    assert_eq!(detail.summary.tags, ["trabajo", "código"]);
    assert_eq!(detail.protected_fields, ["Password"]);
    assert_eq!(detail.summary.modified, at(0));
    assert_eq!(detail.created, at(0));
    assert_eq!(
        vault.reveal_field(id, "Password").unwrap().expose_secret(),
        "tercera-clave"
    );
    assert_eq!(vault.list_entries(vault.root_id()).unwrap().len(), 1);
    // It survives encrypting to bytes and reopening.
    let reopened = open(&vault.to_bytes(&key()).unwrap(), &key()).unwrap();
    assert_eq!(reopened.entry(id).unwrap(), detail);
}

#[test]
fn rf_93_creating_in_a_group_that_does_not_exist_is_no_encontrado() {
    let mut vault = vault();
    let ghost = GroupId::parse("00000000-0000-4000-8000-000000000001").unwrap();
    assert_eq!(
        vault.create_entry(ghost, titled("   "), at(0)).unwrap_err(),
        CoreError::NoEncontrado,
        "NoEncontrado comes before the domain rules"
    );
    let ghost_entry = EntryId::parse("00000000-0000-4000-8000-000000000002").unwrap();
    let patch = EntryPatch {
        title: Some(String::new()),
        ..EntryPatch::default()
    };
    assert_eq!(
        vault.update_entry(ghost_entry, patch, at(1)).unwrap_err(),
        CoreError::NoEncontrado
    );
}

#[test]
fn rf_39_rf_40_example_9_title_rules() {
    let mut vault = vault();
    assert_eq!(
        create(&mut vault, titled("   ")).unwrap_err(),
        CoreError::TituloVacio
    );
    assert_eq!(
        create(&mut vault, titled(&" ".repeat(300))).unwrap_err(),
        CoreError::TituloVacio
    );
    let banco = create(&mut vault, titled("  Banco  ")).unwrap();
    assert_eq!(
        vault.entry(banco).unwrap().summary.title.as_deref(),
        Some("Banco")
    );
    assert_eq!(
        create(&mut vault, titled(&"a".repeat(256))).unwrap_err(),
        CoreError::CampoMuyLargo
    );
    let emoji = create(&mut vault, titled(&"😀".repeat(255))).unwrap();
    assert_eq!(
        vault.entry(emoji).unwrap().summary.title,
        Some("😀".repeat(255))
    );
    // The same rules on an edit.
    let blank = EntryPatch {
        title: Some("  ".into()),
        ..EntryPatch::default()
    };
    assert_eq!(
        vault.update_entry(banco, blank, at(1)).unwrap_err(),
        CoreError::TituloVacio
    );
}

#[test]
fn rf_40_each_limit_counts_characters_and_the_limit_itself_is_accepted() {
    let mut vault = vault();
    let fits = NewEntry {
        title: "ñ".repeat(255),
        username: "ñ".repeat(255),
        password: secret(&"ñ".repeat(1_024)),
        url: "ñ".repeat(2_048),
        notes: "ñ".repeat(100_000),
        tags: (0..50).map(|n| format!("{n:0>50}")).collect(),
    };
    create(&mut vault, fits).unwrap();
    let over: [NewEntry; 7] = [
        NewEntry {
            username: "a".repeat(256),
            ..titled("t")
        },
        NewEntry {
            password: secret(&"a".repeat(1_025)),
            ..titled("t")
        },
        NewEntry {
            url: "a".repeat(2_049),
            ..titled("t")
        },
        NewEntry {
            notes: "a".repeat(100_001),
            ..titled("t")
        },
        NewEntry {
            tags: (0..51).map(|n| format!("e{n}")).collect(),
            ..titled("t")
        },
        NewEntry {
            tags: vec!["a".repeat(51)],
            ..titled("t")
        },
        titled(&"b".repeat(256)),
    ];
    for entry in over {
        assert_eq!(
            create(&mut vault, entry).unwrap_err(),
            CoreError::CampoMuyLargo
        );
    }
}

#[test]
fn errors_follow_the_order_of_the_spec() {
    let mut vault = vault();
    // TituloVacio before CampoMuyLargo.
    let both = NewEntry {
        notes: "a".repeat(100_001),
        ..titled(" ")
    };
    assert_eq!(
        create(&mut vault, both).unwrap_err(),
        CoreError::TituloVacio
    );
    // CampoMuyLargo before CaracterNoPermitido before EtiquetaInvalida.
    let long_and_control = NewEntry {
        username: "a".repeat(256),
        url: "x\u{7}".into(),
        tags: vec!["a;b".into()],
        ..titled("t")
    };
    assert_eq!(
        create(&mut vault, long_and_control).unwrap_err(),
        CoreError::CampoMuyLargo
    );
    let control_and_tag = NewEntry {
        url: "x\u{7}".into(),
        tags: vec!["a;b".into()],
        ..titled("t")
    };
    assert_eq!(
        create(&mut vault, control_and_tag).unwrap_err(),
        CoreError::CaracterNoPermitido
    );
}

#[test]
fn control_characters_are_rejected_except_tab_and_line_feed() {
    let mut vault = vault();
    // In the middle: CR and U+0085 are also White_Space, so at an end the title's trim would remove them.
    for bad in [
        "a\u{0}b", "a\rb", "a\u{1F}b", "a\u{7F}b", "a\u{85}b", "a\u{9F}b",
    ] {
        assert_eq!(
            create(&mut vault, titled(bad)).unwrap_err(),
            CoreError::CaracterNoPermitido,
            "{bad:?}"
        );
        let password = NewEntry {
            password: secret(bad),
            ..titled("t")
        };
        assert_eq!(
            create(&mut vault, password).unwrap_err(),
            CoreError::CaracterNoPermitido
        );
    }
    create(&mut vault, titled("a\tb")).unwrap();
}

#[test]
fn rf_41_the_password_is_stored_exactly_as_written() {
    let mut vault = vault();
    // Spaces at both ends, capitals and a decomposed "é" (e + U+0301): nothing is trimmed or normalized.
    let exact = "  Clave E\u{301}ñ \u{3000}";
    let id = create(
        &mut vault,
        NewEntry {
            password: secret(exact),
            ..titled("t")
        },
    )
    .unwrap();
    let reopened = open(&vault.to_bytes(&key()).unwrap(), &key()).unwrap();
    assert_eq!(
        reopened
            .reveal_field(id, "Password")
            .unwrap()
            .expose_secret(),
        exact
    );
}

#[test]
fn rf_91_tags_with_separators_or_empty_are_rejected_and_kept_trimmed() {
    let mut vault = vault();
    for bad in ["a;b", "a,b", "   ", ""] {
        let entry = NewEntry {
            tags: vec!["bien".into(), bad.into()],
            ..titled("t")
        };
        assert_eq!(
            create(&mut vault, entry).unwrap_err(),
            CoreError::EtiquetaInvalida,
            "{bad:?}"
        );
    }
    let id = create(
        &mut vault,
        NewEntry {
            tags: vec!["  casa ".into()],
            ..titled("t")
        },
    )
    .unwrap();
    assert_eq!(vault.entry(id).unwrap().summary.tags, ["casa"]);
}

#[test]
fn rf_45_a_patch_changes_only_what_it_brings_also_to_empty() {
    let mut vault = vault();
    let id = create(
        &mut vault,
        NewEntry {
            title: "GitHub".into(),
            username: "ana".into(),
            password: secret("tercera-clave"),
            url: "https://github.com".into(),
            notes: "notas".into(),
            tags: vec!["trabajo".into()],
        },
    )
    .unwrap();
    let patch = EntryPatch {
        username: Some(String::new()),
        tags: Some(Vec::new()),
        ..EntryPatch::default()
    };
    assert!(vault.update_entry(id, patch, at(60)).unwrap());
    let detail = vault.entry(id).unwrap();
    assert_eq!(detail.summary.title.as_deref(), Some("GitHub"));
    assert_eq!(detail.summary.username.as_deref(), Some(""));
    assert_eq!(detail.summary.url.as_deref(), Some("https://github.com"));
    assert_eq!(detail.notes.as_deref(), Some("notas"));
    assert!(detail.summary.tags.is_empty());
    assert_eq!(detail.summary.modified, at(60));
    assert_eq!(detail.created, at(0));
    assert_eq!(
        vault.reveal_field(id, "Password").unwrap().expose_secret(),
        "tercera-clave"
    );
    // Same values again (the title compared after trimming): no change, the date stays.
    let same = EntryPatch {
        title: Some(" GitHub ".into()),
        username: Some(String::new()),
        password: secret("tercera-clave"),
        ..EntryPatch::default()
    };
    assert!(!vault.update_entry(id, same, at(120)).unwrap());
    assert_eq!(vault.entry(id).unwrap().summary.modified, at(60));
    // The password can be replaced by the empty one.
    let empty_password = EntryPatch {
        password: secret(""),
        ..EntryPatch::default()
    };
    assert!(vault.update_entry(id, empty_password, at(180)).unwrap());
    assert_eq!(
        vault.reveal_field(id, "Password").unwrap().expose_secret(),
        ""
    );
    assert!(vault.entry(id).unwrap().protected_fields.is_empty());
}

#[test]
fn rf_43_canary_new_entry_input_and_patch_do_not_show_the_password_in_debug() {
    let canary = "CANARIO-entrada-9d2";
    let entry = NewEntry {
        password: secret(canary),
        ..titled("t")
    };
    let patch = EntryPatch {
        password: secret(canary),
        ..EntryPatch::default()
    };
    assert!(!format!("{entry:?}").contains(canary));
    assert!(!format!("{patch:?}").contains(canary));
    let mut vault = vault();
    let id = create(&mut vault, entry).unwrap();
    assert!(!format!("{:?}", vault.entry(id).unwrap()).contains(canary));
    assert!(!format!("{:?}", vault.list_entries(vault.root_id()).unwrap()).contains(canary));
}

fn v3() -> OpenedVault {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keepass2/v3-aes.kdbx");
    let key = VaultKey::new(Some(MasterPassword::new("lab-Maestra-2026".into())), None).unwrap();
    open(&fs::read(path).unwrap(), &key).unwrap()
}

#[test]
fn rf_14_a_kdbx3_vault_reads_but_every_change_and_save_is_solo_lectura() {
    let mut vault = v3();
    assert!(vault.is_read_only());
    let root = vault.root_id();
    let listed = vault.list_entries(root).unwrap();
    assert!(
        !listed.is_empty(),
        "the 3.1 fixture has an entry in the root"
    );
    assert_eq!(
        vault.create_entry(root, titled("t"), at(0)).unwrap_err(),
        CoreError::SoloLectura
    );
    // SoloLectura comes before NoEncontrado and before the domain rules.
    let ghost = EntryId::parse("00000000-0000-4000-8000-000000000003").unwrap();
    let blank = EntryPatch {
        title: Some(String::new()),
        ..EntryPatch::default()
    };
    assert_eq!(
        vault.update_entry(ghost, blank, at(0)).unwrap_err(),
        CoreError::SoloLectura
    );
    assert_eq!(
        vault
            .update_entry(listed[0].id, EntryPatch::default(), at(0))
            .unwrap_err(),
        CoreError::SoloLectura
    );
    assert_eq!(vault.to_bytes(&key()).unwrap_err(), CoreError::SoloLectura);
}
