//! Where a new vault is proposed (RF-97) and where its key file goes (RF-99), through the public API. Pure rules:
//! no test here touches the disk, and "Documents" is an invented folder.
//!
//! `fixtures/location/file-names.json` is the table of names and file names; the window's fake core
//! (`src/ipc/fake.ts`) is tested against the same file, so both follow one rule.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use almarioks_core::location::{
    DEFAULT_FILE_STEM, DEFAULT_FOLDER_NAME, MAX_FILE_STEM_CHARS, VaultLocation, default_folder,
    file_stem, is_in_default_folder, key_file_name, vault_file_name,
};
use serde_json::Value;

const FILE_NAMES: &str = include_str!("fixtures/location/file-names.json");

/// An invented Documents folder: nothing is read or written there.
fn documents() -> PathBuf {
    Path::new("documentos-de-prueba").join("Documentos")
}

fn cases() -> Vec<(String, String, String)> {
    let table: Value = serde_json::from_str(FILE_NAMES).expect("the table is JSON");
    table
        .as_array()
        .expect("a list of cases")
        .iter()
        .map(|case| {
            let text = |key: &str| case[key].as_str().expect("a text").to_owned();
            (text("why"), text("name"), text("stem"))
        })
        .collect()
}

// ---------- RF-97: the file name comes from the vault's name ----------

#[test]
fn rf_97_the_file_name_follows_the_table_of_cases() {
    let cases = cases();
    assert!(cases.len() >= 20, "the table lost its cases");
    for (why, name, stem) in cases {
        assert_eq!(file_stem(&name), stem, "{why}: {name:?}");
        assert_eq!(vault_file_name(&name), format!("{stem}.kdbx"), "{why}");
    }
}

#[test]
fn rf_97_a_name_that_leaves_nothing_is_boveda() {
    assert_eq!(DEFAULT_FILE_STEM, "bóveda");
    for name in ["", " ", "\t\n", "\\/:*?\"<>|", ". . .", "\u{0}\u{1b}"] {
        assert_eq!(vault_file_name(name), "bóveda.kdbx", "{name:?}");
    }
}

#[test]
fn rf_97_the_file_name_is_cut_to_the_limit_by_characters() {
    assert_eq!(MAX_FILE_STEM_CHARS, 100);
    assert_eq!(file_stem(&"a".repeat(100)), "a".repeat(100));
    assert_eq!(file_stem(&"a".repeat(101)), "a".repeat(100));
    // By characters, never by bytes or UTF-16 units: no letter is split.
    assert_eq!(file_stem(&"ñ".repeat(250)), "ñ".repeat(100));
    assert_eq!(file_stem(&"🔑".repeat(250)), "🔑".repeat(100));
    // The cut can leave a space or a dot at the end: it goes too.
    let cut_at_a_space = format!("{} final", "a".repeat(99));
    assert_eq!(file_stem(&cut_at_a_space), "a".repeat(99));
    let cut_at_a_dot = format!("{}.final", "a".repeat(99));
    assert_eq!(file_stem(&cut_at_a_dot), "a".repeat(99));
    // And it can leave a device name: it is not used as it is either.
    let cut_to_a_device = format!("NUL{}x", " ".repeat(97));
    assert_eq!(file_stem(&cut_to_a_device), "NUL_");
    // The characters that are removed do not count towards the limit.
    let padded = format!("{}{}", "?".repeat(300), "b".repeat(100));
    assert_eq!(file_stem(&padded), "b".repeat(100));
}

#[test]
fn rf_97_whatever_the_name_the_file_name_is_one_component_windows_accepts() {
    let mut names: Vec<String> = cases().into_iter().map(|(_, name, _)| name).collect();
    // Every character up to U+00FF alone, doubled, and between letters.
    for c in (0..=0xFF_u32).filter_map(char::from_u32) {
        names.extend([
            format!("{c}"),
            format!("{c}{c}"),
            format!("a{c}b"),
            format!("{c}.{c}"),
        ]);
    }
    for device in ["con", "PRN", "Aux", "nul", "COM0", "com9", "LPT1", "CONIN$"] {
        names.extend([
            device.to_owned(),
            format!(" {device} "),
            format!("{device}."),
            format!("{device}.kdbx"),
            format!("{device} .a.b"),
            format!("{device}:"),
        ]);
    }

    for name in names {
        let stem = file_stem(&name);
        assert!(!stem.is_empty(), "{name:?}");
        assert!(stem.chars().count() <= MAX_FILE_STEM_CHARS + 1, "{name:?}");
        assert!(
            !stem.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']),
            "{name:?} → {stem:?}"
        );
        assert!(!stem.chars().any(char::is_control), "{name:?} → {stem:?}");
        assert_eq!(stem.trim_start(), stem, "{name:?} → {stem:?}");
        assert!(!stem.ends_with(['.', ' ']), "{name:?} → {stem:?}");
        let base = stem
            .split('.')
            .next()
            .unwrap()
            .trim_end()
            .to_ascii_uppercase();
        assert!(
            !["CON", "PRN", "AUX", "NUL", "COM0", "COM9", "LPT1", "CONIN$"]
                .contains(&base.as_str()),
            "{name:?} → {stem:?}"
        );
        // One component: joined to a folder, it stays directly inside it.
        let file = vault_file_name(&name);
        let path = documents().join(&file);
        assert_eq!(path.parent(), Some(documents().as_path()), "{name:?}");
        assert_eq!(
            path.file_name().and_then(|f| f.to_str()),
            Some(file.as_str())
        );
    }
}

