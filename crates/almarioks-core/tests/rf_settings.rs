//! Settings and recent vaults through the public API (RF-19, RF-20, RF-82, values of RF-29 and RF-73).
//!
//! Files go to a fresh folder under the system temp dir, removed at the end.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use almarioks_core::{
    CoreError,
    settings::{RecentVault, Settings},
};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!(
            "almarioks-settings-test-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn vault(n: usize) -> PathBuf {
    PathBuf::from(format!("bovedas/cuentas-{n}.kdbx"))
}

#[test]
fn rf_82_defaults_are_5_minutes_20_seconds_and_no_recent_vaults() {
    let settings = Settings::default();
    assert_eq!(settings.auto_lock_minutes(), 5);
    assert_eq!(settings.clipboard_seconds(), 20);
    assert!(settings.recent_vaults().is_empty());
}

#[test]
fn rf_82_damaged_files_give_the_defaults_without_failing() {
    let damaged: [&[u8]; 8] = [
        b"",
        b"{",
        b"not json at all",
        b"[1, 2, 3]",
        b"null",
        b"\xff\xfe\x00\x01",
        b"{\"auto_lock_minutes\": 15, \"clipboard_seconds\": 30, \"recent_vaults\": [\"a.kdbx\"",
        b"\"auto_lock_minutes\"",
    ];
    for bytes in damaged {
        assert_eq!(
            Settings::from_bytes(bytes),
            Settings::default(),
            "{bytes:?}"
        );
    }
}

#[test]
fn rf_82_a_damaged_file_on_disk_loads_as_defaults_and_a_missing_one_too() {
    let dir = TempDir::new();
    let path = dir.file("settings.json");
    assert_eq!(Settings::load(&path), Settings::default());
    fs::write(&path, b"{ this is not json").unwrap();
    assert_eq!(Settings::load(&path), Settings::default());
    fs::write(&path, [0_u8, 159, 146, 150]).unwrap();
    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn rf_29_rf_82_auto_lock_accepts_only_1_5_15_30_and_each_field_falls_back_alone() {
    for minutes in [1, 5, 15, 30] {
        let text = format!("{{\"auto_lock_minutes\": {minutes}}}");
        assert_eq!(
            Settings::from_bytes(text.as_bytes()).auto_lock_minutes(),
            minutes
        );
    }
    for bad in [
        "0",
        "2",
        "10",
        "60",
        "-5",
        "5.5",
        "5.0",
        "\"5\"",
        "null",
        "true",
        "[5]",
        "99999999999999",
    ] {
        // The invalid field gives 5, but the valid clipboard time next to it is kept.
        let text = format!("{{\"auto_lock_minutes\": {bad}, \"clipboard_seconds\": 45}}");
        let settings = Settings::from_bytes(text.as_bytes());
        assert_eq!(settings.auto_lock_minutes(), 5, "{bad}");
        assert_eq!(settings.clipboard_seconds(), 45, "{bad}");
    }
}

#[test]
fn rf_73_rf_82_clipboard_seconds_accepts_10_to_120_and_each_field_falls_back_alone() {
    for seconds in [10, 11, 20, 119, 120] {
        let text = format!("{{\"clipboard_seconds\": {seconds}}}");
        assert_eq!(
            Settings::from_bytes(text.as_bytes()).clipboard_seconds(),
            seconds
        );
    }
    for bad in [
        "0", "9", "121", "100000", "-20", "20.5", "\"20\"", "null", "{}",
    ] {
        let text = format!("{{\"clipboard_seconds\": {bad}, \"auto_lock_minutes\": 30}}");
        let settings = Settings::from_bytes(text.as_bytes());
        assert_eq!(settings.clipboard_seconds(), 20, "{bad}");
        assert_eq!(settings.auto_lock_minutes(), 30, "{bad}");
    }
}

#[test]
fn rf_82_unknown_fields_are_ignored_and_the_known_ones_are_kept() {
    let text = r#"{"auto_lock_minutes": 15, "theme": "dark", "clipboard_seconds": 60,
        "recent_vaults": ["a.kdbx"], "future": {"x": [1, 2]}}"#;
    let settings = Settings::from_bytes(text.as_bytes());
    assert_eq!(settings.auto_lock_minutes(), 15);
    assert_eq!(settings.clipboard_seconds(), 60);
    assert_eq!(settings.recent_vaults(), [PathBuf::from("a.kdbx")]);
}

