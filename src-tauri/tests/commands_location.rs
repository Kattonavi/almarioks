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

// ---------- RF-111: the key file a recent vault was last opened with ----------
// C-1 to C-10 are the conditions of the security review of 0.1 (second round of the owner). Each test names the
// one it proves.

/// A vault that opens with [`PASSWORD`] and a key file that is NOT beside it, left closed. The key file holds
/// [`KEY_CONTENT`] (any file is a key file, RF-87), so it doubles as a canary: it must never show up in the
/// settings file nor in an answer.
struct Keyed {
    app: TestApp,
    dialogs: FakeDialogs,
    vault: PathBuf,
    key: PathBuf,
    dir: TempDir,
}

fn keyed(tag: &str) -> Keyed {
    let dir = TempDir::new(tag);
    let dialogs = FakeDialogs::default();
    let app = app_with(parts_with(&dir, &dialogs));
    let vault = dir.path().join("Familia.kdbx");
    std::fs::create_dir(dir.path().join("llaves")).unwrap();
    let key = dir.path().join("llaves").join("mi-llave.key");
    write_key(&key);
    let existing = json!({ "kind": "existing", "path": text(&key) });
    create(&app, text(&vault), Some(PASSWORD), existing).unwrap();
    close(&app);
    // C-1: creating a vault with a key file remembers the vault and no key file.
    assert_eq!(remembered_in(&dir), None);
    Keyed {
        app,
        dialogs,
        vault,
        key,
        dir,
    }
}

impl Keyed {
    /// The person chooses `key` in the "choose the key file" dialog.
    fn pick_key(&self, key: &Path) {
        self.dialogs.will_choose(key);
        let answer = invoke_plain(&self.app, "pick_key_file").unwrap();
        assert_eq!(answer, json!(text(key)));
    }

    fn open(&self, password: Option<&str>, key: Option<&Path>) -> common::Answer {
        open(&self.app, &self.vault, password, key)
    }

    /// Chooses the key file in the dialog, opens the vault with it and the password, and closes it: from here
    /// on the key file is the one remembered for the vault.
    fn open_once_with_the_key_chosen(&self) {
        self.pick_key(&self.key);
        let opened = self.open(Some(PASSWORD), Some(&self.key)).unwrap();
        assert_eq!(opened["state"], "open");
        close(&self.app);
        assert_eq!(
            remembered_in(&self.dir),
            Some(json!({ text(&self.vault): text(&self.key) }))
        );
    }
}

fn open(app: &TestApp, vault: &Path, password: Option<&str>, key: Option<&Path>) -> common::Answer {
    let mut args = json!({ "path": text(vault) });
    if let Some(password) = password {
        args["password"] = json!(password);
    }
    if let Some(key) = key {
        args["key_file_path"] = json!(text(key));
    }
    invoke_args(app, "vault_open", args)
}

fn close(app: &TestApp) {
    invoke_plain(app, "vault_close").unwrap();
    assert_eq!(state(app), "closed");
}

/// The settings file as it is on disk, and never with the content of a key file in it (C-3).
fn settings_file(dir: &TempDir) -> Vec<u8> {
    let bytes = std::fs::read(dir.settings_file()).unwrap();
    assert!(
        !String::from_utf8_lossy(&bytes).contains(KEY_CONTENT),
        "the content of a key file reached the settings file"
    );
    bytes
}

/// `key_files` of the settings file on disk, or `None` if the file does not have that key.
fn remembered_in(dir: &TempDir) -> Option<Value> {
    let file: Value = serde_json::from_slice(&settings_file(dir)).unwrap();
    file.get("key_files").cloned()
}

/// The answer that proposes the remembered key file.
fn remembered(key: &Path) -> Value {
    json!({
        "key_file_path": text(key),
        "key_file": key.file_name().unwrap().to_str().unwrap(),
        "source": "remembered",
    })
}