#[test]
fn rf_97_the_proposed_location_is_documents_almarioks_and_the_file_name() {
    assert_eq!(DEFAULT_FOLDER_NAME, "AlmarioKS");
    let folder = documents().join("AlmarioKS");
    assert_eq!(default_folder(&documents()), folder);

    let location = VaultLocation::proposed(&documents(), "Mi banco: 2026");
    assert_eq!(
        location,
        VaultLocation {
            vault_path: folder.join("Mi banco 2026.kdbx"),
            vault_file: "Mi banco 2026.kdbx".to_owned(),
            key_file_path: folder.join("Mi banco 2026.keyx"),
            key_file: "Mi banco 2026.keyx".to_owned(),
            in_default_folder: true,
        }
    );
    // A name made to climb out of the folder stays inside it.
    let hostile = VaultLocation::proposed(&documents(), "..\\..\\fuera");
    assert_eq!(hostile.vault_path, folder.join("....fuera.kdbx"));
    assert!(is_in_default_folder(&documents(), &hostile.vault_path));
    assert!(is_in_default_folder(&documents(), &location.vault_path));
}

#[test]
fn rf_97_only_a_file_directly_inside_the_folder_is_in_the_default_folder() {
    let documents = documents();
    let folder = default_folder(&documents);
    assert!(is_in_default_folder(&documents, &folder.join("a.kdbx")));

    for outside in [
        documents.join("a.kdbx"),
        documents.join("Otra").join("a.kdbx"),
        folder.join("sub").join("a.kdbx"),
        folder.join("..").join("a.kdbx"),
        folder.join("..").join("Otra").join("a.kdbx"),
        folder.join("sub").join("..").join("a.kdbx"),
        folder.join(".."),
        folder.clone(),
        documents.join("almarioks").join("a.kdbx"),
        documents.join("AlmarioKS 2").join("a.kdbx"),
        PathBuf::from("a.kdbx"),
        PathBuf::new(),
    ] {
        assert!(!is_in_default_folder(&documents, &outside), "{outside:?}");
    }
}

// ---------- RF-99: the key file goes beside the vault ----------

#[test]
fn rf_99_the_key_file_has_the_name_of_the_vault_and_keyx() {
    for (vault, key) in [
        ("Cuentas.kdbx", "Cuentas.keyx"),
        ("Cuentas.KDBX", "Cuentas.keyx"),
        ("v1.2.kdbx", "v1.2.keyx"),
        // A vault the person named without `.kdbx`: nothing of its name is lost.
        ("v1.2", "v1.2.keyx"),
        ("Cuentas", "Cuentas.keyx"),
        ("Cuentas.kdbx.bak", "Cuentas.kdbx.bak.keyx"),
        // Never the vault's own path.
        ("Cuentas.keyx", "Cuentas.keyx.keyx"),
        (".kdbx", ".keyx"),
        ("ñ", "ñ.keyx"),
        ("bóveda.kdbx", "bóveda.keyx"),
    ] {
        assert_eq!(key_file_name(vault), key, "{vault}");
        assert_ne!(key_file_name(vault), vault);
    }
    for (_, name, stem) in cases() {
        assert_eq!(
            key_file_name(&vault_file_name(&name)),
            format!("{stem}.keyx")
        );
    }
}

#[test]
fn rf_99_a_chosen_location_is_kept_as_it_is_with_its_key_file_beside() {
    let elsewhere = Path::new("otra-unidad").join("Bóvedas");
    let chosen = elsewhere.join("Familia.kdbx");

    let location = VaultLocation::chosen(Some(&documents()), &chosen).expect("a file name");
    assert_eq!(
        location,
        VaultLocation {
            vault_path: chosen.clone(),
            vault_file: "Familia.kdbx".to_owned(),
            key_file_path: elsewhere.join("Familia.keyx"),
            key_file: "Familia.keyx".to_owned(),
            in_default_folder: false,
        }
    );
    // Without a Documents folder the answer is the same: it is only needed to say "in the default folder".
    assert_eq!(VaultLocation::chosen(None, &chosen), Some(location));

    // Chosen by hand inside the default folder: it is the default folder all the same.
    let inside = default_folder(&documents()).join("A mano.kdbx");
    let location = VaultLocation::chosen(Some(&documents()), &inside).expect("a file name");
    assert!(location.in_default_folder);
    assert_eq!(
        location.key_file_path,
        default_folder(&documents()).join("A mano.keyx")
    );
    assert!(
        !VaultLocation::chosen(None, &inside)
            .expect("a file name")
            .in_default_folder
    );

    // A path that does not end in a file name is not a place for a vault. `Path` would read the folder as the
    // file in the ones that end in a separator or a dot.
    for nameless in [
        PathBuf::new(),
        PathBuf::from("/"),
        elsewhere.join(".."),
        elsewhere.join("."),
        PathBuf::from("otra-unidad/Bóvedas/"),
        PathBuf::from("otra-unidad/Bóvedas/."),
        PathBuf::from("otra-unidad/Bóvedas/Familia.kdbx/"),
    ] {
        assert_eq!(
            VaultLocation::chosen(Some(&documents()), &nameless),
            None,
            "{nameless:?}"
        );
    }
}
