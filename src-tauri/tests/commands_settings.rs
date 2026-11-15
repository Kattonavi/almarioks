//! Settings and recent vaults through the commands (RF-19, RF-20, RF-29, RF-73, RF-82), over a temporary settings
//! file. The rules themselves are tested in `almarioks-core` (`rf_settings.rs`); here, that the commands apply
//! them to the right file, all or nothing.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;

use almarioks_core::settings::Settings;
use almarioks_lib::state::{SETTINGS_FILE_NAME, SettingsStore};
use common::{TempDir, app_with, error_code, invoke_args, invoke_plain, test_parts, test_plugins};
use serde_json::{Value, json};
use tauri::{Manager, test::mock_builder};

fn defaults() -> Value {
    json!({ "auto_lock_minutes": 5, "clipboard_seconds": 20 })
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("the test paths are valid text")
}

/// Writes a settings file that remembers `vaults`, the first one the most recent.
fn remember(file: &Path, vaults: &[&Path]) {
    let mut settings = Settings::default();
    for vault in vaults.iter().rev() {
        settings.remember_vault(vault);
    }
    settings.save(file).expect("write the settings file");
}

#[test]
fn rf_82_settings_live_in_the_configuration_folder_tauri_gives() {
    let app = almarioks_lib::app_builder(mock_builder(), test_plugins())
        .build(almarioks_lib::context())
        .expect("build the test app");

    // Only the path is resolved: nothing is read or written in the person's real folder.
    let parts = almarioks_lib::system_parts(app.handle()).expect("the system parts");
    let folder = app
        .path()
        .app_config_dir()
        .expect("the configuration folder");

    assert_eq!(parts.settings_file, folder.join(SETTINGS_FILE_NAME));
    assert!(
        path_text(&folder).ends_with("io.github.kattonavi.almarioks"),
        "the folder is the app's own, named after its identifier"
    );
}

#[test]
fn rf_82_without_a_file_the_settings_are_the_defaults_and_nothing_is_written() {
    let dir = TempDir::new("rf82-missing");
    let app = app_with(test_parts(&dir));

    assert_eq!(invoke_plain(&app, "settings_get"), Ok(defaults()));
    assert_eq!(invoke_plain(&app, "recent_list"), Ok(json!([])));
    assert!(
        !dir.settings_file().exists(),
        "reading must not create the file"
    );
}

#[test]
fn rf_82_a_damaged_file_gives_the_default_of_each_bad_field() {
    let dir = TempDir::new("rf82-damaged");
    std::fs::write(dir.settings_file(), b"{ this is not JSON").unwrap();
    let app = app_with(test_parts(&dir));
    assert_eq!(invoke_plain(&app, "settings_get"), Ok(defaults()));
    drop(app);

    // Edited by hand: one field out of range, one fine. Each stands on its own.
    std::fs::write(
        dir.settings_file(),
        br#"{ "auto_lock_minutes": 0, "clipboard_seconds": 45, "recent_vaults": "nope" }"#,
    )
    .unwrap();
    let app = app_with(test_parts(&dir));
    assert_eq!(
        invoke_plain(&app, "settings_get"),
        Ok(json!({ "auto_lock_minutes": 5, "clipboard_seconds": 45 }))
    );
    assert_eq!(invoke_plain(&app, "recent_list"), Ok(json!([])));
}

#[test]
fn rf_82_settings_set_writes_the_file_and_the_next_start_reads_it() {
    let dir = TempDir::new("rf82-set");
    let app = app_with(test_parts(&dir));
    let chosen = json!({ "auto_lock_minutes": 15, "clipboard_seconds": 60 });

    assert_eq!(
        invoke_args(&app, "settings_set", chosen.clone()),
        Ok(chosen.clone())
    );
    assert_eq!(invoke_plain(&app, "settings_get"), Ok(chosen.clone()));
    drop(app);

    let on_disk = Settings::load(&dir.settings_file());
    assert_eq!(
        (on_disk.auto_lock_minutes(), on_disk.clipboard_seconds()),
        (15, 60)
    );
    let restarted = app_with(test_parts(&dir));
    assert_eq!(invoke_plain(&restarted, "settings_get"), Ok(chosen));
}

#[test]
fn rf_29_rf_73_settings_set_accepts_only_the_values_of_the_spec() {
    let dir = TempDir::new("rf29-values");
    let app = app_with(test_parts(&dir));

    for minutes in [1, 5, 15, 30] {
        for seconds in [10, 20, 120] {
            let chosen = json!({ "auto_lock_minutes": minutes, "clipboard_seconds": seconds });
            assert_eq!(
                invoke_args(&app, "settings_set", chosen.clone()),
                Ok(chosen)
            );
        }
    }
    let last = invoke_plain(&app, "settings_get");

    // There is no "never" (0) and no other choice of minutes; the clipboard time is 10 to 120 seconds.
    for (minutes, seconds) in [
        (0, 20),
        (2, 20),
        (10, 20),
        (31, 20),
        (60, 20),
        (5, 9),
        (5, 0),
        (5, 121),
    ] {
        let answer = invoke_args(
            &app,
            "settings_set",
            json!({ "auto_lock_minutes": minutes, "clipboard_seconds": seconds }),
        );
        assert_eq!(
            error_code(&answer),
            "ArgumentoInvalido",
            "{minutes} min, {seconds} s"
        );
    }
    assert_eq!(invoke_plain(&app, "settings_get"), last);
}