#[test]
fn rf_111_the_key_file_chosen_once_comes_already_chosen_the_next_time() {
    let rig = keyed("rf111-remembered");
    let beside = rig.dir.path().join("Familia.keyx");

    // A recent vault with nothing beside it and nothing remembered yet.
    assert_eq!(suggestion(&rig.app, &rig.vault), Value::Null);

    // The "hecho cuando" of the owner: choose it once, close, and the next time it comes already chosen.
    rig.open_once_with_the_key_chosen();
    let found = suggestion(&rig.app, &rig.vault);
    assert_eq!(found, remembered(&rig.key));
    let opened = invoke_args(
        &rig.app,
        "vault_open",
        json!({
            "path": text(&rig.vault), "password": PASSWORD, "key_file_path": found["key_file_path"],
        }),
    )
    .unwrap();
    assert_eq!(opened["state"], "open");

    // Locked instead of closed: the same answer.
    invoke_plain(&rig.app, "vault_lock").unwrap();
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    close(&rig.app);

    // The remembered one wins over the one beside the vault.
    write_key(&beside);
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));

    // It is in the settings file, so the app started again still knows it; and only the vault it belongs to.
    let again = app_with(test_parts(&rig.dir));
    assert_eq!(suggestion(&again, &rig.vault), remembered(&rig.key));
    assert_eq!(suggestion(&again, &rig.key), Value::Null);
}

#[test]
fn rf_111_c1_a_failed_unlock_and_the_other_commands_leave_what_is_remembered_as_it_was() {
    let rig = keyed("rf111-c1");
    rig.open_once_with_the_key_chosen();
    let before = settings_file(&rig.dir);

    // Three key files the person chooses in the dialog, so that only the failure keeps them from being
    // remembered: one that is not this vault's, one that is not there, and one that cannot be read.
    let wrong = rig.dir.path().join("llaves").join("ajena.key");
    std::fs::write(&wrong, b"otra llave").unwrap();
    let gone = rig.dir.path().join("llaves").join("no-existe.key");
    let unreadable = rig.dir.path().join("llaves").join("carpeta.key");
    std::fs::create_dir(&unreadable).unwrap();
    for (key, code) in [
        (&wrong, "LlaveIncorrecta"),
        (&gone, "ArchivoLlaveNoEncontrado"),
        (&unreadable, "ArchivoLlaveDanado"),
    ] {
        rig.pick_key(key);
        let answer = rig.open(Some(PASSWORD), Some(key));
        assert_eq!(error_code(&answer), code);
        assert_eq!(settings_file(&rig.dir), before, "after {code}");
    }
    // A wrong password with the right key file, chosen again.
    rig.pick_key(&rig.key);
    let answer = rig.open(Some("otra"), Some(&rig.key));
    assert_eq!(error_code(&answer), "LlaveIncorrecta");
    assert_eq!(settings_file(&rig.dir), before);

    // Asking for the suggestion writes nothing either.
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    assert_eq!(settings_file(&rig.dir), before);

    // Saving the open vault somewhere else remembers that vault, and no key file for it; the one remembered
    // for the first vault stays.
    rig.open(Some(PASSWORD), Some(&rig.key)).unwrap();
    assert_eq!(settings_file(&rig.dir), before);
    let copy = rig.dir.path().join("Copia.kdbx");
    invoke_args(&rig.app, "vault_save_as", json!({ "path": text(&copy) })).unwrap();
    assert_eq!(
        invoke_plain(&rig.app, "recent_list").unwrap(),
        json!([
            { "path": text(&copy), "exists": true },
            { "path": text(&rig.vault), "exists": true },
        ])
    );
    assert_eq!(
        remembered_in(&rig.dir),
        Some(json!({ text(&rig.vault): text(&rig.key) }))
    );
}

#[cfg(windows)]
#[test]
fn rf_111_c2_a_key_file_that_may_not_be_remembered_opens_the_vault_and_nothing_stays_remembered() {
    let rig = keyed("rf111-c2");
    rig.open_once_with_the_key_chosen();

    // The same file by a path that is not `X:\…`. The person chose it in the dialog and the vault opens with it.
    let verbatim = PathBuf::from(format!(r"\\?\{}", text(&rig.key)));
    assert!(
        verbatim.is_file(),
        "the test needs its folder on a local drive"
    );
    rig.pick_key(&verbatim);
    let opened = rig.open(Some(PASSWORD), Some(&verbatim)).unwrap();
    assert_eq!(opened["state"], "open");
    close(&rig.app);

    // Not remembered, and the one that was remembered is not the last one it opened with any more.
    assert_eq!(remembered_in(&rig.dir), None);
    assert_eq!(suggestion(&rig.app, &rig.vault), Value::Null);
}

