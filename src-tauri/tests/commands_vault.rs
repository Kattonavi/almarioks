//! The commands of the vault over the session of the core (T13), through the real IPC and the real allow-list:
//! no secret crosses to the window (RF-76), a locked vault answers `Bloqueada` to everything (RF-35), a copy in
//! flight never outlives a lock (RF-90), unknown ids (RF-93), strict arguments (RF-80) and the data of the
//! validation errors (RF-40). See `common/vault.rs` for the app and the canary vault.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::{
    collections::BTreeSet,
    thread,
    time::{Duration, Instant},
};

use almarioks_core::kdbx::{MasterPassword, NewVaultKdf, VaultKey, create_vault};
use almarioks_lib::{
    commands::COMMANDS,
    lock::{VAULT_LOCKED_EVENT, VAULT_SAVE_STATE_EVENT},
    state::AppState,
};
use common::{
    Answer, LOCAL_ORIGIN, error_code, invoke_raw,
    vault::{CANARY, CanaryVault, MISSING_ID, PATIENCE, Rig, copy_core_fixture, master},
};
use serde_json::{Value, json};
use tauri::{Manager, ipc::InvokeBody};

/// RF-76, ADR-0002: the only commands whose answer may hold a secret. Any other answer with the canary fails.
const MAY_ANSWER_A_SECRET: [&str; 2] = ["entry_reveal", "generate_password"];

/// Whether the command reads or changes the content of the vault, and so must answer `Bloqueada` while it is
/// closed or locked (RF-35). Every command of the build is named here: a new one makes these tests fail until
/// somebody decides which side it is on.
fn needs_open_vault(command: &str) -> bool {
    match command {
        "vault_convert"
        | "vault_retry_save"
        | "vault_save_as"
        | "vault_reload"
        | "group_tree"
        | "group_create"
        | "group_rename"
        | "group_move"
        | "group_delete_preview"
        | "group_delete"
        | "recycle_bin_empty"
        | "entry_list"
        | "entry_get"
        | "entry_create"
        | "entry_update"
        | "entry_move"
        | "entry_delete"
        | "entry_reveal"
        | "entry_copy"
        | "entry_open_url"
        | "search" => true,
        "vault_status"
        | "vault_create"
        | "vault_create_location"
        | "vault_open"
        | "vault_lock"
        | "vault_close"
        | "app_prepare_update"
        | "activity"
        | "pick_vault_to_open"
        | "pick_vault_to_create"
        | "pick_key_file"
        | "pick_new_key_file"
        | "recent_list"
        | "recent_remove"
        | "password_strength"
        | "generate_password"
        | "settings_get"
        | "settings_set" => false,
        other => panic!(
            "`{other}` is not classified: does it need an open vault? may it answer a secret?"
        ),
    }
}

/// Well-formed arguments for a command that needs an open vault; `None` for one that takes none.
fn well_formed(command: &str, vault: &CanaryVault, rig: &Rig) -> Option<Value> {
    let (root, work, bank) = (&vault.root, &vault.work, &vault.bank);
    Some(match command {
        "vault_retry_save" | "vault_reload" | "group_tree" | "recycle_bin_empty" => return None,
        "vault_convert" => json!({ "new_path": rig.path("convertida.kdbx") }),
        "vault_save_as" => json!({ "path": rig.path("otra.kdbx") }),
        "group_create" => json!({ "parent_id": root, "name": "Nuevo" }),
        "group_rename" => json!({ "id": work, "name": "Oficina" }),
        "group_move" => json!({ "id": work, "parent_id": root }),
        "group_delete_preview" | "group_delete" => json!({ "id": work }),
        "entry_list" => json!({ "group_id": work }),
        "entry_get" | "entry_delete" | "entry_open_url" => json!({ "id": bank }),
        "entry_create" => new_entry(work, "Nueva", "sin-secreto"),
        "entry_update" => json!({ "id": bank, "title": "Banco 2" }),
        "entry_move" => json!({ "id": bank, "group_id": root }),
        "entry_reveal" | "entry_copy" => json!({ "id": bank, "field": "Password" }),
        "search" => json!({ "query": "banco" }),
        other => panic!("no arguments written for `{other}`"),
    })
}

fn new_entry(group: &str, title: &str, password: &str) -> Value {
    json!({
        "group_id": group, "title": title, "username": "u", "password": password,
        "url": "", "notes": "", "tags": [],
    })
}

/// Invokes commands, remembers which ones and checks every answer for the canary.
struct Sweep<'a> {
    rig: &'a Rig,
    called: BTreeSet<String>,
}

impl Sweep<'_> {
    fn call(&mut self, command: &str, args: Option<Value>) -> Answer {
        let answer = match args {
            Some(args) => self.rig.args(command, args),
            None => self.rig.plain(command),
        };
        self.called.insert(command.to_owned());
        let (Ok(body) | Err(body)) = &answer;
        if !MAY_ANSWER_A_SECRET.contains(&command) {
            assert!(
                !body.to_string().contains(CANARY),
                "`{command}` answered a protected value (RF-76)"
            );
        }
        answer
    }

    fn ok(&mut self, command: &str, args: Option<Value>) -> Value {
        self.call(command, args)
            .unwrap_or_else(|error| panic!("`{command}` failed: {error}"))
    }

    fn code(&mut self, command: &str, args: Option<Value>) -> String {
        error_code(&self.call(command, args))
    }
}

