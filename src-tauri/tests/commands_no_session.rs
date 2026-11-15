//! The commands that need no open vault: file dialogs, password strength and the generator (T16 and part of T13).
//! Through the same IPC entry as the window, with a fake person answering the dialogs.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use almarioks_core::generator::{DIGITS, LOOKALIKES, LOWERCASE, SYMBOLS, UPPERCASE};
use almarioks_lib::commands::{
    INTERNAL_CODE,
    dialogs::{DialogMode, DialogUnavailable, FileDialogs, FilePick, SystemDialogs},
};
use common::{
    FakeDialogs, TempDir, app_with, error_code, invoke_args, invoke_plain, parts_with, test_parts,
};
use serde_json::{Value, json};
use tauri::test::{mock_builder, mock_context, noop_assets};

/// Fake data, never a real password.
const CANARY: &str = "CANARIO-ALMARIOKS-maestra-Zx9!";

const PICKS: [(&str, FilePick); 4] = [
    ("pick_vault_to_open", FilePick::VaultToOpen),
    ("pick_vault_to_create", FilePick::VaultToCreate),
    ("pick_key_file", FilePick::KeyFile),
    ("pick_new_key_file", FilePick::NewKeyFile),
];

// ---------- File dialogs (RF-1, RF-10, RF-20, RF-87) ----------

#[test]
fn rf_10_each_pick_command_shows_its_dialog_and_answers_the_chosen_path() {
    let dir = TempDir::new("pick-chosen");
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    let chosen = dir.path().join("mi bóveda.kdbx");
    dialogs.will_choose(&chosen);

    for (command, _) in PICKS {
        assert_eq!(
            invoke_plain(&app, command),
            Ok(json!(chosen.to_str().unwrap())),
            "{command}"
        );
    }

    assert_eq!(dialogs.asked(), PICKS.map(|(_, pick)| pick));
    assert!(
        !chosen.exists(),
        "a dialog only answers a path: it writes nothing"
    );
}

#[test]
fn rf_10_a_cancelled_dialog_answers_null() {
    let dir = TempDir::new("pick-cancelled");
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    dialogs.will_cancel();

    for (command, _) in PICKS {
        assert_eq!(invoke_plain(&app, command), Ok(Value::Null), "{command}");
    }
}

#[test]
fn rf_80_pick_commands_take_no_arguments_and_show_no_dialog_when_given_some() {
    let dir = TempDir::new("pick-args");
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    dialogs.will_choose(dir.path().join("x.kdbx"));

    for (command, _) in PICKS {
        // A window that tries to steer the dialog (a starting folder, a default path) is refused.
        let answer = invoke_args(&app, command, json!({ "default_path": "C:/Windows" }));
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "{command}");
    }
    assert!(dialogs.asked().is_empty());
}

#[test]
fn rf_83_a_dialog_that_cannot_be_shown_is_an_error_not_a_cancellation() {
    let dir = TempDir::new("pick-unavailable");
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    dialogs.will_fail();

    let answer = invoke_plain(&app, "pick_vault_to_open");

    assert_eq!(error_code(&answer), INTERNAL_CODE);
}

#[test]
fn rf_87_the_key_file_dialog_offers_keyx_and_any_file() {
    let key = FilePick::KeyFile.plan();
    assert_eq!(key.mode, DialogMode::Open);
    assert_eq!(key.filters[0].extensions, ["keyx", "key"]);
    assert!(
        key.filters.iter().any(|filter| filter.extensions == ["*"]),
        "KeePass takes any file as a key file: the dialog must let the person pick one"
    );

    let new_key = FilePick::NewKeyFile.plan();
    assert_eq!(new_key.mode, DialogMode::Save);
    assert_eq!(new_key.filters.len(), 1);
    assert_eq!(new_key.filters[0].extensions, ["keyx"]);
}