#[test]
fn rf_111_c3_only_a_key_file_chosen_in_its_dialog_is_remembered_and_only_its_path() {
    let rig = keyed("rf111-c3");

    // The window sends a path nobody chose in the dialog: the vault opens and nothing is remembered.
    let opened = rig.open(Some(PASSWORD), Some(&rig.key)).unwrap();
    assert_eq!(opened["state"], "open");
    close(&rig.app);
    assert_eq!(remembered_in(&rig.dir), None);
    assert_eq!(suggestion(&rig.app, &rig.vault), Value::Null);

    // The other three dialogs answer the same path, and none of them is "the key file to open with".
    rig.dialogs.will_choose(&rig.key);
    for command in [
        "pick_vault_to_open",
        "pick_vault_to_create",
        "pick_new_key_file",
    ] {
        invoke_plain(&rig.app, command).unwrap();
        rig.open(Some(PASSWORD), Some(&rig.key)).unwrap();
        close(&rig.app);
        assert_eq!(remembered_in(&rig.dir), None, "after `{command}`");
    }

    // Chosen in its dialog: remembered. The file holds its four keys and, of the key file, its path.
    rig.open_once_with_the_key_chosen();
    let file: Value = serde_json::from_slice(&settings_file(&rig.dir)).unwrap();
    let keys: Vec<&String> = file.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "auto_lock_minutes",
            "clipboard_seconds",
            "key_files",
            "recent_vaults"
        ]
    );
    assert_eq!(
        file["key_files"],
        json!({ text(&rig.vault): text(&rig.key) })
    );

    // The one already remembered needs no dialog the next time: the suggestion gave it to the window.
    rig.dialogs
        .will_choose(rig.dir.path().join("otra-cosa.key"));
    invoke_plain(&rig.app, "pick_key_file").unwrap();
    let before = settings_file(&rig.dir);
    rig.open(Some(PASSWORD), Some(&rig.key)).unwrap();
    close(&rig.app);
    assert_eq!(settings_file(&rig.dir), before);

    // Another path to the same key file, sent without a dialog: it opens, it is not remembered, and the one
    // that was remembered goes (it is not the key file the vault was last opened with).
    let copy = rig.dir.path().join("llaves").join("copia.key");
    std::fs::copy(&rig.key, &copy).unwrap();
    rig.open(Some(PASSWORD), Some(&copy)).unwrap();
    close(&rig.app);
    assert_eq!(remembered_in(&rig.dir), None);
}

#[cfg(windows)]
#[test]
fn rf_111_c4_a_remembered_path_written_by_hand_is_validated_before_the_disk_is_asked() {
    let dir = TempDir::new("rf111-c4");
    let vault = dir.path().join("Familia.kdbx");
    let other = dir.path().join("Otra.kdbx");
    let key = dir.path().join("llave.key");
    write_key(&key);
    // Each of these is a file that IS there, or would send the question to another machine: if the app asked
    // the disk about it, the answer would not be `null`.
    let verbatim = format!(r"\\?\{}", text(&key));
    assert!(Path::new(&verbatim).is_file());
    let unc = format!(r"\\localhost\{}", text(&key).replacen(':', "$", 1));
    let bad_values = [
        json!(verbatim),
        json!(unc),
        json!(r"\\servidor.invalid\recurso\llave.key"),
        json!("llave.key"),
        json!(format!("{}:flujo", text(&key))),
        json!(7),
        json!(null),
        json!([text(&key)]),
    ];
    for bad in bad_values {
        let written = json!({
            "recent_vaults": [text(&vault)],
            // The second entry is a good path for a vault that is not in the recent list.
            "key_files": { text(&vault): bad, text(&other): text(&key) },
        })
        .to_string();
        std::fs::write(dir.settings_file(), &written).unwrap();
        let dialogs = FakeDialogs::default();
        let app = app_with(parts_with(&dir, &dialogs));

        // The same answer as with nothing remembered: `null`, not "the remembered one is missing".
        assert_eq!(suggestion(&app, &vault), Value::Null, "{bad}");
        // A vault that is not recent has nothing remembered, also once the app answers about it.
        dialogs.will_choose(&other);
        invoke_plain(&app, "pick_vault_to_open").unwrap();
        assert_eq!(suggestion(&app, &other), Value::Null, "{bad}");
        // With a key file beside the vault, the plain answer of RF-110.
        let beside = dir.path().join("Familia.keyx");
        write_key(&beside);
        assert_eq!(
            suggestion(&app, &vault),
            json!({ "key_file_path": text(&beside), "key_file": "Familia.keyx" }),
            "{bad}"
        );
        std::fs::remove_file(&beside).unwrap();
        // Reading rewrote nothing.
        assert_eq!(
            std::fs::read(dir.settings_file()).unwrap(),
            written.as_bytes()
        );
    }
}