/// Blocks on the save events (never sleeps) until the disk has confirmed every change.
fn wait_until_saved(rig: &Rig) {
    loop {
        let status = rig.plain("vault_status").unwrap();
        if status["save_pending"] == false {
            return;
        }
        rig.events.wait_for(VAULT_SAVE_STATE_EVENT, "\"saved\"");
    }
}

fn find_group<'a>(tree: &'a Value, name: &str) -> Option<&'a Value> {
    if tree["name"] == name {
        return Some(tree);
    }
    tree["children"]
        .as_array()?
        .iter()
        .find_map(|child| find_group(child, name))
}

// ---------- RF-76: the canary ----------

#[test]
#[allow(clippy::too_many_lines)]
fn rf_76_no_command_and_no_event_hands_a_protected_value_to_the_window() {
    let rig = Rig::new("rf76-canary");
    let mut sweep = Sweep {
        rig: &rig,
        called: BTreeSet::new(),
    };
    for command in COMMANDS {
        // Panics on a command nobody classified.
        needs_open_vault(command);
    }

    // Without a vault: what the person types is a secret too.
    for command in [
        "pick_vault_to_open",
        "pick_vault_to_create",
        "pick_key_file",
        "pick_new_key_file",
        "recent_list",
        "settings_get",
        "activity",
    ] {
        sweep.ok(command, None);
    }
    // Closes whatever is open before an update: with no vault it has nothing to do, and it answers nothing.
    assert_eq!(sweep.ok("app_prepare_update", None), Value::Null);
    assert_eq!(sweep.ok("vault_status", None), json!({ "state": "closed" }));
    sweep.ok(
        "recent_remove",
        Some(json!({ "path": rig.path("no.kdbx") })),
    );
    sweep.ok(
        "settings_set",
        Some(json!({ "auto_lock_minutes": 5, "clipboard_seconds": 20 })),
    );
    assert_eq!(
        sweep
            .ok("password_strength", Some(json!({ "password": master() })))
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["score"]
    );
    let generated = sweep.ok(
        "generate_password",
        Some(json!({
            "length": 20, "lower": true, "upper": true, "digits": true, "symbols": true,
            "avoid_lookalikes": false,
        })),
    );
    assert_eq!(generated["password"].as_str().unwrap().chars().count(), 20);

    // Where a new vault would go (RF-97): paths, names and booleans, with or without a chosen location.
    let proposed = sweep.ok(
        "vault_create_location",
        Some(json!({ "name": "Nueva", "chosen_path": null })),
    );
    assert_eq!(proposed["vault_file"], "Nueva.kdbx");
    let chosen = sweep.ok(
        "vault_create_location",
        Some(json!({ "name": "Nueva", "chosen_path": rig.path("nueva.kdbx") })),
    );
    assert_eq!(
        chosen.as_object().unwrap().keys().collect::<Vec<_>>(),
        [
            "in_default_folder",
            "key_file",
            "key_file_exists",
            "key_file_path",
            "vault_exists",
            "vault_file",
            "vault_path"
        ]
    );

    // Creating a vault with the canary as master password, and closing it.
    let created = sweep.ok(
        "vault_create",
        Some(json!({
            "path": rig.path("nueva.kdbx"), "name": "Nueva", "password": master(),
            "key_file": { "kind": "none" },
        })),
    );
    assert_eq!(created["state"], "open");
    assert_eq!(created["name"], "Nueva");
    assert_eq!(created["read_only"], false);
    sweep.ok("vault_close", None);
    assert_eq!(rig.state(), "closed");

    // The canary vault: a wrong key first, then the right one.
    let path = rig.write_canary_vault();
    let wrong = json!({ "path": path, "password": format!("{CANARY}-equivocada") });
    assert_eq!(sweep.code("vault_open", Some(wrong)), "LlaveIncorrecta");
    let opened = sweep.ok(
        "vault_open",
        Some(json!({ "path": path, "password": master() })),
    );
    assert_eq!(opened["state"], "open");
    assert_eq!(opened["recovery"], Value::Null);
    assert_eq!(opened["locks_in_seconds"], 300);
    let vault = rig.discover(path);
    let status = sweep.ok("vault_status", None);
    assert_eq!(status["name"], "Canario");

    // Reading.
    let tree = sweep.ok("group_tree", None);
    assert_eq!(tree["kind"], "root");
    assert_eq!(find_group(&tree, "Trabajo").unwrap()["entry_count"], 2);
    let list = sweep.ok("entry_list", Some(json!({ "group_id": vault.work })));
    assert_eq!(list.as_array().unwrap().len(), 2);
    // RF-105: both have a password; the list learns that and nothing more (the sweep looked for the canary).
    for entry in list.as_array().unwrap() {
        assert_eq!(entry["has_password"], true);
    }
    let bank = sweep.ok("entry_get", Some(json!({ "id": vault.bank })));
    assert_eq!(bank["title"], "Banco");
    assert_eq!(bank["username"], "ana");
    assert_eq!(bank["notes"], Value::Null);
    assert_eq!(bank["group_id"], json!(vault.work));
    assert_eq!(
        bank["protected_fields"],
        json!(["Password", "Notes", "PIN"])
    );
    assert_eq!(
        bank["custom_fields"],
        json!([
            { "name": "Cliente", "protected": false, "value": "12345" },
            { "name": "PIN", "protected": true },
        ])
    );
    let hidden = sweep.ok("entry_get", Some(json!({ "id": vault.all_protected })));
    for field in ["title", "username", "url", "notes"] {
        assert_eq!(hidden[field], Value::Null, "{field} is protected");
    }
    assert_eq!(
        hidden["protected_fields"],
        json!(["Title", "UserName", "Password", "URL", "Notes"])
    );
    let hits = sweep.ok("search", Some(json!({ "query": "banco" })));
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["group_path"], json!(["Canario", "Trabajo"]));
    assert_eq!(hits[0]["has_password"], true);
    // RF-61: a protected field is never searched.
    assert_eq!(
        sweep.ok("search", Some(json!({ "query": CANARY }))),
        json!([])
    );

    // RF-44: the one command that answers a protected value, and only the field asked for.
    for (field, value) in [("Password", "password"), ("PIN", "pin"), ("Notes", "notes")] {
        assert_eq!(
            sweep.ok(
                "entry_reveal",
                Some(json!({ "id": vault.bank, "field": field }))
            ),
            json!({ "value": format!("{CANARY}-{value}") })
        );
    }

    // RF-71, RF-74: the copy goes to the clipboard from Rust; the answer only says when it is cleared.
    assert_eq!(
        sweep.ok(
            "entry_copy",
            Some(json!({ "id": vault.bank, "field": "Password" }))
        ),
        json!({ "clears_in_seconds": 20 })
    );
    assert_eq!(rig.clipboard.text(), Some(format!("{CANARY}-password")));
    assert_eq!(
        sweep.ok(
            "entry_copy",
            Some(json!({ "id": vault.all_protected, "field": "UserName" }))
        ),
        json!({ "clears_in_seconds": 20 })
    );
    assert_eq!(
        sweep.ok(
            "entry_copy",
            Some(json!({ "id": vault.bank, "field": "UserName" }))
        ),
        json!({ "clears_in_seconds": null })
    );
    assert_eq!(rig.clipboard.text().as_deref(), Some("ana"));

    // RF-49: the URL is opened from Rust; a protected one that is not a web address is refused without echo.
    assert_eq!(
        sweep.ok("entry_open_url", Some(json!({ "id": vault.bank }))),
        Value::Null
    );
    assert_eq!(rig.opener.opened(), ["https://banco.example/login"]);
    assert_eq!(
        sweep.code("entry_open_url", Some(json!({ "id": vault.all_protected }))),
        "UrlNoPermitida"
    );
    assert_eq!(rig.opener.opened().len(), 1);

    // Writing: the new password is a secret from the moment it is typed.
    let made = sweep.ok(
        "entry_create",
        Some(new_entry(&vault.work, "Nueva", &format!("{CANARY}-nueva"))),
    );
    let made_id = made["id"].as_str().unwrap().to_owned();
    assert_eq!(made["protected_fields"], json!(["Password"]));
    let edited = sweep.ok(
        "entry_update",
        Some(json!({ "id": made_id, "title": "Nueva 2", "password": format!("{CANARY}-otra") })),
    );
    assert_eq!(edited["title"], "Nueva 2");
    assert_eq!(
        sweep.ok(
            "entry_reveal",
            Some(json!({ "id": made_id, "field": "Password" }))
        ),
        json!({ "value": format!("{CANARY}-otra") })
    );
    assert_eq!(
        sweep.code(
            "entry_update",
            Some(json!({ "id": made_id, "title": "  " }))
        ),
        "TituloVacio"
    );
    assert_eq!(
        sweep.ok(
            "entry_move",
            Some(json!({ "id": made_id, "group_id": vault.root }))
        ),
        Value::Null
    );
    assert_eq!(
        sweep.ok("entry_delete", Some(json!({ "id": made_id }))),
        Value::Null
    );

    let tree = sweep.ok(
        "group_create",
        Some(json!({ "parent_id": vault.root, "name": "Proyectos" })),
    );
    let group = find_group(&tree, "Proyectos").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let tree = sweep.ok(
        "group_rename",
        Some(json!({ "id": group, "name": "Planes" })),
    );
    assert!(find_group(&tree, "Planes").is_some());
    let tree = sweep.ok(
        "group_move",
        Some(json!({ "id": group, "parent_id": vault.work })),
    );
    let work = find_group(&tree, "Trabajo").unwrap();
    assert!(find_group(work, "Planes").is_some());
    assert_eq!(
        sweep.ok("group_delete_preview", Some(json!({ "id": group }))),
        json!({ "entries": 0, "subgroups": 0 })
    );
    let tree = sweep.ok("group_delete", Some(json!({ "id": group })));
    let bin = find_group(&tree, "Papelera").expect("the recycle bin was created");
    assert_eq!(bin["kind"], "recycle_bin");
    assert!(find_group(bin, "Planes").is_some());
    let tree = sweep.ok("recycle_bin_empty", None);
    assert!(find_group(&tree, "Planes").is_none());

    // Saving.
    wait_until_saved(&rig);
    assert_eq!(sweep.ok("vault_retry_save", None)["save_pending"], false);
    assert_eq!(sweep.ok("vault_reload", None)["state"], "open");
    let moved = sweep.ok(
        "vault_save_as",
        Some(json!({ "path": rig.path("copia.kdbx") })),
    );
    assert_eq!(moved["path"], json!(rig.path("copia.kdbx")));
    // Already KDBX 4: there is nothing to convert.
    assert_eq!(
        sweep.code("vault_convert", well_formed("vault_convert", &vault, &rig)),
        "ArgumentoInvalido"
    );

    // RF-28, RF-31, RF-35: locked, the clipboard is clean and every command of the vault answers `Bloqueada`,
    // whatever comes with it (`Bloqueada` wins over `ArgumentoInvalido`).
    assert_eq!(sweep.ok("vault_lock", None), Value::Null);
    assert_eq!(rig.state(), "locked");
    assert_eq!(rig.clipboard.text(), None);
    for state in ["locked", "closed"] {
        for command in COMMANDS.iter().filter(|command| needs_open_vault(command)) {
            let good = well_formed(command, &vault, &rig);
            assert_eq!(
                sweep.code(command, good),
                "Bloqueada",
                "`{command}` while {state}"
            );
            let bad = Some(json!({ "nobody": "knows this field" }));
            assert_eq!(
                sweep.code(command, bad),
                "Bloqueada",
                "`{command}` while {state}"
            );
        }
        sweep.ok("vault_close", None);
        assert_eq!(rig.state(), "closed");
    }

    // Every command of the build went through the sweep.
    let listed: BTreeSet<String> = COMMANDS.iter().map(|&command| command.to_owned()).collect();
    assert_eq!(sweep.called, listed);

    // The events the window heard: the two of the contract, and never the canary.
    let events = rig.events.all();
    assert!(events.iter().any(|(name, _)| name == VAULT_LOCKED_EVENT));
    assert!(
        events
            .iter()
            .any(|(name, body)| name == VAULT_SAVE_STATE_EVENT && body == "\"saved\"")
    );
    for (name, body) in &events {
        assert!(!body.contains(CANARY), "`{name}` carried a protected value");
        if name == VAULT_LOCKED_EVENT {
            assert_eq!(body, "null");
        } else {
            assert!(
                [
                    "\"saving\"",
                    "\"saved\"",
                    "\"failed\"",
                    "\"external-change\""
                ]
                .contains(&body.as_str()),
                "unexpected save state {body}"
            );
        }
    }
}

