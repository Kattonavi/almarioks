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
use common::{DOCUMENTS, TempDir, app_with, error_code, invoke_args, invoke_plain, test_parts};
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

// ---------- RF-97 to RF-99: creating at the proposed location ----------

type TestApp = tauri::App<tauri::test::MockRuntime>;

/// Fake data, never a real password.
const PASSWORD: &str = "contraseña-de-prueba";

/// `vault_create_location` for a name, without a chosen path: `(vault_path, key_file_path)`.
fn propose(app: &TestApp, name: &str) -> (String, String) {
    let answer = invoke_args(
        app,
        "vault_create_location",
        json!({ "name": name, "chosen_path": null }),
    )
    .unwrap();
    (
        answer["vault_path"].as_str().unwrap().to_owned(),
        answer["key_file_path"].as_str().unwrap().to_owned(),
    )
}

fn create(app: &TestApp, path: &str, password: Option<&str>, key_file: Value) -> common::Answer {
    let mut args = json!({ "path": path, "name": "Casa", "key_file": key_file });
    if let Some(password) = password {
        args["password"] = json!(password);
    }
    invoke_args(app, "vault_create", args)
}

fn state(app: &TestApp) -> String {
    let status = invoke_plain(app, "vault_status").unwrap();
    status["state"].as_str().unwrap().to_owned()
}

fn none() -> Value {
    json!({ "kind": "none" })
}

#[test]
fn rf_97_vault_create_creates_the_proposed_folder_when_it_is_missing() {
    let dir = TempDir::new("rf97-create-folder");
    let app = app_with(test_parts(&dir));
    let (documents, proposed) = folders(&dir);
    let (vault, _) = propose(&app, "Casa");
    assert!(!proposed.exists());

    let created = create(&app, &vault, Some(PASSWORD), none()).unwrap();
    assert_eq!(created["state"], "open");
    assert_eq!(created["path"], json!(vault));
    assert!(proposed.is_dir());
    assert_eq!(files_in(&documents), ["AlmarioKS"]);
    assert_eq!(files_in(&proposed), ["Casa.kdbx"]);

    // It is a vault that opens again with its key, and the next one goes into the folder that now exists.
    invoke_plain(&app, "vault_close").unwrap();
    let opened = invoke_args(
        &app,
        "vault_open",
        json!({ "path": vault, "password": PASSWORD }),
    )
    .unwrap();
    assert_eq!(opened["state"], "open");
    invoke_plain(&app, "vault_close").unwrap();
    let (second, _) = propose(&app, "Trabajo");
    assert_eq!(
        create(&app, &second, Some(PASSWORD), none()).unwrap()["state"],
        "open"
    );
    assert_eq!(files_in(&proposed), ["Casa.kdbx", "Trabajo.kdbx"]);
}

#[test]
fn rf_97_a_creation_that_is_refused_leaves_no_folder_behind() {
    let dir = TempDir::new("rf97-refused");
    let app = app_with(test_parts(&dir));
    let (documents, proposed) = folders(&dir);
    let (vault, key_file) = propose(&app, "Casa");

    // Refused before anything is written: no key, a key file that is not there.
    assert_eq!(
        error_code(&create(&app, &vault, None, none())),
        "LlaveVacia"
    );
    let missing = json!({ "kind": "existing", "path": key_file });
    assert_eq!(
        error_code(&create(&app, &vault, None, missing)),
        "ArchivoLlaveNoEncontrado"
    );
    assert!(files_in(&documents).is_empty());

    // Refused by the core once the folder was already made (the vault's name): the folder goes away again.
    let unnamed = invoke_args(
        &app,
        "vault_create",
        json!({ "path": vault, "name": "   ", "password": PASSWORD, "key_file": none() }),
    );
    assert_eq!(error_code(&unnamed), "NombreVacio");
    assert!(!proposed.exists());
    assert!(files_in(&documents).is_empty());
    assert_eq!(state(&app), "closed");

    // A folder that was already there, with something of the person's inside, is not touched by a refusal.
    std::fs::create_dir(&proposed).unwrap();
    std::fs::write(proposed.join("notas.txt"), b"de la persona").unwrap();
    let unnamed = invoke_args(
        &app,
        "vault_create",
        json!({ "path": vault, "name": "   ", "password": PASSWORD, "key_file": none() }),
    );
    assert_eq!(error_code(&unnamed), "NombreVacio");
    assert_eq!(files_in(&proposed), ["notas.txt"]);
}

#[test]
fn rf_97_vault_create_creates_no_folder_but_the_proposed_one() {
    let dir = TempDir::new("rf97-no-other-folder");
    let app = app_with(test_parts(&dir));
    let (documents, proposed) = folders(&dir);

    // A missing folder that is not `Documents › AlmarioKS`: as before, the creation fails and nothing appears.
    for missing in [
        dir.path().join("Otra").join("Casa.kdbx"),
        dir.path().join("AlmarioKS").join("Casa.kdbx"),
        documents.join("Otra").join("Casa.kdbx"),
        documents.join("almarioks 2").join("Casa.kdbx"),
        proposed.join("sub").join("Casa.kdbx"),
        proposed.join("..").join("Otra").join("Casa.kdbx"),
        proposed.join("sub").join("..").join("Casa.kdbx"),
    ] {
        let answer = create(&app, text(&missing), Some(PASSWORD), none());
        assert_eq!(error_code(&answer), "GuardadoFallido", "{missing:?}");
        assert!(files_in(&documents).is_empty(), "{missing:?}");
        assert_eq!(state(&app), "closed", "{missing:?}");
    }
    assert_eq!(files_in(dir.path()), [DOCUMENTS]);

    // A path that only passes through the proposed folder does not create it either, whatever the system makes
    // of the `..` (Windows resolves it before looking at the disk; other systems do not).
    let through = proposed.join("..").join("Casa.kdbx");
    drop(create(&app, text(&through), Some(PASSWORD), none()));
    assert!(!proposed.exists());
}