#[test]
fn rf_82_settings_set_is_all_or_nothing() {
    let dir = TempDir::new("rf82-atomic");
    let app = app_with(test_parts(&dir));
    let before = json!({ "auto_lock_minutes": 30, "clipboard_seconds": 90 });
    invoke_args(&app, "settings_set", before.clone()).expect("a valid change");
    let file_before = std::fs::read(dir.settings_file()).unwrap();

    // The first field is valid and the second is not: the first must not stick, in memory or on disk.
    let answer = invoke_args(
        &app,
        "settings_set",
        json!({ "auto_lock_minutes": 1, "clipboard_seconds": 5 }),
    );
    assert_eq!(error_code(&answer), "ArgumentoInvalido");
    assert_eq!(invoke_plain(&app, "settings_get"), Ok(before));
    assert_eq!(std::fs::read(dir.settings_file()).unwrap(), file_before);
}

#[test]
fn rf_82_a_failed_write_changes_nothing_and_says_so() {
    let dir = TempDir::new("rf82-write-fails");
    // The settings "file" is inside something that is a file, not a folder: it can never be written.
    let blocker = dir.path().join("not-a-folder");
    std::fs::write(&blocker, b"x").unwrap();
    let store = SettingsStore::new(blocker.join(SETTINGS_FILE_NAME));

    let result = store.update(|settings| settings.set_auto_lock_minutes(30));

    assert_eq!(
        result.map(|_| ()),
        Err(almarioks_core::CoreError::GuardadoFallido)
    );
    assert_eq!(
        store.read().auto_lock_minutes(),
        5,
        "memory must not run ahead of the file"
    );
}

#[test]
fn rf_19_rf_20_recent_list_tells_which_vaults_are_still_there() {
    let dir = TempDir::new("rf20-list");
    let here = dir.path().join("personal.kdbx");
    let gone = dir.path().join("trabajo.kdbx");
    std::fs::write(&here, b"not a real vault").unwrap();
    remember(&dir.settings_file(), &[&gone, &here]);
    let app = app_with(test_parts(&dir));

    assert_eq!(
        invoke_plain(&app, "recent_list"),
        Ok(json!([
            { "path": path_text(&gone), "exists": false },
            { "path": path_text(&here), "exists": true },
        ]))
    );

    // RF-19: the settings file remembers paths and times, and nothing that looks like a key.
    let file: Value = serde_json::from_slice(&std::fs::read(dir.settings_file()).unwrap()).unwrap();
    let keys: Vec<&String> = file.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        ["auto_lock_minutes", "clipboard_seconds", "recent_vaults"]
    );
}

#[test]
fn rf_20_recent_remove_takes_one_vault_out_and_answers_the_rest() {
    let dir = TempDir::new("rf20-remove");
    let here = dir.path().join("personal.kdbx");
    let gone = dir.path().join("trabajo.kdbx");
    std::fs::write(&here, b"not a real vault").unwrap();
    remember(&dir.settings_file(), &[&gone, &here]);
    let app = app_with(test_parts(&dir));

    let left = json!([{ "path": path_text(&here), "exists": true }]);
    assert_eq!(
        invoke_args(&app, "recent_remove", json!({ "path": path_text(&gone) })),
        Ok(left.clone())
    );
    assert_eq!(invoke_plain(&app, "recent_list"), Ok(left.clone()));
    assert_eq!(
        Settings::load(&dir.settings_file()).recent_vaults(),
        std::slice::from_ref(&here)
    );
    assert!(
        here.exists(),
        "removing from the list never touches the vault file"
    );

    // A path that is not in the list changes nothing; an empty one is not a path.
    let file_before = std::fs::read(dir.settings_file()).unwrap();
    assert_eq!(
        invoke_args(&app, "recent_remove", json!({ "path": path_text(&gone) })),
        Ok(left)
    );
    assert_eq!(std::fs::read(dir.settings_file()).unwrap(), file_before);
    let answer = invoke_args(&app, "recent_remove", json!({ "path": "" }));
    assert_eq!(error_code(&answer), "ArgumentoInvalido");
    let answer = invoke_args(&app, "recent_remove", json!({ "path": 7 }));
    assert_eq!(error_code(&answer), "ArgumentoInvalido");
}