// ---------- RF-105: the list says whether there is a password, never the password ----------

/// The summaries `entry_list` answers for a group.
fn listed(rig: &Rig, group: &str) -> Vec<Value> {
    let list = rig
        .args("entry_list", json!({ "group_id": group }))
        .unwrap();
    assert!(
        !list.to_string().contains(CANARY),
        "`entry_list` answered a protected value (RF-105)"
    );
    list.as_array().unwrap().clone()
}

fn titled<'a>(summaries: &'a [Value], title: &str) -> &'a Value {
    summaries
        .iter()
        .find(|summary| summary["title"] == title)
        .unwrap_or_else(|| panic!("no entry titled {title}"))
}

#[test]
#[allow(clippy::too_many_lines)]
fn rf_105_entry_list_and_search_say_whether_there_is_a_password_and_never_the_password() {
    let rig = Rig::new("rf105-has-password");
    let vault = rig.open_canary_vault();

    // The canary vault: both entries of "Trabajo" have a password that holds the canary.
    let work = listed(&rig, &vault.work);
    assert_eq!(work.len(), 2);
    for summary in &work {
        assert_eq!(summary["has_password"], true);
        // Exactly these keys: nothing else about the password travels (no length, no strength, no mask).
        assert_eq!(
            summary.as_object().unwrap().keys().collect::<Vec<_>>(),
            [
                "has_password",
                "id",
                "modified",
                "tags",
                "title",
                "url",
                "username"
            ]
        );
    }

    // Entries made here: without a password, and with two passwords of different lengths.
    let empty = rig
        .args("entry_create", new_entry(&vault.root, "rf105 sin", ""))
        .unwrap();
    assert_eq!(empty["has_password"], false);
    assert_eq!(empty["protected_fields"], json!([]));
    let short = format!("{CANARY}-a");
    let long = format!("{CANARY}-{}", "b".repeat(200));
    for (title, password) in [("rf105 corta", &short), ("rf105 larga", &long)] {
        let made = rig
            .args("entry_create", new_entry(&vault.root, title, password))
            .unwrap();
        assert_eq!(made["has_password"], true);
        assert!(!made.to_string().contains(CANARY));
    }

    let root = listed(&rig, &vault.root);
    assert_eq!(titled(&root, "rf105 sin")["has_password"], false);
    assert_eq!(titled(&root, "rf105 corta")["has_password"], true);
    assert_eq!(titled(&root, "rf105 larga")["has_password"], true);
    // The two that differ only in their passwords are told apart by nothing but id, title and time.
    let comparable = |title: &str| {
        let mut summary = titled(&root, title).clone();
        for key in ["id", "title", "modified"] {
            summary.as_object_mut().unwrap().remove(key);
        }
        summary
    };
    assert_eq!(comparable("rf105 corta"), comparable("rf105 larga"));

    // The same summary travels in every search result.
    let hits = rig.args("search", json!({ "query": "rf105" })).unwrap();
    assert!(!hits.to_string().contains(CANARY));
    let hits = hits.as_array().unwrap().clone();
    assert_eq!(hits.len(), 3);
    assert_eq!(titled(&hits, "rf105 sin")["has_password"], false);
    assert_eq!(titled(&hits, "rf105 corta")["has_password"], true);
    assert_eq!(titled(&hits, "rf105 larga")["has_password"], true);
    let bank = rig.args("search", json!({ "query": "banco" })).unwrap();
    assert_eq!(bank[0]["has_password"], true);

    // Giving the empty entry a password turns it to yes; the answer still does not carry it.
    let id = titled(&root, "rf105 sin")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let edited = rig
        .args(
            "entry_update",
            json!({ "id": id, "password": format!("{CANARY}-nueva") }),
        )
        .unwrap();
    assert_eq!(edited["has_password"], true);
    assert!(!edited.to_string().contains(CANARY));
    let root = listed(&rig, &vault.root);
    assert_eq!(titled(&root, "rf105 sin")["has_password"], true);
    let got = rig.args("entry_get", json!({ "id": id })).unwrap();
    assert_eq!(got["has_password"], true);
    assert!(!got.to_string().contains(CANARY));
}