#[test]
fn rf_82_setters_refuse_values_the_file_would_refuse() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.set_auto_lock_minutes(10),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!(
        settings.set_auto_lock_minutes(0),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!(
        settings.set_clipboard_seconds(9),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!(
        settings.set_clipboard_seconds(121),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!(settings.auto_lock_minutes(), 5);
    assert_eq!(settings.clipboard_seconds(), 20);
    settings.set_auto_lock_minutes(30).unwrap();
    settings.set_clipboard_seconds(120).unwrap();
    assert_eq!(
        (settings.auto_lock_minutes(), settings.clipboard_seconds()),
        (30, 120)
    );
}

#[test]
fn rf_19_six_vaults_leave_the_last_five_most_recent_first() {
    let mut settings = Settings::default();
    for n in 1..=6 {
        settings.remember_vault(&vault(n));
    }
    let expected: Vec<PathBuf> = (2..=6).rev().map(vault).collect();
    assert_eq!(settings.recent_vaults(), expected);
}

#[test]
fn rf_19_opening_a_known_vault_again_moves_it_first_without_duplicating() {
    let mut settings = Settings::default();
    for n in 1..=3 {
        settings.remember_vault(&vault(n));
    }
    settings.remember_vault(&vault(1));
    settings.remember_vault(&vault(1));
    assert_eq!(settings.recent_vaults(), [vault(1), vault(3), vault(2)]);
}

#[test]
fn rf_19_a_hand_edited_file_with_duplicates_or_more_than_five_is_cleaned() {
    let text = r#"{"recent_vaults": ["a", "b", "a", "c", "d", "e", "f", "g", "", 7, null, "b"]}"#;
    let settings = Settings::from_bytes(text.as_bytes());
    let expected: Vec<PathBuf> = ["a", "b", "c", "d", "e"].map(PathBuf::from).to_vec();
    assert_eq!(settings.recent_vaults(), expected);
    let not_a_list =
        Settings::from_bytes(br#"{"recent_vaults": "a.kdbx", "clipboard_seconds": 90}"#);
    assert!(not_a_list.recent_vaults().is_empty());
    assert_eq!(not_a_list.clipboard_seconds(), 90);
}

#[test]
fn rf_19_the_file_holds_only_the_three_settings_never_a_key_or_a_password() {
    let mut settings = Settings::default();
    settings.remember_vault(&vault(1));
    let value: serde_json::Value = serde_json::from_str(&settings.to_text()).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["auto_lock_minutes", "clipboard_seconds", "recent_vaults"]
    );
}

#[test]
fn rf_20_each_recent_vault_says_whether_it_still_exists_and_forget_removes_it() {
    let mut settings = Settings::default();
    for n in 1..=3 {
        settings.remember_vault(&vault(n));
    }
    let there = vault(2);
    let status = settings.recent_with_status(|path| path == there.as_path());
    assert_eq!(
        status,
        [
            RecentVault {
                path: vault(3),
                exists: false
            },
            RecentVault {
                path: vault(2),
                exists: true
            },
            RecentVault {
                path: vault(1),
                exists: false
            },
        ]
    );
    settings.forget_vault(&vault(3));
    settings.forget_vault(Path::new("never/remembered.kdbx"));
    assert_eq!(settings.recent_vaults(), [vault(2), vault(1)]);
}

#[test]
fn rf_20_status_uses_the_real_disk_when_asked_to() {
    let dir = TempDir::new();
    let present = dir.file("cuentas.kdbx");
    fs::write(&present, b"x").unwrap();
    let gone = dir.file("borrada.kdbx");
    let mut settings = Settings::default();
    settings.remember_vault(&gone);
    settings.remember_vault(&present);
    let status = settings.recent_with_status(Path::exists);
    assert_eq!(
        status.iter().map(|v| v.exists).collect::<Vec<_>>(),
        [true, false]
    );
}

#[test]
fn rf_82_round_trip_through_text_and_through_disk() {
    let mut settings = Settings::default();
    settings.set_auto_lock_minutes(15).unwrap();
    settings.set_clipboard_seconds(75).unwrap();
    for n in 1..=4 {
        settings.remember_vault(&vault(n));
    }
    settings.remember_vault(Path::new("carpeta con espacios/ñandú €.kdbx"));
    assert_eq!(
        Settings::from_bytes(settings.to_text().as_bytes()),
        settings
    );

    let dir = TempDir::new();
    let path = dir.file("config").join("settings.json");
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path), settings);
    assert!(!dir.file("config").join("settings.json.tmp").exists());

    // Saving again replaces the file.
    settings.forget_vault(&vault(4));
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path), settings);
}

#[test]
fn rf_82_save_that_cannot_write_reports_guardado_fallido_and_leaves_the_old_file() {
    let dir = TempDir::new();
    let path = dir.file("settings.json");
    let mut settings = Settings::default();
    settings.set_clipboard_seconds(60).unwrap();
    settings.save(&path).unwrap();
    // A folder where the temporary file should go makes the write fail.
    fs::create_dir(dir.file("settings.json.tmp")).unwrap();
    let other = Settings::default();
    assert_eq!(other.save(&path), Err(CoreError::GuardadoFallido));
    assert_eq!(Settings::load(&path), settings);
}

#[test]
fn rf_19_empty_paths_are_not_remembered() {
    let mut settings = Settings::default();
    settings.remember_vault(Path::new(""));
    assert!(settings.recent_vaults().is_empty());
}