#[test]
fn rf_1_rf_10_the_vault_dialogs_offer_only_kdbx() {
    let open = FilePick::VaultToOpen.plan();
    let create = FilePick::VaultToCreate.plan();

    assert_eq!(
        (open.mode, create.mode),
        (DialogMode::Open, DialogMode::Save)
    );
    for plan in [open, create] {
        assert_eq!(plan.filters.len(), 1);
        assert_eq!(plan.filters[0].extensions, ["kdbx"]);
    }
    for (_, pick) in PICKS {
        assert!(!pick.plan().title.is_empty());
    }
}

#[test]
fn rf_79_the_system_dialogs_work_from_rust_while_the_window_has_no_dialog_permission() {
    // With the plugin registered, Rust reaches it through the app handle: no capability is involved, because
    // capabilities only guard what comes through the IPC. (That the window cannot call `plugin:dialog|*` is in
    // `commands_contract.rs`.) Actually showing a dialog needs a person: manual check of the release.
    let dir = TempDir::new("dialogs-from-rust");
    let app = app_with(test_parts(&dir));
    assert!(SystemDialogs::new(app.handle().clone()).is_available());

    // Without the plugin the dialogs say so instead of panicking, and nothing is shown.
    let bare = mock_builder()
        .build(mock_context(noop_assets()))
        .expect("an app without plugins");
    let dialogs = SystemDialogs::new(bare.handle().clone());
    assert!(!dialogs.is_available());
    assert_eq!(dialogs.pick(FilePick::VaultToOpen), Err(DialogUnavailable));
}

// ---------- Password strength (RF-7, RF-70) ----------

#[test]
fn rf_7_password_strength_answers_the_score_and_only_the_score() {
    let dir = TempDir::new("strength");
    let app = app_with(test_parts(&dir));

    // Anchors of the spec ("Fortaleza"): `123456` is 0; 20 random characters of the four types are 4.
    assert_eq!(
        invoke_args(&app, "password_strength", json!({ "password": "123456" })),
        Ok(json!({ "score": 0 }))
    );
    assert_eq!(
        invoke_args(
            &app,
            "password_strength",
            json!({ "password": "q7#Vd2!mZp9&Xr4$Lk8@" })
        ),
        Ok(json!({ "score": 4 }))
    );
    assert_eq!(
        invoke_args(&app, "password_strength", json!({ "password": "" })),
        Ok(json!({ "score": 0 }))
    );

    let answer = invoke_args(&app, "password_strength", json!({ "password": CANARY }));
    let text = serde_json::to_string(&answer.expect("a score")).unwrap();
    assert!(!text.contains(CANARY), "the password came back: {text}");
}

#[test]
fn rf_80_password_strength_rejects_malformed_arguments_without_repeating_them() {
    let dir = TempDir::new("strength-args");
    let app = app_with(test_parts(&dir));

    for args in [
        json!({}),
        json!({ "password": 123_456 }),
        json!({ "password": null }),
        json!({ "password": [CANARY] }),
        json!({ "password": CANARY, "user_inputs": [CANARY] }),
        json!({ "Password": CANARY }),
    ] {
        let answer = invoke_args(&app, "password_strength", args);
        assert_eq!(error_code(&answer), "ArgumentoInvalido");
        let text = serde_json::to_string(&answer.unwrap_err()).unwrap();
        assert!(
            !text.contains(CANARY),
            "the rejected password leaked: {text}"
        );
    }
}

// ---------- Generator (RF-65 to RF-70) ----------

fn rules(length: u32, lower: bool, upper: bool, digits: bool, symbols: bool, avoid: bool) -> Value {
    json!({
        "length": length,
        "lower": lower,
        "upper": upper,
        "digits": digits,
        "symbols": symbols,
        "avoid_lookalikes": avoid,
    })
}