#[cfg(windows)]
#[test]
fn rf_111_c5_the_remembered_key_file_is_looked_at_without_opening_it() {
    use std::os::windows::fs::OpenOptionsExt;

    let rig = keyed("rf111-c5");
    rig.open_once_with_the_key_chosen();
    let before = settings_file(&rig.dir);

    // Another program holds the key file and shares nothing: whoever opens it, to read or to write, fails.
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&rig.key)
        .unwrap();
    assert!(
        std::fs::read(&rig.key).is_err(),
        "the lock of the test does not hold: reading the key file still works"
    );
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    drop(held);

    assert_eq!(std::fs::read(&rig.key).unwrap(), KEY_CONTENT.as_bytes());
    assert_eq!(settings_file(&rig.dir), before);
    assert_eq!(
        files_in(&rig.dir.path().join("llaves")),
        ["mi-llave.key"],
        "nothing was created beside the key file"
    );
}

#[test]
fn rf_111_c6_a_remembered_key_file_that_is_gone_is_said_and_not_forgotten() {
    let rig = keyed("rf111-c6");
    rig.open_once_with_the_key_chosen();
    let before = settings_file(&rig.dir);
    let away = rig.dir.path().join("llaves").join("desconectada.key");
    let missing = json!({ "remembered_missing": text(&rig.key) });

    // The drive it lives on is unplugged: the answer says which file it was, and proposes none.
    std::fs::rename(&rig.key, &away).unwrap();
    assert_eq!(suggestion(&rig.app, &rig.vault), missing);
    // A folder at that path is not the key file either.
    std::fs::create_dir(&rig.key).unwrap();
    assert_eq!(suggestion(&rig.app, &rig.vault), missing);
    std::fs::remove_dir(&rig.key).unwrap();

    // With a key file beside the vault, that one is proposed, and the answer still says the other is missing.
    let beside = rig.dir.path().join("Familia.keyx");
    write_key(&beside);
    assert_eq!(
        suggestion(&rig.app, &rig.vault),
        json!({
            "key_file_path": text(&beside),
            "key_file": "Familia.keyx",
            "remembered_missing": text(&rig.key),
        })
    );

    // Nothing was forgotten for that: plugged in again, it comes already chosen as before.
    assert_eq!(settings_file(&rig.dir), before);
    std::fs::rename(&away, &rig.key).unwrap();
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    assert_eq!(settings_file(&rig.dir), before);
}

#[test]
fn rf_111_c7_a_the_key_file_is_forgotten_when_the_vault_leaves_the_recent_list() {
    let rig = keyed("rf111-c7a");

    // Removed from the list by the person: one write takes the vault and its key file.
    rig.open_once_with_the_key_chosen();
    let left = invoke_args(
        &rig.app,
        "recent_remove",
        json!({ "path": text(&rig.vault) }),
    );
    assert_eq!(left, Ok(json!([])));
    assert_eq!(remembered_in(&rig.dir), None);

    // Pushed out by five newer vaults.
    rig.open_once_with_the_key_chosen();
    let first = rig.dir.path().join("nueva-1.kdbx");
    create(&rig.app, text(&first), Some(PASSWORD), none()).unwrap();
    for n in 2..=4 {
        let next = rig.dir.path().join(format!("nueva-{n}.kdbx"));
        invoke_args(&rig.app, "vault_save_as", json!({ "path": text(&next) })).unwrap();
        assert_eq!(
            remembered_in(&rig.dir),
            Some(json!({ text(&rig.vault): text(&rig.key) })),
            "still among the five"
        );
    }
    let sixth = rig.dir.path().join("nueva-5.kdbx");
    invoke_args(&rig.app, "vault_save_as", json!({ "path": text(&sixth) })).unwrap();
    let recent = invoke_plain(&rig.app, "recent_list").unwrap();
    assert_eq!(recent.as_array().unwrap().len(), 5);
    assert!(!recent.to_string().contains("Familia.kdbx"));
    assert_eq!(remembered_in(&rig.dir), None);
}