// ---------- RF-90: copy and lock, one at a time ----------

#[test]
fn rf_90_a_copy_in_flight_when_the_lock_is_asked_leaves_nothing_and_answers_bloqueada() {
    let rig = Rig::new("rf90-copy-lock");
    let vault = rig.open_canary_vault();
    let hold = rig.clipboard.hold_next_set();
    let clipboard = rig.app.state::<AppState>().clipboard().unwrap();
    let invoke_from_thread = |command: &'static str, body: Value| {
        let window = rig.window();
        thread::spawn(move || invoke_raw(&window, LOCAL_ORIGIN, command, InvokeBody::Json(body)))
    };

    let copy = invoke_from_thread(
        "entry_copy",
        json!({ "args": { "id": vault.bank, "field": "Password" } }),
    );
    // The value is on its way to the system clipboard.
    hold.inside.recv_timeout(PATIENCE).unwrap();
    let lock = invoke_from_thread("vault_lock", json!({}));
    // The lock was asked for and waits its turn behind the copy.
    let deadline = Instant::now() + PATIENCE;
    while clipboard.lock_requests() == 0 {
        assert!(
            Instant::now() < deadline,
            "the lock never reached the clipboard"
        );
        thread::yield_now();
    }
    // Commands already answer `Bloqueada` (RF-30), although the copy has not finished.
    assert_eq!(
        error_code(&rig.args("entry_get", json!({ "id": vault.bank }))),
        "Bloqueada"
    );
    hold.release.send(()).unwrap();

    assert_eq!(error_code(&copy.join().unwrap()), "Bloqueada");
    assert_eq!(lock.join().unwrap(), Ok(Value::Null));
    assert_eq!(
        rig.clipboard.text(),
        None,
        "the canary stayed in the clipboard"
    );
    assert_eq!(rig.state(), "locked");
}

