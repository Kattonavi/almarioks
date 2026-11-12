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

// ---------- RF-111: the key file remembered for a recent vault ----------
// The conditions C-1 to C-10 are those of the security review of 0.1, second round of the owner.

/// A key file path that may be remembered. Nothing is at it: these tests never touch the disk.
const KEY_FILE: &str = r"E:\llaves\cuentas.keyx";

fn key_files_in(settings: &Settings) -> Vec<Option<PathBuf>> {
    settings
        .recent_vaults()
        .iter()
        .map(|vault| settings.key_file(vault).map(Path::to_path_buf))
        .collect()
}

#[test]
fn rf_111_c2_only_a_full_path_on_a_lettered_drive_may_be_remembered() {
    use almarioks_core::settings::{MAX_KEY_FILE_PATH_CHARS, is_rememberable_key_file};

    // The limit counts characters, not bytes: `ñ` takes two bytes.
    assert_eq!(MAX_KEY_FILE_PATH_CHARS, 1024);
    let at_the_limit = format!(r"C:\{}", "ñ".repeat(1021));
    let over_the_limit = format!(r"C:\{}", "ñ".repeat(1022));
    assert_eq!(at_the_limit.chars().count(), 1024);

    let accepted = [
        r"C:\llaves\cuentas.keyx",
        "d:/llaves/cuentas.keyx",
        r"E:\cuentas.keyx",
        r"C:\carpeta con espacios\ñandú.key",
        r"Z:\a\..\b.keyx",
        r"C:\llaves/mezcla\cuentas",
        at_the_limit.as_str(),
    ];
    for path in accepted {
        assert!(is_rememberable_key_file(path), "refused {path:?}");
    }

    let refused = [
        // Nothing, or no file name at the end.
        "",
        "C:",
        r"C:\",
        "C:/",
        r"C:\llaves\",
        r"C:\llaves\.",
        r"C:\llaves\..",
        // Another machine.
        r"\\servidor\recurso\cuentas.keyx",
        "//servidor/recurso/cuentas.keyx",
        r"\/servidor/recurso/cuentas.keyx",
        // Verbatim and device paths, also when they name a local drive.
        r"\\?\C:\llaves\cuentas.keyx",
        r"\\?\UNC\servidor\recurso\cuentas.keyx",
        r"\\.\C:\llaves\cuentas.keyx",
        r"\\.\pipe\cuentas",
        r"\??\C:\llaves\cuentas.keyx",
        // Relative to the folder or to the drive the app happens to be in.
        "cuentas.keyx",
        r"llaves\cuentas.keyx",
        r".\cuentas.keyx",
        r"..\cuentas.keyx",
        r"\llaves\cuentas.keyx",
        "/llaves/cuentas.keyx",
        "C:cuentas.keyx",
        r"C:llaves\cuentas.keyx",
        // A `:` anywhere but after the drive letter: alternate streams and the like.
        r"C:\llaves\cuentas.keyx:flujo",
        r"C:\llaves:flujo\cuentas.keyx",
        r"C:\llaves\cuentas.keyx::$DATA",
        r"C:\D:\cuentas.keyx",
        "file:///C:/llaves/cuentas.keyx",
        // The drive is one ASCII letter, at the very start.
        r"1:\cuentas.keyx",
        r"é:\cuentas.keyx",
        r"CC:\cuentas.keyx",
        r" C:\cuentas.keyx",
        over_the_limit.as_str(),
    ];
    for path in refused {
        assert!(!is_rememberable_key_file(path), "accepted {path:?}");
    }
}

#[test]
fn rf_111_a_key_file_is_remembered_only_for_a_recent_vault_and_goes_away_with_it() {
    let mut settings = Settings::default();
    let key = Path::new(KEY_FILE);

    // Not a recent vault: nothing to hang the key file on.
    settings.set_key_file(&vault(1), Some(key));
    assert_eq!(settings.key_file(&vault(1)), None);
    assert_eq!(settings, Settings::default());

    settings.remember_vault(&vault(1));
    assert_eq!(settings.key_file(&vault(1)), None);
    settings.set_key_file(&vault(1), Some(key));
    assert_eq!(settings.key_file(&vault(1)), Some(key));

    // C-1: the vault moving first in the list again (what creating, converting and saving as do) changes nothing.
    settings.remember_vault(&vault(2));
    settings.remember_vault(&vault(1));
    assert_eq!(key_files_in(&settings), [Some(key.to_path_buf()), None]);

    // C-7 (c): another key file takes its place. (b): none forgets it.
    let other = Path::new(r"F:\otra.key");
    settings.set_key_file(&vault(1), Some(other));
    assert_eq!(settings.key_file(&vault(1)), Some(other));
    settings.set_key_file(&vault(1), None);
    assert_eq!(settings.key_file(&vault(1)), None);

    // C-2: a path that may not be remembered is not, and what was remembered goes.
    settings.set_key_file(&vault(1), Some(key));
    settings.set_key_file(
        &vault(1),
        Some(Path::new(r"\\servidor\recurso\cuentas.keyx")),
    );
    assert_eq!(settings.key_file(&vault(1)), None);

    // C-7 (a): removed from the list, and coming back later, it has none.
    settings.set_key_file(&vault(1), Some(key));
    settings.forget_vault(&vault(1));
    assert_eq!(settings.key_file(&vault(1)), None);
    settings.remember_vault(&vault(1));
    assert_eq!(settings.key_file(&vault(1)), None);

    // C-7 (a): the sixth vault pushes the oldest out, with its key file.
    settings.set_key_file(&vault(1), Some(key));
    settings.set_key_file(&vault(2), Some(other));
    for n in 3..=6 {
        settings.remember_vault(&vault(n));
    }
    assert_eq!(settings.recent_vaults().len(), 5);
    assert!(!settings.recent_vaults().contains(&vault(2)));
    assert_eq!(settings.key_file(&vault(2)), None);
    assert_eq!(settings.key_file(&vault(1)), Some(key));
    assert!(!settings.to_text().contains("otra.key"));
}