#[test]
fn rf_111_c7_b_opening_without_a_key_file_or_with_the_one_beside_forgets_it() {
    let rig = keyed("rf111-c7b");
    let beside = rig.dir.path().join("Familia.keyx");

    // With the key file beside the vault, even chosen in the dialog: nothing is kept, it is found on its own.
    rig.open_once_with_the_key_chosen();
    std::fs::copy(&rig.key, &beside).unwrap();
    rig.pick_key(&beside);
    rig.open(Some(PASSWORD), Some(&beside)).unwrap();
    close(&rig.app);
    assert_eq!(remembered_in(&rig.dir), None);
    assert_eq!(
        suggestion(&rig.app, &rig.vault),
        json!({ "key_file_path": text(&beside), "key_file": "Familia.keyx" })
    );
    std::fs::remove_file(&beside).unwrap();

    // Without a key file: the vault at that path is now one that opens with the password alone.
    rig.open_once_with_the_key_chosen();
    let plain = rig.dir.path().join("sin-llave.kdbx");
    create(&rig.app, text(&plain), Some(PASSWORD), none()).unwrap();
    close(&rig.app);
    std::fs::copy(&plain, &rig.vault).unwrap();
    assert_eq!(
        remembered_in(&rig.dir),
        Some(json!({ text(&rig.vault): text(&rig.key) }))
    );
    let opened = rig.open(Some(PASSWORD), None).unwrap();
    assert_eq!(opened["state"], "open");
    close(&rig.app);
    assert_eq!(remembered_in(&rig.dir), None);
    assert_eq!(suggestion(&rig.app, &rig.vault), Value::Null);
}

#[test]
fn rf_111_c7_c_another_key_file_chosen_takes_the_place_of_the_remembered_one() {
    let rig = keyed("rf111-c7c");
    rig.open_once_with_the_key_chosen();

    // The person moved the key file again and chose it where it is now.
    let moved = rig.dir.path().join("llaves").join("movida.key");
    std::fs::rename(&rig.key, &moved).unwrap();
    rig.pick_key(&moved);
    rig.open(Some(PASSWORD), Some(&moved)).unwrap();
    close(&rig.app);

    assert_eq!(
        remembered_in(&rig.dir),
        Some(json!({ text(&rig.vault): text(&moved) }))
    );
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&moved));
}

#[test]
fn rf_111_c7_d_a_remembered_path_that_is_not_valid_leaves_the_file_at_the_next_write() {
    let dir = TempDir::new("rf111-c7d");
    let vault = dir.path().join("Familia.kdbx");
    let other = dir.path().join("Otra.kdbx");
    let good = dir.path().join("llave.key");
    let written = json!({
        "auto_lock_minutes": 15,
        "recent_vaults": [text(&vault), text(&other)],
        "key_files": { text(&vault): r"\\servidor.invalid\recurso\llave.key", text(&other): text(&good) },
    })
    .to_string();
    std::fs::write(dir.settings_file(), &written).unwrap();
    let app = app_with(test_parts(&dir));

    // Read and dropped in memory; the file is not rewritten for that.
    assert_eq!(suggestion(&app, &vault), Value::Null);
    assert_eq!(
        std::fs::read(dir.settings_file()).unwrap(),
        written.as_bytes()
    );

    // The next change of the settings, whatever it is, writes the file without it.
    let chosen = json!({ "auto_lock_minutes": 15, "clipboard_seconds": 30 });
    invoke_args(&app, "settings_set", chosen).unwrap();
    assert_eq!(
        remembered_in(&dir),
        Some(json!({ text(&other): text(&good) }))
    );
    let file: Value = serde_json::from_slice(&settings_file(&dir)).unwrap();
    assert_eq!(file["auto_lock_minutes"], 15);
    assert_eq!(file["recent_vaults"], json!([text(&vault), text(&other)]));
}

