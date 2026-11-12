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

use almarioks_core::settings::Settings;
use almarioks_lib::{
    commands::INTERNAL_CODE,
    state::{AppParts, DocumentsFolder},
};
use common::{
    DOCUMENTS, FakeDialogs, TempDir, app_with, error_code, invoke_args, invoke_plain, parts_with,
    test_parts,
};
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

// ---------- RF-110: the key file beside the vault, proposed when unlocking ----------

/// What the files that play a key file hold. `unlock_key_suggestion` never reads them, so it is not a key, and
/// no answer may contain it.
const KEY_CONTENT: &str = "CANARIO-ALMARIOKS-rf110-contenido-de-la-llave";

fn write_key(path: &Path) {
    std::fs::write(path, KEY_CONTENT).unwrap();
}

/// Writes a settings file that remembers `vaults` as recent, the first one the most recent.
fn remember(dir: &TempDir, vaults: &[&Path]) {
    let mut settings = Settings::default();
    for vault in vaults.iter().rev() {
        settings.remember_vault(vault);
    }
    settings
        .save(&dir.settings_file())
        .expect("write the settings file");
}

fn suggestion(app: &TestApp, vault: &Path) -> Value {
    let answer = invoke_args(app, "unlock_key_suggestion", json!({ "path": text(vault) }))
        .expect("unlock_key_suggestion answers, also when there is nothing to propose");
    assert!(
        !answer.to_string().contains(KEY_CONTENT),
        "the content of a key file travelled"
    );
    answer
}

#[test]
fn rf_110_a_recent_vault_gets_the_key_file_beside_it_and_null_when_there_is_none() {
    let dir = TempDir::new("rf110-recent");
    let vault = dir.path().join("Familia.kdbx");
    let key = dir.path().join("Familia.keyx");
    let upper = dir.path().join("Trabajo.KDBX");
    let odd = dir.path().join("sin-extension");
    remember(&dir, &[&vault, &upper, &odd]);
    let app = app_with(test_parts(&dir));

    // Nothing beside it yet.
    assert_eq!(suggestion(&app, &vault), Value::Null);

    write_key(&key);
    assert_eq!(
        suggestion(&app, &vault),
        json!({ "key_file_path": text(&key), "key_file": "Familia.keyx" })
    );

    // The name of the key file follows the rule of the core (`location::key_file_name`): `.keyx` in the place
    // of a final `.kdbx` in any case, or added to a name that does not end in it.
    write_key(&dir.path().join("Trabajo.keyx"));
    assert_eq!(
        suggestion(&app, &upper),
        json!({
            "key_file_path": text(&dir.path().join("Trabajo.keyx")),
            "key_file": "Trabajo.keyx",
        })
    );
    write_key(&dir.path().join("sin-extension.keyx"));
    assert_eq!(suggestion(&app, &odd)["key_file"], "sin-extension.keyx");

    // Gone again: nothing to propose. A folder with that name is not a key file either.
    std::fs::remove_file(&key).unwrap();
    assert_eq!(suggestion(&app, &vault), Value::Null);
    std::fs::create_dir(&key).unwrap();
    assert_eq!(suggestion(&app, &vault), Value::Null);
}

#[test]
fn rf_110_the_vault_chosen_in_the_open_dialog_gets_it_and_the_other_dialogs_do_not_count() {
    let dir = TempDir::new("rf110-picked");
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    let first = dir.path().join("uno.kdbx");
    let second = dir.path().join("dos.kdbx");
    write_key(&dir.path().join("uno.keyx"));
    write_key(&dir.path().join("dos.keyx"));

    // Nobody chose it yet: the file beside it is there, and the answer is still `null`.
    assert_eq!(suggestion(&app, &first), Value::Null);

    // The other three dialogs answer the same path, and none of them is "the vault to open".
    dialogs.will_choose(&first);
    for command in ["pick_vault_to_create", "pick_key_file", "pick_new_key_file"] {
        assert_eq!(invoke_plain(&app, command).unwrap(), json!(text(&first)));
        assert_eq!(suggestion(&app, &first), Value::Null, "after `{command}`");
    }

    assert_eq!(
        invoke_plain(&app, "pick_vault_to_open").unwrap(),
        json!(text(&first))
    );
    assert_eq!(
        suggestion(&app, &first),
        json!({
            "key_file_path": text(&dir.path().join("uno.keyx")),
            "key_file": "uno.keyx",
        })
    );

    // A dialog closed without choosing answers no path: the last one chosen stays.
    dialogs.will_cancel();
    assert_eq!(
        invoke_plain(&app, "pick_vault_to_open").unwrap(),
        Value::Null
    );
    assert_eq!(suggestion(&app, &first)["key_file"], "uno.keyx");

    // Only the last path the dialog answered counts.
    dialogs.will_choose(&second);
    invoke_plain(&app, "pick_vault_to_open").unwrap();
    assert_eq!(suggestion(&app, &second)["key_file"], "dos.keyx");
    assert_eq!(suggestion(&app, &first), Value::Null);

    // It lives in memory only: nothing was written down.
    assert!(!dir.settings_file().exists());
}

