//! RF-40 with its data (plan "Precisiones" 15): the field and, for `CampoMuyLargo`, the limit, as data. Limits and
//! field names come from the spec: title and user name 255, URL 2 048, password 1 024, notes 100 000, 50 tags of 50
//! characters; group names 255 (RF-52).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use almarioks_core::{
    CoreError, FieldError,
    vault::{EntryPatch, NewEntry, validate_group_name, validate_new_entry, validate_patch},
};
use secrecy::SecretString;

fn titled(title: &str) -> NewEntry {
    NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    }
}

fn data(error: FieldError) -> (&'static str, &'static str, Option<usize>) {
    (error.code(), error.field().name(), error.limit())
}

#[test]
fn rf_40_each_too_long_field_says_which_field_and_which_limit() {
    let cases: Vec<(NewEntry, &str, usize)> = vec![
        (titled(&"a".repeat(256)), "Title", 255),
        (
            NewEntry {
                username: "u".repeat(256),
                ..titled("t")
            },
            "UserName",
            255,
        ),
        (
            NewEntry {
                url: "h".repeat(2_049),
                ..titled("t")
            },
            "URL",
            2_048,
        ),
        (
            NewEntry {
                password: Some(SecretString::from("p".repeat(1_025))),
                ..titled("t")
            },
            "Password",
            1_024,
        ),
        (
            NewEntry {
                notes: "n".repeat(100_001),
                ..titled("t")
            },
            "Notes",
            100_000,
        ),
        (
            NewEntry {
                tags: (0..51).map(|i| format!("e{i}")).collect(),
                ..titled("t")
            },
            "Tags",
            50,
        ),
        (
            NewEntry {
                tags: vec!["x".repeat(51)],
                ..titled("t")
            },
            "Tags",
            50,
        ),
    ];
    for (entry, field, limit) in cases {
        let error = validate_new_entry(&entry).unwrap_err();
        assert_eq!(data(error), ("CampoMuyLargo", field, Some(limit)));
        assert_eq!(CoreError::from(error), CoreError::CampoMuyLargo);
        assert_eq!(error.message(), CoreError::CampoMuyLargo.message());
    }
    // At the limit, in characters (255 emoji = 1 020 bytes): accepted (example 9).
    validate_new_entry(&titled(&"\u{1F600}".repeat(255))).unwrap();
}

#[test]
fn rf_40_a_control_character_or_a_bad_tag_says_which_field() {
    let notes = EntryPatch {
        notes: Some("a\u{7}b".into()),
        ..EntryPatch::default()
    };
    assert_eq!(
        data(validate_patch(&notes).unwrap_err()),
        ("CaracterNoPermitido", "Notes", None)
    );
    let password = EntryPatch {
        password: Some(SecretString::from("a\u{0}b".to_owned())),
        ..EntryPatch::default()
    };
    assert_eq!(
        data(validate_patch(&password).unwrap_err()),
        ("CaracterNoPermitido", "Password", None)
    );
    let tag = EntryPatch {
        tags: Some(vec!["a;b".into()]),
        ..EntryPatch::default()
    };
    assert_eq!(
        data(validate_patch(&tag).unwrap_err()),
        ("EtiquetaInvalida", "Tags", None)
    );
    // Only the fields present are checked; an empty patch is valid.
    validate_patch(&EntryPatch::default()).unwrap();
}

#[test]
fn rf_40_rf_52_a_group_name_error_names_the_field_name() {
    assert_eq!(
        data(validate_group_name(&"g".repeat(256)).unwrap_err()),
        ("CampoMuyLargo", "Name", Some(255))
    );
    assert_eq!(
        data(validate_group_name("a\u{1B}b").unwrap_err()),
        ("CaracterNoPermitido", "Name", None)
    );
    assert_eq!(validate_group_name("  Casa  ").unwrap(), "Casa");
}