#[test]
fn rf_111_the_file_gets_a_fourth_key_only_while_a_key_file_is_remembered() {
    let mut settings = Settings::default();
    settings.remember_vault(&vault(1));
    settings.remember_vault(&vault(2));
    let keys_of = |settings: &Settings| -> Vec<String> {
        let value: serde_json::Value = serde_json::from_str(&settings.to_text()).unwrap();
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort_unstable();
        keys
    };
    let three = ["auto_lock_minutes", "clipboard_seconds", "recent_vaults"];
    assert_eq!(keys_of(&settings), three);

    settings.set_key_file(&vault(1), Some(Path::new(KEY_FILE)));
    assert_eq!(
        keys_of(&settings),
        [
            "auto_lock_minutes",
            "clipboard_seconds",
            "key_files",
            "recent_vaults"
        ]
    );
    // C-3: a text for a text, the path and nothing else about the file.
    let value: serde_json::Value = serde_json::from_str(&settings.to_text()).unwrap();
    assert_eq!(
        value["key_files"],
        serde_json::json!({ vault(1).to_str().unwrap(): KEY_FILE })
    );
    // What is written is what is read back.
    assert_eq!(
        Settings::from_bytes(settings.to_text().as_bytes()),
        settings
    );

    settings.set_key_file(&vault(1), None);
    assert_eq!(keys_of(&settings), three);
}

#[test]
fn rf_111_c4_what_the_file_says_is_validated_like_what_is_set() {
    let recent: Vec<String> = (1..=5)
        .map(|n| vault(n).to_str().unwrap().to_owned())
        .collect();
    let file = |key_files: serde_json::Value| -> Settings {
        let text =
            serde_json::json!({ "recent_vaults": recent, "key_files": key_files }).to_string();
        Settings::from_bytes(text.as_bytes())
    };

    // One good entry among everything a hand-edited file can hold.
    let settings = file(serde_json::json!({
        recent[0].as_str(): r"\\servidor\recurso\cuentas.keyx",
        recent[1].as_str(): KEY_FILE,
        recent[2].as_str(): r"\\?\C:\llaves\cuentas.keyx",
        recent[3].as_str(): r"llaves\cuentas.keyx",
        recent[4].as_str(): 7,
        "bovedas/no-es-reciente.kdbx": KEY_FILE,
    }));
    assert_eq!(
        key_files_in(&settings),
        [None, Some(PathBuf::from(KEY_FILE)), None, None, None]
    );
    assert_eq!(
        settings.key_file(Path::new("bovedas/no-es-reciente.kdbx")),
        None
    );
    // The rest of the file is read as always, and what was dropped is not written again.
    assert_eq!(settings.recent_vaults().len(), 5);
    let written: serde_json::Value = serde_json::from_str(&settings.to_text()).unwrap();
    assert_eq!(
        written["key_files"],
        serde_json::json!({ recent[1].as_str(): KEY_FILE })
    );

    // Values that are not a text, and a `key_files` that is not an object: nothing remembered, nothing fails.
    for odd in [
        serde_json::json!({ recent[0].as_str(): null, recent[1].as_str(): [KEY_FILE], recent[2].as_str(): { "path": KEY_FILE }, recent[3].as_str(): true, recent[4].as_str(): "" }),
        serde_json::json!([KEY_FILE]),
        serde_json::json!(KEY_FILE),
        serde_json::json!(null),
        serde_json::json!(5),
    ] {
        let settings = file(odd.clone());
        assert_eq!(
            key_files_in(&settings),
            [None, None, None, None, None],
            "{odd}"
        );
        assert_eq!(settings.recent_vaults().len(), 5, "{odd}");
    }

    // A file from before RF-111 has no `key_files`: none remembered.
    let old = Settings::from_bytes(
        br#"{"recent_vaults": ["bovedas/cuentas-1.kdbx"], "clipboard_seconds": 90}"#,
    );
    assert_eq!(key_files_in(&old), [None]);
    assert_eq!(old.clipboard_seconds(), 90);
    // And an entry that comes before its vault in the file is still tied to the recent list.
    let any_order = Settings::from_bytes(
        br#"{"key_files": {"bovedas/cuentas-1.kdbx": "E:\\llaves\\cuentas.keyx"}, "recent_vaults": ["bovedas/cuentas-1.kdbx"]}"#,
    );
    assert_eq!(key_files_in(&any_order), [Some(PathBuf::from(KEY_FILE))]);
}