#[test]
fn rf_110_the_locked_vault_of_the_session_gets_it_and_a_closed_one_does_not() {
    let dir = TempDir::new("rf110-locked");
    let app = app_with(test_parts(&dir));
    let vault = dir.path().join("Familia.kdbx");
    let key = dir.path().join("Familia.keyx");
    let forget = || {
        let left = invoke_args(&app, "recent_remove", json!({ "path": text(&vault) })).unwrap();
        assert_eq!(left, json!([]));
    };

    // RF-99: a vault with a new key file beside it. Out of the recent list, so only the session knows it.
    let generate = json!({ "kind": "generate", "path": text(&key) });
    create(&app, text(&vault), Some(PASSWORD), generate).unwrap();
    forget();
    // Open is not "about to unlock": without another source there is no answer.
    assert_eq!(suggestion(&app, &vault), Value::Null);

    invoke_plain(&app, "vault_lock").unwrap();
    assert_eq!(state(&app), "locked");
    let found = suggestion(&app, &vault);
    assert_eq!(
        found,
        json!({ "key_file_path": text(&key), "key_file": "Familia.keyx" })
    );
    // Locked on one vault says nothing about another one.
    let other = dir.path().join("otra.kdbx");
    write_key(&dir.path().join("otra.keyx"));
    assert_eq!(suggestion(&app, &other), Value::Null);

    // The "hecho cuando" of the owner: with what was proposed, only the password is typed.
    let opened = invoke_args(
        &app,
        "vault_open",
        json!({
            "path": text(&vault), "password": PASSWORD, "key_file_path": found["key_file_path"],
        }),
    )
    .unwrap();
    assert_eq!(opened["state"], "open");

    // Closed, and not a recent vault: the session has forgotten which file it was (RF-36).
    forget();
    invoke_plain(&app, "vault_close").unwrap();
    assert_eq!(state(&app), "closed");
    assert_eq!(suggestion(&app, &vault), Value::Null);
}

#[test]
fn rf_110_a_path_from_nowhere_answers_null_even_when_its_key_file_is_there() {
    let dir = TempDir::new("rf110-unknown");
    let known = dir.path().join("conocida.kdbx");
    remember(&dir, &[&known]);
    let app = app_with(test_parts(&dir));
    let unknown = dir.path().join("ajena.kdbx");
    std::fs::write(&unknown, b"no es una boveda").unwrap();
    write_key(&dir.path().join("ajena.keyx"));
    write_key(&dir.path().join("conocida.keyx"));
    std::fs::create_dir(dir.path().join("sub")).unwrap();

    // The recent one is answered, so the files are where the command would look.
    assert_eq!(suggestion(&app, &known)["key_file"], "conocida.keyx");

    // Not recent, not chosen, not locked: `null`, the same answer as "there is no key file". Not an error.
    assert_eq!(suggestion(&app, &unknown), Value::Null);

    // Trying to open it and failing does not make it known: the session stays closed.
    let failed = invoke_args(
        &app,
        "vault_open",
        json!({ "path": text(&unknown), "password": PASSWORD }),
    );
    assert!(failed.is_err());
    assert_eq!(state(&app), "closed");
    assert_eq!(suggestion(&app, &unknown), Value::Null);

    // Paths are compared as they were stored, like the recent list does: another way of writing the recent
    // vault, its folder, its key file or a bare name are all "another path".
    let folder = text(dir.path());
    let others = [
        format!("{folder}\\sub\\..\\conocida.kdbx"),
        format!("{folder}\\CONOCIDA.KDBX"),
        format!("{folder}\\conocida.keyx"),
        format!("{folder}\\"),
        folder.to_owned(),
        "conocida.kdbx".to_owned(),
        ".".to_owned(),
        "..".to_owned(),
    ];
    for path in others {
        let answer = invoke_args(&app, "unlock_key_suggestion", json!({ "path": path }));
        assert_eq!(answer, Ok(Value::Null), "{path}");
    }
}