#[test]
fn rf_90_a_copy_asked_after_the_lock_copies_nothing() {
    let rig = Rig::new("rf90-after");
    let vault = rig.open_canary_vault();
    rig.clipboard.other_app_copies("hola");
    rig.plain("vault_lock").unwrap();

    let late = rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    );
    assert_eq!(error_code(&late), "Bloqueada");
    // RF-31: what another app copied is not ours to clear.
    assert_eq!(rig.clipboard.text().as_deref(), Some("hola"));
}

// ---------- RF-93: unknown ids ----------

#[test]
fn rf_93_an_entry_or_group_that_does_not_exist_answers_no_encontrado() {
    let rig = Rig::new("rf93-missing");
    let vault = rig.open_canary_vault();
    let (missing, work, bank) = (MISSING_ID, &vault.work, &vault.bank);

    for (command, args) in [
        ("entry_list", json!({ "group_id": missing })),
        ("entry_get", json!({ "id": missing })),
        ("entry_create", new_entry(missing, "Nueva", "x")),
        ("entry_update", json!({ "id": missing, "title": "x" })),
        ("entry_move", json!({ "id": missing, "group_id": work })),
        ("entry_move", json!({ "id": bank, "group_id": missing })),
        ("entry_delete", json!({ "id": missing })),
        (
            "entry_reveal",
            json!({ "id": missing, "field": "Password" }),
        ),
        ("entry_reveal", json!({ "id": bank, "field": "No existe" })),
        ("entry_copy", json!({ "id": missing, "field": "Password" })),
        ("entry_copy", json!({ "id": bank, "field": "No existe" })),
        ("entry_open_url", json!({ "id": missing })),
        ("group_create", json!({ "parent_id": missing, "name": "x" })),
        ("group_rename", json!({ "id": missing, "name": "x" })),
        ("group_move", json!({ "id": missing, "parent_id": work })),
        ("group_move", json!({ "id": work, "parent_id": missing })),
        ("group_delete_preview", json!({ "id": missing })),
        ("group_delete", json!({ "id": missing })),
    ] {
        let answer = rig.args(command, args.clone());
        assert_eq!(
            error_code(&answer),
            "NoEncontrado",
            "`{command}` with {args}"
        );
    }
    // `NoEncontrado` wins over the rule of the domain (spec, "Códigos de error").
    let both = rig.args("entry_create", new_entry(missing, "   ", "x"));
    assert_eq!(error_code(&both), "NoEncontrado");
    let both = rig.args("group_rename", json!({ "id": missing, "name": "" }));
    assert_eq!(error_code(&both), "NoEncontrado");
    assert_eq!(rig.clipboard.text(), None);
    assert!(rig.opener.opened().is_empty());
}

// ---------- RF-80: strict arguments of the new commands ----------

