//! Where a new vault goes (RF-97 to RF-99), through the real IPC and the real allow-list: the location proposed
//! from the vault's name, the key file beside it, and what `vault_create` does there.
//!
//! "Documents" is a folder inside the test's own temporary folder (`common::documents_in`): no test reads or
//! writes the real Documents of the machine.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use almarioks_lib::{
    commands::INTERNAL_CODE,
    state::{AppParts, DocumentsFolder},
};
use common::{DOCUMENTS, TempDir, app_with, error_code, invoke_args, test_parts};
use serde_json::{Value, json};

fn text(path: &Path) -> &str {
    path.to_str().expect("the test paths are valid text")
}

/// The test's Documents folder and the folder the app proposes inside it.
fn folders(dir: &TempDir) -> (PathBuf, PathBuf) {
    let documents = dir.path().join(DOCUMENTS);
    let proposed = documents.join("AlmarioKS");
    (documents, proposed)
}

fn files_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

// ---------- RF-97: the proposed location ----------

#[test]
fn rf_97_the_proposed_location_is_documents_almarioks_and_the_name_and_nothing_is_written() {
    let dir = TempDir::new("rf97-proposed");
    let app = app_with(test_parts(&dir));
    let (documents, proposed) = folders(&dir);

    let answer = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "Mi banco: 2026", "chosen_path": null }),
    );
    assert_eq!(
        answer,
        Ok(json!({
            "vault_path": text(&proposed.join("Mi banco 2026.kdbx")),
            "vault_file": "Mi banco 2026.kdbx",
            "key_file_path": text(&proposed.join("Mi banco 2026.keyx")),
            "key_file": "Mi banco 2026.keyx",
            "in_default_folder": true,
            "vault_exists": false,
            "key_file_exists": false,
        }))
    );

    // The name is not validated here: one that leaves no file name is `bóveda`, and one made to leave the folder
    // stays inside it.
    for (name, file) in [
        ("", "bóveda.kdbx"),
        ("  ???  ", "bóveda.kdbx"),
        ("..\\..\\fuera", "....fuera.kdbx"),
        ("nul", "nul_.kdbx"),
        ("Cuentas.", "Cuentas.kdbx"),
    ] {
        let answer = invoke_args(
            &app,
            "vault_create_location",
            json!({ "name": name, "chosen_path": null }),
        )
        .unwrap();
        assert_eq!(answer["vault_file"], file, "{name:?}");
        assert_eq!(answer["vault_path"], text(&proposed.join(file)), "{name:?}");
        assert_eq!(answer["in_default_folder"], true, "{name:?}");
    }

    // Read-only: not even the folder was created.
    assert!(!proposed.exists());
    assert!(files_in(&documents).is_empty());
}

#[test]
fn rf_97_the_location_says_whether_the_vault_or_the_key_file_already_exist() {
    let dir = TempDir::new("rf97-exists");
    let app = app_with(test_parts(&dir));
    let (_, proposed) = folders(&dir);
    std::fs::create_dir(&proposed).unwrap();
    let ask = |name: &str| {
        let answer = invoke_args(
            &app,
            "vault_create_location",
            json!({ "name": name, "chosen_path": null }),
        )
        .unwrap();
        (
            answer["vault_exists"].as_bool().unwrap(),
            answer["key_file_exists"].as_bool().unwrap(),
        )
    };

    assert_eq!(ask("Casa"), (false, false));
    std::fs::write(proposed.join("Casa.kdbx"), b"una boveda").unwrap();
    assert_eq!(ask("Casa"), (true, false));
    std::fs::write(proposed.join("Casa.keyx"), b"una llave").unwrap();
    assert_eq!(ask("Casa"), (true, true));
    assert_eq!(ask("Otra"), (false, false));
    // Anything at the path counts, as it does for `vault_create`: a folder too.
    std::fs::create_dir(proposed.join("Carpeta.keyx")).unwrap();
    assert_eq!(ask("Carpeta"), (false, true));

    // Asking changed nothing.
    assert_eq!(
        std::fs::read(proposed.join("Casa.kdbx")).unwrap(),
        b"una boveda"
    );
    assert_eq!(
        files_in(&proposed),
        ["Carpeta.keyx", "Casa.kdbx", "Casa.keyx"]
    );
}