#[test]
fn rf_97_without_a_documents_folder_vault_create_creates_no_folder() {
    let dir = TempDir::new("rf97-create-no-documents");
    let no_folder: Arc<DocumentsFolder> = Arc::new(|| None);
    let app = app_with(AppParts {
        documents_dir: no_folder,
        ..test_parts(&dir)
    });
    let (documents, proposed) = folders(&dir);

    let answer = create(
        &app,
        text(&proposed.join("Casa.kdbx")),
        Some(PASSWORD),
        none(),
    );
    assert_eq!(error_code(&answer), "GuardadoFallido");
    assert!(files_in(&documents).is_empty());

    // In a folder that exists it creates the vault, as it always did.
    let created = create(
        &app,
        text(&documents.join("Casa.kdbx")),
        Some(PASSWORD),
        none(),
    );
    assert_eq!(created.unwrap()["state"], "open");
}

#[test]
fn rf_98_a_file_at_the_proposed_location_is_never_replaced() {
    let dir = TempDir::new("rf98-exists");
    let app = app_with(test_parts(&dir));
    let (_, proposed) = folders(&dir);
    let (vault, key_file) = propose(&app, "Casa");
    std::fs::create_dir(&proposed).unwrap();
    std::fs::write(&vault, b"la boveda de antes").unwrap();

    // The location warns before "Crear" is pressed…
    let location = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "Casa", "chosen_path": null }),
    )
    .unwrap();
    assert_eq!(location["vault_exists"], true);

    // …and creating answers `ArchivoExiste`, with a password, with a new key file or with both: no dialog asked
    // anything here, the refusal is the app's own.
    for (password, key) in [
        (Some(PASSWORD), none()),
        (None, json!({ "kind": "generate", "path": key_file })),
        (
            Some(PASSWORD),
            json!({ "kind": "generate", "path": key_file }),
        ),
    ] {
        let answer = create(&app, &vault, password, key);
        assert_eq!(error_code(&answer), "ArchivoExiste");
        assert_eq!(std::fs::read(&vault).unwrap(), b"la boveda de antes");
        assert_eq!(files_in(&proposed), ["Casa.kdbx"]);
        assert_eq!(state(&app), "closed");
    }

    // A folder with that name is in the way just the same.
    let (other, _) = propose(&app, "Carpeta");
    std::fs::create_dir(&other).unwrap();
    let answer = create(&app, &other, Some(PASSWORD), none());
    assert_eq!(error_code(&answer), "ArchivoExiste");
    assert!(Path::new(&other).is_dir());
    assert_eq!(
        invoke_plain(&app, "recent_list").unwrap(),
        json!([]),
        "a vault that was not created is not remembered"
    );
}

#[test]
fn rf_99_a_new_key_file_beside_the_vault_never_replaces_an_existing_one() {
    let dir = TempDir::new("rf99-key-exists");
    let app = app_with(test_parts(&dir));
    let (_, proposed) = folders(&dir);
    let (vault, key_file) = propose(&app, "Casa");
    std::fs::create_dir(&proposed).unwrap();
    std::fs::write(&key_file, b"la llave de otra boveda").unwrap();

    let location = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "Casa", "chosen_path": null }),
    )
    .unwrap();
    assert_eq!(
        (&location["vault_exists"], &location["key_file_exists"]),
        (&json!(false), &json!(true))
    );

    for password in [None, Some(PASSWORD)] {
        let generate = json!({ "kind": "generate", "path": key_file });
        let answer = create(&app, &vault, password, generate);
        assert_eq!(error_code(&answer), "ArchivoExiste");
        // The key file is as it was, and no vault was left without the key it was asked with.
        assert_eq!(
            std::fs::read(&key_file).unwrap(),
            b"la llave de otra boveda"
        );
        assert_eq!(files_in(&proposed), ["Casa.keyx"]);
        assert_eq!(state(&app), "closed");
    }
}

#[test]
fn rf_99_the_new_key_file_is_created_beside_the_vault_in_the_proposed_folder() {
    let dir = TempDir::new("rf99-key-beside");
    let app = app_with(test_parts(&dir));
    let (_, proposed) = folders(&dir);
    let (vault, key_file) = propose(&app, "Mi banco: 2026");
    assert!(!proposed.exists());

    let generate = json!({ "kind": "generate", "path": key_file });
    let created = create(&app, &vault, None, generate).unwrap();
    assert_eq!(created["state"], "open");
    assert_eq!(
        files_in(&proposed),
        ["Mi banco 2026.kdbx", "Mi banco 2026.keyx"]
    );

    // The pair opens again, and the location now says both are taken.
    invoke_plain(&app, "vault_close").unwrap();
    let opened = invoke_args(
        &app,
        "vault_open",
        json!({ "path": vault, "key_file_path": key_file }),
    )
    .unwrap();
    assert_eq!(opened["state"], "open");
    let location = invoke_args(
        &app,
        "vault_create_location",
        json!({ "name": "Mi banco: 2026", "chosen_path": null }),
    )
    .unwrap();
    assert_eq!(
        (&location["vault_exists"], &location["key_file_exists"]),
        (&json!(true), &json!(true))
    );
}