#[test]
fn rf_65_generate_password_answers_a_password_of_the_asked_length_and_its_score() {
    let dir = TempDir::new("generate");
    let app = app_with(test_parts(&dir));

    for length in [8_u32, 20, 128] {
        let answer = invoke_args(
            &app,
            "generate_password",
            rules(length, true, true, true, true, false),
        )
        .expect("a password");
        let password = answer["password"].as_str().expect("the password");
        assert_eq!(password.chars().count(), usize::try_from(length).unwrap());
        // RF-66: at least one of each chosen type.
        for class in [LOWERCASE, UPPERCASE, DIGITS, SYMBOLS] {
            assert!(password.chars().any(|c| class.contains(c)), "{length}");
        }
        let score = answer["score"].as_u64().expect("the score");
        assert!(score <= 4);
        assert_eq!(
            answer.as_object().unwrap().len(),
            2,
            "exactly password and score"
        );
    }

    let strong = invoke_args(
        &app,
        "generate_password",
        rules(20, true, true, true, true, false),
    )
    .expect("a password");
    assert_eq!(
        strong["score"],
        json!(4),
        "spec anchor: 20 random characters of the four types"
    );
}

#[test]
fn rf_67_generate_password_uses_the_field_names_the_window_sends() {
    let dir = TempDir::new("generate-names");
    let app = app_with(test_parts(&dir));

    // Worked example 5 of the spec: length 8, the four types, no look-alikes.
    let answer = invoke_args(
        &app,
        "generate_password",
        rules(8, true, true, true, true, true),
    )
    .expect("a password");
    let password = answer["password"].as_str().unwrap();
    assert_eq!(password.chars().count(), 8);
    assert!(!password.chars().any(|c| LOOKALIKES.contains(c)));

    // Only digits: `lower`/`upper`/`symbols` off must really be off.
    let digits = invoke_args(
        &app,
        "generate_password",
        rules(32, false, false, true, false, false),
    )
    .expect("a password");
    assert!(
        digits["password"]
            .as_str()
            .unwrap()
            .chars()
            .all(|c| DIGITS.contains(c))
    );

    // The core's own names (`lowercase`, `uppercase`) are not the contract: unknown fields.
    let answer = invoke_args(
        &app,
        "generate_password",
        json!({
            "length": 20, "lowercase": true, "uppercase": true, "digits": true, "symbols": true,
            "avoid_lookalikes": false,
        }),
    );
    assert_eq!(error_code(&answer), "ArgumentoInvalido");
}

#[test]
fn rf_68_impossible_rules_generate_nothing() {
    let dir = TempDir::new("generate-impossible");
    let app = app_with(test_parts(&dir));

    for args in [
        rules(20, false, false, false, false, false),
        rules(7, true, true, true, true, false),
        rules(0, true, true, true, true, false),
        rules(129, true, true, true, true, false),
        rules(u32::MAX, true, true, true, true, false),
    ] {
        let answer = invoke_args(&app, "generate_password", args.clone());
        assert_eq!(error_code(&answer), "ReglasImposibles", "{args}");
        assert!(answer.unwrap_err().get("password").is_none());
    }
}

#[test]
fn rf_80_generate_password_rejects_malformed_arguments_before_any_rule() {
    let dir = TempDir::new("generate-args");
    let app = app_with(test_parts(&dir));
    let mut missing = rules(20, true, true, true, true, false);
    missing.as_object_mut().unwrap().remove("symbols");
    let mut extra = rules(20, true, true, true, true, false);
    extra["seed"] = json!(1);
    let mut wrong_and_impossible = rules(20, false, false, false, false, false);
    wrong_and_impossible["length"] = json!("20");

    for args in [
        missing,
        extra,
        // Precedence of the spec: `ArgumentoInvalido` wins over the rule of the domain.
        wrong_and_impossible,
        json!({ "length": -8, "lower": true, "upper": true, "digits": true, "symbols": true, "avoid_lookalikes": false }),
        json!({ "length": 20.5, "lower": true, "upper": true, "digits": true, "symbols": true, "avoid_lookalikes": false }),
        json!({ "length": 20, "lower": 1, "upper": true, "digits": true, "symbols": true, "avoid_lookalikes": false }),
        json!({ "length": 20, "lower": "true", "upper": true, "digits": true, "symbols": true, "avoid_lookalikes": false }),
    ] {
        let answer = invoke_args(&app, "generate_password", args.clone());
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "{args}");
    }
    assert_eq!(
        error_code(&invoke_plain(&app, "generate_password")),
        "ArgumentoInvalido"
    );
}