#[test]
fn rf_111_c10_a_vault_that_opens_with_its_key_file_alone_leaves_nothing_remembered() {
    let dir = TempDir::new("rf111-c10");
    let dialogs = FakeDialogs::default();
    let vault = dir.path().join("Solo-llave.kdbx");
    let key = dir.path().join("llave.key");
    write_key(&key);
    // The settings already remember that key file for the vault, as a version without this rule could have.
    let written = json!({
        "recent_vaults": [text(&vault)],
        "key_files": { text(&vault): text(&key) },
    });
    let app = app_with(parts_with(&dir, &dialogs));
    let existing = json!({ "kind": "existing", "path": text(&key) });
    create(&app, text(&vault), None, existing).unwrap();
    close(&app);
    std::fs::write(dir.settings_file(), written.to_string()).unwrap();

    // A fresh app reads that file: the key file comes proposed, as remembered.
    let app = app_with(parts_with(&dir, &dialogs));
    assert_eq!(suggestion(&app, &vault), remembered(&key));

    // Opened with the key file alone, chosen in the dialog: it opens, and nothing stays remembered.
    for password in [None, Some("")] {
        dialogs.will_choose(&key);
        invoke_plain(&app, "pick_key_file").unwrap();
        let opened = open(&app, &vault, password, Some(&key)).unwrap();
        assert_eq!(opened["state"], "open", "with password {password:?}");
        close(&app);
        assert_eq!(remembered_in(&dir), None, "with password {password:?}");
        assert_eq!(
            suggestion(&app, &vault),
            Value::Null,
            "with password {password:?}"
        );
    }
}

// ---------- Review 0.1, second round, U-1: a new file at a recent path does not inherit its key file ----------

#[test]
fn rf_111_u1_a_vault_created_where_a_recent_one_was_does_not_inherit_its_key_file() {
    let dir = TempDir::new("rf111-u1");
    let dialogs = FakeDialogs::default();
    let vault = dir.path().join("Recreada.kdbx");
    let key = dir.path().join("llave-vieja.key");
    write_key(&key);
    // The settings remember a key file for a recent vault whose file is gone (deleted, or moved away).
    let written = json!({
        "recent_vaults": [text(&vault)],
        "key_files": { text(&vault): text(&key) },
    });
    std::fs::write(dir.settings_file(), written.to_string()).unwrap();

    // A new vault is created at that same path, with a password and no key file.
    let app = app_with(parts_with(&dir, &dialogs));
    let created = create(&app, text(&vault), Some(PASSWORD), none()).unwrap();
    assert_eq!(created["state"], "open");
    close(&app);

    // Nothing is remembered for it and nothing is proposed: the old key file is not its key file.
    assert_eq!(remembered_in(&dir), None);
    assert_eq!(suggestion(&app, &vault), Value::Null);
}

// ---------- Review 0.1, Q-2: a huge file nobody chose is not proposed as a key file ----------

/// Makes the file at `path` exactly `bytes` long (what is added reads as zeros).
fn resize(path: &Path, bytes: u64) {
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_len(bytes).unwrap();
}

#[test]
fn rf_110_rf_111_a_key_file_larger_than_1_mib_is_not_proposed() {
    const MIB: u64 = 1024 * 1024;
    let rig = keyed("q2-size");
    let beside = rig.dir.path().join("Familia.keyx");
    let proposed_beside = json!({ "key_file_path": text(&beside), "key_file": "Familia.keyx" });

    // RF-110, the one beside the vault: up to 1 MiB it is proposed, one byte more and it is not.
    write_key(&beside);
    assert_eq!(suggestion(&rig.app, &rig.vault), proposed_beside);
    resize(&beside, MIB);
    assert_eq!(suggestion(&rig.app, &rig.vault), proposed_beside);
    resize(&beside, MIB + 1);
    assert_eq!(suggestion(&rig.app, &rig.vault), Value::Null);
    std::fs::remove_file(&beside).unwrap();

    // RF-111, the remembered one: the same limit. Too large, it is not proposed: the answer is the one of a
    // remembered key file that is not there, and it is not forgotten.
    rig.open_once_with_the_key_chosen();
    let before = settings_file(&rig.dir);
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    resize(&rig.key, MIB);
    assert_eq!(suggestion(&rig.app, &rig.vault), remembered(&rig.key));
    resize(&rig.key, MIB + 1);
    assert_eq!(
        suggestion(&rig.app, &rig.vault),
        json!({ "remembered_missing": text(&rig.key) })
    );
    assert_eq!(settings_file(&rig.dir), before);
}