#[test]
fn rf_97_a_chosen_location_is_answered_as_it_is_with_the_key_file_beside_it() {
    let dir = TempDir::new("rf97-chosen");
    let app = app_with(test_parts(&dir));
    let (_, proposed) = folders(&dir);
    let elsewhere = dir.path().join("Otra carpeta");
    let chosen = elsewhere.join("Familia.kdbx");

    // The folder does not even exist: the command only answers paths.
    let answer = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "No se usa para la ruta", "chosen_path": text(&chosen) }),
    );
    assert_eq!(
        answer,
        Ok(json!({
            "vault_path": text(&chosen),
            "vault_file": "Familia.kdbx",
            "key_file_path": text(&elsewhere.join("Familia.keyx")),
            "key_file": "Familia.keyx",
            "in_default_folder": false,
            "vault_exists": false,
            "key_file_exists": false,
        }))
    );
    assert!(!elsewhere.exists());

    // Chosen by hand inside the proposed folder, and a name without `.kdbx`: nothing of the name is lost.
    let by_hand = proposed.join("v1.2");
    let answer = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "x", "chosen_path": text(&by_hand) }),
    )
    .unwrap();
    assert_eq!(answer["vault_path"], text(&by_hand));
    assert_eq!(answer["key_file"], "v1.2.keyx");
    assert_eq!(answer["in_default_folder"], true);
    assert!(!proposed.exists());
}

#[test]
fn rf_97_without_a_documents_folder_nothing_is_proposed_and_a_chosen_location_still_works() {
    let dir = TempDir::new("rf97-no-documents");
    let chosen = dir.path().join("Familia.kdbx");
    let no_folder: Arc<DocumentsFolder> = Arc::new(|| None);
    // A folder that is not a full path is no folder to propose either.
    let relative: Arc<DocumentsFolder> = Arc::new(|| Some(PathBuf::from("Documentos")));

    for (case, documents_dir) in [("none", no_folder), ("relative", relative)] {
        let app = app_with(AppParts {
            documents_dir,
            ..test_parts(&dir)
        });
        let proposal = invoke_args(
            &app,
            "vault_create_location",
            json!({ "name": "Casa", "chosen_path": null }),
        );
        assert_eq!(error_code(&proposal), INTERNAL_CODE, "{case}");

        let answer = invoke_args(
            &app,
            "vault_create_location",
            json!({ "name": "Casa", "chosen_path": text(&chosen) }),
        )
        .unwrap();
        assert_eq!(answer["vault_path"], text(&chosen), "{case}");
        assert_eq!(
            answer["key_file_path"],
            text(&dir.path().join("Familia.keyx"))
        );
        assert_eq!(answer["in_default_folder"], false, "{case}");
    }
    assert!(!Path::new("Documentos").exists());
}

// ---------- RF-80: strict arguments ----------

#[test]
fn rf_80_vault_create_location_takes_exactly_a_name_and_a_chosen_path_that_may_be_null() {
    let dir = TempDir::new("rf80-location");
    let app = app_with(test_parts(&dir));
    let (documents, _) = folders(&dir);
    let chosen = text(&dir.path().join("a.kdbx")).to_owned();

    let malformed = [
        // `chosen_path` is always sent: a missing key is not "nothing chosen".
        json!({ "name": "Casa" }),
        json!({ "chosen_path": null }),
        json!({}),
        // Unknown field, camelCase.
        json!({ "name": "Casa", "chosen_path": null, "create": true }),
        json!({ "name": "Casa", "chosenPath": null }),
        json!({ "name": "Casa", "chosen_path": null, "chosenPath": chosen }),
        // Wrong type.
        json!({ "name": null, "chosen_path": null }),
        json!({ "name": 7, "chosen_path": null }),
        json!({ "name": ["Casa"], "chosen_path": null }),
        json!({ "name": "Casa", "chosen_path": 7 }),
        json!({ "name": "Casa", "chosen_path": false }),
        json!({ "name": "Casa", "chosen_path": [chosen] }),
        json!({ "name": "Casa", "chosen_path": { "path": chosen } }),
        // An empty path, and paths without a file name.
        json!({ "name": "Casa", "chosen_path": "" }),
        json!({ "name": "Casa", "chosen_path": text(&dir.path().join("..")) }),
        json!({ "name": "Casa", "chosen_path": "/" }),
        json!({ "name": "Casa", "chosen_path": format!("{}/", text(dir.path())) }),
        json!({ "name": "Casa", "chosen_path": format!("{chosen}/") }),
        json!({ "name": "Casa", "chosen_path": text(&dir.path().join(".")) }),
    ];
    for args in malformed {
        let answer = invoke_args(&app, "vault_create_location", args.clone());
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "{args}");
    }
    let positional = common::invoke(
        &app,
        "vault_create_location",
        json!({ "args": ["Casa", Value::Null] }),
    );
    assert_eq!(error_code(&positional), "ArgumentoInvalido");

    // The two well-formed shapes.
    for chosen_path in [Value::Null, json!(chosen)] {
        let answer = invoke_args(
            &app,
            "vault_create_location",
            json!({ "name": "Casa", "chosen_path": chosen_path }),
        );
        assert!(answer.is_ok(), "{answer:?}");
    }
    assert!(files_in(&documents).is_empty());
}