#[cfg(windows)]
#[test]
fn rf_110_the_key_file_is_never_opened_and_nothing_is_written() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = TempDir::new("rf110-no-read-no-write");
    let folder = dir.path().join("bovedas");
    std::fs::create_dir(&folder).unwrap();
    let vault = folder.join("Familia.kdbx");
    let key = folder.join("Familia.keyx");
    std::fs::write(&vault, b"no se abre en esta prueba").unwrap();
    write_key(&key);
    remember(&dir, &[&vault]);
    let settings_before = std::fs::read(dir.settings_file()).unwrap();
    let app = app_with(test_parts(&dir));

    // Another program holds the key file and shares nothing: whoever opens it, to read or to write, fails.
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&key)
        .unwrap();
    assert!(
        std::fs::read(&key).is_err(),
        "the lock of the test does not hold: reading the key file still works"
    );

    assert_eq!(
        suggestion(&app, &vault),
        json!({ "key_file_path": text(&key), "key_file": "Familia.keyx" })
    );
    drop(held);

    // Nothing new, nothing changed: not beside the vault, not in the settings, not in the session.
    assert_eq!(files_in(&folder), ["Familia.kdbx", "Familia.keyx"]);
    assert_eq!(std::fs::read(&key).unwrap(), KEY_CONTENT.as_bytes());
    assert_eq!(std::fs::read(&vault).unwrap(), b"no se abre en esta prueba");
    assert_eq!(
        files_in(dir.path()),
        [DOCUMENTS, "bovedas", "settings.json"]
    );
    assert_eq!(std::fs::read(dir.settings_file()).unwrap(), settings_before);
    assert_eq!(
        invoke_plain(&app, "recent_list").unwrap(),
        json!([{ "path": text(&vault), "exists": true }])
    );
    assert_eq!(state(&app), "closed");
}

#[test]
fn rf_80_unlock_key_suggestion_takes_exactly_one_path() {
    let dir = TempDir::new("rf80-key-suggestion");
    let vault = dir.path().join("Familia.kdbx");
    write_key(&dir.path().join("Familia.keyx"));
    remember(&dir, &[&vault]);
    let app = app_with(test_parts(&dir));
    let path = text(&vault);

    let malformed = [
        json!({}),
        json!({ "path": null }),
        json!({ "path": 7 }),
        json!({ "path": [path] }),
        json!({ "path": "" }),
        json!({ "path": path, "key_file_path": path }),
        json!({ "Path": path }),
        json!({ "vault_path": path }),
        json!([path]),
    ];
    for args in malformed {
        let answer = invoke_args(&app, "unlock_key_suggestion", args.clone());
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "with {args}");
    }
    assert_eq!(
        error_code(&invoke_plain(&app, "unlock_key_suggestion")),
        "ArgumentoInvalido"
    );
    assert_eq!(suggestion(&app, &vault)["key_file"], "Familia.keyx");
}

// ---------- RF-112: a failed unlock does not say which part of the key failed ----------

#[test]
fn rf_112_a_missing_key_file_and_a_wrong_password_fail_with_the_same_answer() {
    let dir = TempDir::new("rf112-same-answer");
    let app = app_with(test_parts(&dir));
    let vault = dir.path().join("Familia.kdbx");
    let key = dir.path().join("Familia.keyx");
    let generate = json!({ "kind": "generate", "path": text(&key) });
    create(&app, text(&vault), Some(PASSWORD), generate).unwrap();
    invoke_plain(&app, "vault_close").unwrap();

    // This vault uses a key file. Opening it with the password alone, with the key file alone or with a wrong
    // password gives one and the same error: nothing in it says whether the vault uses a key file.
    let attempts = [
        json!({ "path": text(&vault), "password": PASSWORD }),
        json!({ "path": text(&vault), "key_file_path": text(&key) }),
        json!({ "path": text(&vault), "password": "otra", "key_file_path": text(&key) }),
    ];
    let answers: Vec<common::Answer> = attempts
        .into_iter()
        .map(|args| invoke_args(&app, "vault_open", args))
        .collect();
    for answer in &answers {
        assert_eq!(error_code(answer), "LlaveIncorrecta");
        assert_eq!(answer, &answers[0]);
    }
    assert_eq!(state(&app), "closed");
}