#[test]
fn rf_80_the_vault_commands_reject_malformed_arguments_and_change_nothing() {
    let rig = Rig::new("rf80-vault");
    let vault = rig.open_canary_vault();
    let (root, work, bank) = (&vault.root, &vault.work, &vault.bank);
    let before = rig.plain("group_tree").unwrap();
    let new_vault = rig.path("otra-nueva.kdbx");

    let malformed = [
        // Unknown field.
        ("entry_get", json!({ "id": bank, "extra": 1 })),
        ("entry_update", json!({ "id": bank, "titulo": "x" })),
        (
            "group_create",
            json!({ "parent_id": root, "name": "x", "icon": 3 }),
        ),
        ("search", json!({ "query": "a", "limit": 5 })),
        (
            "vault_save_as",
            json!({ "path": rig.path("z.kdbx"), "overwrite": true }),
        ),
        // camelCase instead of snake_case.
        ("entry_list", json!({ "groupId": work })),
        ("group_move", json!({ "id": work, "parentId": root })),
        ("vault_convert", json!({ "newPath": rig.path("c.kdbx") })),
        // Wrong type.
        ("entry_get", json!({ "id": 7 })),
        ("entry_reveal", json!({ "id": bank, "field": ["Password"] })),
        ("entry_copy", json!({ "id": bank, "field": null })),
        (
            "entry_create",
            json!({
                "group_id": work, "title": "x", "username": "u", "password": "p", "url": "",
                "notes": "", "tags": "una",
            }),
        ),
        ("group_rename", json!({ "id": work, "name": 5 })),
        // RF-45: in a patch `null` is not "absent".
        ("entry_update", json!({ "id": bank, "title": null })),
        ("entry_update", json!({ "id": bank, "password": null })),
        // Missing field.
        ("entry_create", json!({ "group_id": work, "title": "x" })),
        ("entry_move", json!({ "id": bank })),
        // An id that is not a UUID, an empty path.
        ("entry_get", json!({ "id": "no-es-un-uuid" })),
        ("group_delete", json!({ "id": "" })),
        ("vault_save_as", json!({ "path": "" })),
        // `key_file` must be exactly one of its three objects (plan, "Precisiones" 4).
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": "p", "key_file": ["existing", "llave.keyx"],
            }),
        ),
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": "p", "key_file": ["none"],
            }),
        ),
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": "p",
                "key_file": { "kind": "none", "path": "llave.keyx" },
            }),
        ),
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": "p", "key_file": { "kind": "existing" },
            }),
        ),
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": "p", "key_file": { "kind": "otra", "path": "k" },
            }),
        ),
        (
            "vault_create",
            json!({ "path": new_vault, "name": "x", "password": "p", "key_file": "none" }),
        ),
        (
            "vault_create",
            json!({ "path": new_vault, "name": "x", "password": "p" }),
        ),
        (
            "vault_create",
            json!({
                "path": new_vault, "name": "x", "password": 12345, "key_file": { "kind": "none" },
            }),
        ),
        (
            "vault_open",
            json!({ "path": vault.path, "password": master(), "keyFilePath": "k" }),
        ),
        (
            "vault_open",
            json!({ "path": vault.path, "password": [master()] }),
        ),
        ("vault_open", json!({ "path": "", "password": master() })),
    ];
    for (command, args) in malformed {
        let answer = rig.args(command, args.clone());
        assert_eq!(
            error_code(&answer),
            "ArgumentoInvalido",
            "`{command}` with {args}"
        );
        let (Ok(body) | Err(body)) = &answer;
        assert!(
            !body.to_string().contains(CANARY),
            "`{command}` repeated its argument"
        );
    }
    // A command without data rejects anything sent with it; one with data rejects a positional list.
    for command in [
        "group_tree",
        "recycle_bin_empty",
        "vault_reload",
        "vault_retry_save",
    ] {
        let answer = rig.args(command, json!({ "id": bank }));
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "`{command}`");
    }
    for command in [
        "vault_status",
        "vault_lock",
        "vault_close",
        "app_prepare_update",
        "activity",
    ] {
        let answer = rig.args(command, json!({}));
        assert_eq!(error_code(&answer), "ArgumentoInvalido", "`{command}`");
    }
    let positional = common::invoke(
        &rig.app,
        "entry_reveal",
        json!({ "args": [bank, "Password"] }),
    );
    assert_eq!(error_code(&positional), "ArgumentoInvalido");

    // Nothing changed, nothing was created, and the vault is still open.
    assert_eq!(rig.plain("group_tree").unwrap(), before);
    assert!(!std::path::Path::new(&new_vault).exists());
    assert_eq!(rig.state(), "open");
}

// ---------- RF-40, RF-39, RF-52, RF-91: which field and which limit ----------

#[test]
fn rf_40_a_validation_error_says_the_field_and_the_limit() {
    let rig = Rig::new("rf40-field-data");
    let vault = rig.open_canary_vault();
    let long = "a".repeat(256);

    let mut entry = new_entry(&vault.work, &long, "x");
    let too_long = rig.args("entry_create", entry.clone()).unwrap_err();
    assert_eq!(too_long["code"], "CampoMuyLargo");
    assert_eq!(too_long["field"], "Title");
    assert_eq!(too_long["limit"], 255);

    entry["title"] = json!("Con etiqueta");
    entry["tags"] = json!(["bien", "mal;mal"]);
    let tag = rig.args("entry_create", entry).unwrap_err();
    assert_eq!(tag["code"], "EtiquetaInvalida");
    assert_eq!(tag["field"], "Tags");
    assert_eq!(tag.get("limit"), None);

    let patch = json!({ "id": vault.bank, "username": "u".repeat(256) });
    let user = rig.args("entry_update", patch).unwrap_err();
    assert_eq!(
        (&user["code"], &user["field"], &user["limit"]),
        (&json!("CampoMuyLargo"), &json!("UserName"), &json!(255))
    );
    let empty = rig
        .args("entry_update", json!({ "id": vault.bank, "title": " " }))
        .unwrap_err();
    assert_eq!(
        (&empty["code"], &empty["field"]),
        (&json!("TituloVacio"), &json!("Title"))
    );

    let group = rig
        .args(
            "group_create",
            json!({ "parent_id": vault.root, "name": long }),
        )
        .unwrap_err();
    assert_eq!(
        (&group["code"], &group["field"], &group["limit"]),
        (&json!("CampoMuyLargo"), &json!("Name"), &json!(255))
    );
    let unnamed = rig
        .args("group_rename", json!({ "id": vault.work, "name": "  " }))
        .unwrap_err();
    assert_eq!(
        (&unnamed["code"], &unnamed["field"]),
        (&json!("NombreVacio"), &json!("Name"))
    );
    // RF-57 wins over the name's rule: the recycle bin does not exist yet, so the root is used for the order
    // `NoEncontrado` › rule in `rf_93_*`; here an error without data carries no `field`.
    let missing = rig
        .args("entry_get", json!({ "id": MISSING_ID }))
        .unwrap_err();
    assert_eq!(missing.get("field"), None);
}

// ---------- RF-45: a patch changes only what it brings ----------

#[test]
fn rf_45_an_update_without_password_leaves_the_password_as_it_was() {
    let rig = Rig::new("rf45-patch");
    let vault = rig.open_canary_vault();
    let password = json!({ "id": vault.bank, "field": "Password" });
    let before = rig.args("entry_reveal", password.clone()).unwrap();

    let edited = rig
        .args(
            "entry_update",
            json!({ "id": vault.bank, "title": "Banco personal" }),
        )
        .unwrap();
    assert_eq!(edited["title"], "Banco personal");
    assert_eq!(edited["username"], "ana");
    assert_eq!(rig.args("entry_reveal", password).unwrap(), before);

    // Confirming again without changing anything touches neither the date nor the file.
    wait_until_saved(&rig);
    let again = rig
        .args(
            "entry_update",
            json!({ "id": vault.bank, "title": "Banco personal" }),
        )
        .unwrap();
    assert_eq!(again["modified"], edited["modified"]);
    assert_eq!(rig.plain("vault_status").unwrap()["save_pending"], false);
}

// ---------- RF-3, RF-4, RF-6: the order of the refusals of `vault_create` ----------

#[test]
fn rf_4_vault_create_refuses_in_the_order_of_the_plan_and_touches_nothing() {
    let rig = Rig::new("rf4-create-order");
    let taken = rig.path("ya-existe.kdbx");
    let taken_key = rig.path("ya-existe.keyx");
    std::fs::write(&taken, b"no tocar").unwrap();
    std::fs::write(&taken_key, b"tampoco").unwrap();
    let create = |path: &str, password: Option<&str>, key_file: Value| {
        let mut args = json!({ "path": path, "name": "x", "key_file": key_file });
        if let Some(password) = password {
            args["password"] = json!(password);
        }
        error_code(&rig.args("vault_create", args))
    };

    // `LlaveVacia` before `ArchivoExiste`.
    assert_eq!(
        create(&taken, None, json!({ "kind": "none" })),
        "LlaveVacia"
    );
    assert_eq!(
        create(&taken, Some(""), json!({ "kind": "none" })),
        "LlaveVacia"
    );
    // The vault's path before the key file's.
    assert_eq!(
        create(
            &taken,
            Some("p"),
            json!({ "kind": "generate", "path": taken_key })
        ),
        "ArchivoExiste"
    );
    assert_eq!(
        create(
            &rig.path("libre.kdbx"),
            Some("p"),
            json!({ "kind": "generate", "path": taken_key })
        ),
        "ArchivoExiste"
    );
    assert_eq!(
        create(
            &rig.path("libre.kdbx"),
            None,
            json!({ "kind": "existing", "path": rig.path("no.keyx") })
        ),
        "ArchivoLlaveNoEncontrado"
    );
    assert_eq!(std::fs::read(&taken).unwrap(), b"no tocar");
    assert_eq!(std::fs::read(&taken_key).unwrap(), b"tampoco");
    assert!(!std::path::Path::new(&rig.path("libre.kdbx")).exists());
    assert_eq!(rig.state(), "closed");
}

// ---------- RF-1, RF-5, RF-9, RF-19: create with a new key file, reopen, recent list ----------

#[test]
fn rf_5_a_vault_created_with_a_new_key_file_reopens_with_it_and_is_remembered() {
    let rig = Rig::new("rf5-keyfile");
    let (path, key_file) = (rig.path("con-llave.kdbx"), rig.path("llave.keyx"));
    let created = rig
        .args(
            "vault_create",
            json!({ "path": path, "name": "Con llave", "key_file": { "kind": "generate", "path": key_file } }),
        )
        .unwrap();
    assert_eq!(
        (&created["state"], &created["name"]),
        (&json!("open"), &json!("Con llave"))
    );
    assert_eq!(created["weak_kdf"], false);
    rig.plain("vault_close").unwrap();

    // Without the key file it does not open; with it, it does.
    let without = rig.args(
        "vault_open",
        json!({ "path": path, "password": "cualquiera" }),
    );
    assert_eq!(error_code(&without), "LlaveIncorrecta");
    assert_eq!(rig.state(), "closed");
    let opened = rig
        .args(
            "vault_open",
            json!({ "path": path, "key_file_path": key_file }),
        )
        .unwrap();
    assert_eq!(opened["state"], "open");

    // RF-19: opening or creating remembers the path, the most recent first.
    let recent = rig.plain("recent_list").unwrap();
    assert_eq!(recent, json!([{ "path": path, "exists": true }]));
    // RF-95.
    rig.plain("vault_close").unwrap();
    let gone = rig.args(
        "vault_open",
        json!({ "path": rig.path("no-esta.kdbx"), "password": "p" }),
    );
    assert_eq!(error_code(&gone), "ArchivoNoEncontrado");
}

// ---------- RF-14, RF-15: KDBX 3.x is read-only until it is converted ----------

#[test]
fn rf_15_a_kdbx3_vault_is_read_only_and_converts_to_a_new_file_leaving_the_original_intact() {
    let rig = Rig::new("rf15-convert");
    let old = copy_core_fixture(&rig, "keepass2/v3-aes.kdbx", "vieja.kdbx");
    let original = std::fs::read(&old).unwrap();
    let opened = rig
        .args(
            "vault_open",
            json!({ "path": old, "password": "lab-Maestra-2026" }),
        )
        .unwrap();
    assert_eq!(opened["read_only"], true);
    let root = rig.plain("group_tree").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // RF-14, and `SoloLectura` wins over `NoEncontrado` and over the rule of the domain.
    for (command, args) in [
        ("group_create", json!({ "parent_id": root, "name": "x" })),
        (
            "group_create",
            json!({ "parent_id": MISSING_ID, "name": "" }),
        ),
        ("entry_create", new_entry(MISSING_ID, " ", "x")),
        ("entry_delete", json!({ "id": MISSING_ID })),
        ("vault_save_as", json!({ "path": rig.path("no.kdbx") })),
    ] {
        assert_eq!(
            error_code(&rig.args(command, args)),
            "SoloLectura",
            "`{command}`"
        );
    }
    assert_eq!(
        rig.plain("recycle_bin_empty")
            .map_err(|e| e["code"].clone()),
        Err(json!("SoloLectura"))
    );

    // RF-15: nothing is written over an existing file, and the conversion goes to a new one.
    let taken = rig.args("vault_convert", json!({ "new_path": old }));
    assert_eq!(error_code(&taken), "ArchivoExiste");
    let new = rig.path("nueva-v4.kdbx");
    let converted = rig
        .args("vault_convert", json!({ "new_path": new }))
        .unwrap();
    assert_eq!(converted["state"], "open");
    assert_eq!(converted["path"], json!(new));
    assert_eq!(converted["read_only"], false);
    assert_eq!(converted["weak_kdf"], false);
    assert_eq!(
        std::fs::read(&old).unwrap(),
        original,
        "the original changed"
    );

    // The session goes on in the new file: it can be edited, and it reopens with the same key.
    rig.args(
        "group_create",
        json!({ "parent_id": root, "name": "Ya se puede" }),
    )
    .unwrap();
    wait_until_saved(&rig);
    rig.plain("vault_close").unwrap();
    let reopened = rig
        .args(
            "vault_open",
            json!({ "path": new, "password": "lab-Maestra-2026" }),
        )
        .unwrap();
    assert_eq!(reopened["read_only"], false);
    assert!(find_group(&rig.plain("group_tree").unwrap(), "Ya se puede").is_some());
    assert_eq!(std::fs::read(&old).unwrap(), original);
}

// ---------- RF-18: the state answers while a key is derived ----------

#[test]
fn rf_18_vault_status_answers_while_another_command_derives_the_key() {
    let rig = Rig::new("rf18-status");
    // A vault that takes seconds to unlock: 25 iterations instead of the 2 of the other tests.
    let key = VaultKey::new(Some(MasterPassword::new("rf18-maestra".to_owned())), None).unwrap();
    let slow = create_vault(&key, "Lenta", NewVaultKdf::with_iterations(25).unwrap()).unwrap();
    let path = rig.path("lenta.kdbx");
    std::fs::write(&path, slow).unwrap();

    let window = rig.window();
    let body = json!({ "args": { "path": path, "password": "rf18-maestra" } });
    let opening = thread::spawn(move || {
        invoke_raw(&window, LOCAL_ORIGIN, "vault_open", InvokeBody::Json(body))
    });
    let mut while_deriving = Vec::new();
    while !opening.is_finished() {
        let asked = Instant::now();
        let status = rig.plain("vault_status").unwrap();
        let took = asked.elapsed();
        if status["state"] == "deriving" {
            assert_eq!(status["path"], json!(path));
            while_deriving.push(took);
        }
    }
    assert_eq!(opening.join().unwrap().unwrap()["state"], "open");

    assert!(
        !while_deriving.is_empty(),
        "the status never said `deriving`"
    );
    while_deriving.sort_unstable();
    // The median, not the slowest: nextest runs other tests at the same time, each deriving keys on every core,
    // and a single late answer would say more about the machine than about the command. A status that waited
    // for the derivation would take seconds every time.
    let median = while_deriving[while_deriving.len() / 2];
    assert!(
        median < Duration::from_millis(200),
        "vault_status took {median:?} (median of {}) while deriving; slowest {:?}",
        while_deriving.len(),
        while_deriving.last()
    );
}
