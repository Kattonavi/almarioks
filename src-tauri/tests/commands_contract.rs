//! The contract every command keeps (spec 001, section J): the allow-list (RF-79), strict arguments (RF-80),
//! Spanish messages without the code (RF-83) and no new network permission (RF-84). Runs on the real context:
//! see `common/mod.rs`.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeSet;

use almarioks_core::CoreError;
use almarioks_lib::{
    clipboard::ClipboardError,
    commands::{APP_ONLY_CODES, AppError, COMMANDS, SecretArg, expect_no_args, parse_args},
};
use common::{
    Answer, LOCAL_ORIGIN, TempDir, app_with, error_code, invoke, invoke_plain, invoke_raw,
    is_app_error, main_window, test_parts,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::ipc::InvokeBody;

/// Fake data, never a real password.
const CANARY: &str = "CANARIO-ALMARIOKS-argumento";

/// Every code of the core. `CoreError` is `non_exhaustive` and does not export its list, so it is repeated here;
/// `rf_83_the_window_knows_every_code_of_the_core` fails if the window's list and this one drift apart.
const CORE_ERRORS: [CoreError; 25] = [
    CoreError::Bloqueada,
    CoreError::Ocupada,
    CoreError::ArgumentoInvalido,
    CoreError::SoloLectura,
    CoreError::NoEncontrado,
    CoreError::LlaveIncorrecta,
    CoreError::LlaveVacia,
    CoreError::ArchivoDanado,
    CoreError::FormatoNoSoportado,
    CoreError::ArchivoLlaveDanado,
    CoreError::ParametrosKdfExcesivos,
    CoreError::ArchivoExiste,
    CoreError::TituloVacio,
    CoreError::NombreVacio,
    CoreError::CampoMuyLargo,
    CoreError::CaracterNoPermitido,
    CoreError::EtiquetaInvalida,
    CoreError::UrlNoPermitida,
    CoreError::MovimientoInvalido,
    CoreError::GrupoProtegido,
    CoreError::ReglasImposibles,
    CoreError::GuardadoFallido,
    CoreError::CambioExterno,
    CoreError::ArchivoNoEncontrado,
    CoreError::ArchivoLlaveNoEncontrado,
];

fn read(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn capability() -> Value {
    serde_json::from_str(&read("capabilities/main-window.json")).expect("the capability is JSON")
}

fn permissions() -> BTreeSet<String> {
    capability()["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .map(|permission| {
            permission
                .as_str()
                .expect("a permission identifier")
                .to_owned()
        })
        .collect()
}

/// Tauri's own rejection is a line of text; ours is an `AppError` object.
fn tauri_rejection(answer: &Answer) -> &str {
    match answer {
        Err(Value::String(text)) => text,
        other => panic!("expected Tauri to reject the call, got {other:?}"),
    }
}

// ---------- RF-79: the allow-list ----------

#[test]
fn rf_79_a_command_that_is_not_in_the_list_is_rejected() {
    let dir = TempDir::new("rf79-unlisted");
    let app = app_with(test_parts(&dir));

    // Names the window could try: an invented one, and vault commands that do not exist in this build.
    for command in [
        "vault_export_everything",
        "entry_reveal_all",
        "settings_reset",
        "",
    ] {
        let answer = invoke_plain(&app, command);
        let text = tauri_rejection(&answer);
        assert!(
            text.contains("not allowed") || text.contains("not found"),
            "`{command}` was not rejected as unknown: {text}"
        );
    }
}

#[test]
fn rf_79_plugin_commands_are_out_of_reach_of_the_window() {
    let dir = TempDir::new("rf79-plugins");
    let app = app_with(test_parts(&dir));

    // The dialog plugin IS registered (Rust uses it), and still the window cannot call it: the capability has
    // no `dialog:*` permission. The others are not even registered.
    for command in [
        "plugin:dialog|open",
        "plugin:dialog|save",
        "plugin:dialog|message",
        "plugin:opener|open_url",
        "plugin:opener|open_path",
        "plugin:fs|read_file",
        "plugin:shell|execute",
        "plugin:http|fetch",
        "plugin:updater|download_and_install",
    ] {
        let answer = invoke(
            &app,
            command,
            json!({ "path": "C:/x", "url": "https://x.test" }),
        );
        let text = tauri_rejection(&answer);
        assert!(
            text.contains("not allowed"),
            "`{command}` was not stopped by the ACL: {text}"
        );
    }
}

#[test]
fn rf_79_every_listed_command_reaches_its_handler() {
    let dir = TempDir::new("rf79-listed");
    let app = app_with(test_parts(&dir));

    for command in COMMANDS {
        // With an empty body a command either answers, or says `ArgumentoInvalido` because it wants `args`, or
        // says `Bloqueada` because it needs an open vault (RF-35, which wins over the arguments). All three prove
        // the ACL let it through and the handler knows it; Tauri's text would prove the opposite.
        let answer = invoke_plain(&app, command);
        assert!(
            answer.is_ok()
                || ["ArgumentoInvalido", "Bloqueada"].contains(&error_code(&answer).as_str()),
            "`{command}` is listed but did not reach its handler: {answer:?}"
        );
    }
    assert_eq!(
        invoke_plain(&app, "settings_get"),
        Ok(json!({ "auto_lock_minutes": 5, "clipboard_seconds": 20 }))
    );
}

#[test]
fn rf_79_a_listed_command_is_rejected_from_another_origin_or_window() {
    let dir = TempDir::new("rf79-origin");
    let app = app_with(test_parts(&dir));
    let main = main_window(&app);

    let remote = invoke_raw(
        &main,
        "https://evil.example",
        "settings_get",
        InvokeBody::Json(json!({})),
    );
    assert!(tauri_rejection(&remote).contains("not allowed"));

    let other = tauri::WebviewWindowBuilder::new(&app, "other", tauri::WebviewUrl::default())
        .build()
        .expect("a second window");
    let from_other = invoke_raw(
        &other,
        LOCAL_ORIGIN,
        "settings_get",
        InvokeBody::Json(json!({})),
    );
    assert!(tauri_rejection(&from_other).contains("not allowed"));

    assert!(invoke_plain(&app, "settings_get").is_ok());
}

#[test]
fn rf_79_the_capability_allows_exactly_the_listed_commands() {
    let mut expected: BTreeSet<String> = [
        "core:app:allow-version",
        "core:event:allow-listen",
        "core:event:allow-unlisten",
        "updater:allow-check",
        "updater:allow-download",
        "updater:allow-install",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    expected.extend(
        COMMANDS
            .iter()
            .map(|command| format!("allow-{}", command.replace('_', "-"))),
    );

    assert_eq!(permissions(), expected);

    let mut sorted = COMMANDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), COMMANDS.len(), "a command is listed twice");

    let capability = capability();
    assert_eq!(capability["windows"], json!(["main"]));
    assert!(
        capability.get("remote").is_none(),
        "no remote origin may invoke commands"
    );
}

// ---------- RF-80: strict arguments ----------

/// Bodies that are not what `settings_set` expects, each for a different reason.
fn malformed_settings_bodies() -> Vec<(&'static str, InvokeBody)> {
    let good = json!({ "auto_lock_minutes": 15, "clipboard_seconds": 30 });
    vec![
        ("no body", InvokeBody::Json(Value::Null)),
        ("empty object", InvokeBody::Json(json!({}))),
        ("args is null", InvokeBody::Json(json!({ "args": null }))),
        (
            "args is a list",
            InvokeBody::Json(json!({ "args": [15, 30] })),
        ),
        ("args is text", InvokeBody::Json(json!({ "args": CANARY }))),
        ("fields at the top level", InvokeBody::Json(good.clone())),
        (
            "extra top-level key",
            InvokeBody::Json(json!({ "args": good, "extra": 1 })),
        ),
        (
            "camelCase fields",
            InvokeBody::Json(json!({ "args": { "autoLockMinutes": 15, "clipboardSeconds": 30 } })),
        ),
        (
            "unknown field",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": 15, "clipboard_seconds": 30, "never": true } }),
            ),
        ),
        (
            "missing field",
            InvokeBody::Json(json!({ "args": { "auto_lock_minutes": 15 } })),
        ),
        (
            "text instead of number",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": CANARY, "clipboard_seconds": 30 } }),
            ),
        ),
        (
            "numeric text",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": "15", "clipboard_seconds": 30 } }),
            ),
        ),
        (
            "negative number",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": -15, "clipboard_seconds": 30 } }),
            ),
        ),
        (
            "decimal number",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": 15.5, "clipboard_seconds": 30 } }),
            ),
        ),
        (
            "number too big for its type",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": 15, "clipboard_seconds": 4_294_967_296_u64 } }),
            ),
        ),
        (
            "null field",
            InvokeBody::Json(
                json!({ "args": { "auto_lock_minutes": null, "clipboard_seconds": 30 } }),
            ),
        ),
        ("raw bytes", InvokeBody::Raw(CANARY.as_bytes().to_vec())),
    ]
}

#[test]
fn rf_80_a_malformed_argument_always_answers_argumento_invalido_and_changes_nothing() {
    let dir = TempDir::new("rf80-malformed");
    let app = app_with(test_parts(&dir));
    let main = main_window(&app);
    let expected = json!({
        "code": "ArgumentoInvalido",
        "message": CoreError::ArgumentoInvalido.message(),
    });

    for (case, body) in malformed_settings_bodies() {
        let answer = invoke_raw(&main, LOCAL_ORIGIN, "settings_set", body);
        // The whole error, key by key: an `AppError` object and nothing of serde's or Tauri's text.
        assert_eq!(answer, Err(expected.clone()), "case: {case}");
    }

    assert!(
        !dir.settings_file().exists(),
        "a rejected call must not write the settings file"
    );
    assert_eq!(
        invoke_plain(&app, "settings_get"),
        Ok(json!({ "auto_lock_minutes": 5, "clipboard_seconds": 20 }))
    );
}

#[test]
fn rf_80_a_command_without_data_rejects_anything_sent_with_it() {
    let dir = TempDir::new("rf80-no-data");
    let app = app_with(test_parts(&dir));
    let main = main_window(&app);

    for command in ["settings_get", "recent_list"] {
        for body in [
            InvokeBody::Json(json!({ "args": {} })),
            InvokeBody::Json(json!({ "anything": 1 })),
            InvokeBody::Json(json!([])),
            InvokeBody::Json(json!(CANARY)),
            InvokeBody::Raw(vec![1, 2, 3]),
        ] {
            let answer = invoke_raw(&main, LOCAL_ORIGIN, command, body);
            assert_eq!(error_code(&answer), "ArgumentoInvalido", "{command}");
        }
        assert!(invoke_plain(&app, command).is_ok(), "{command}");
        assert!(invoke(&app, command, Value::Null).is_ok(), "{command}");
    }
}

#[test]
fn rf_80_the_rejection_never_repeats_the_value_it_rejected() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Numbers {
        #[allow(dead_code)]
        length: u32,
    }

    // serde's own text for these says `invalid type: string "CANARIO…"` or `unknown field `CANARIO…``.
    for body in [
        json!({ "args": { "length": CANARY } }),
        json!({ "args": { "length": 8, CANARY: true } }),
        json!({ "args": CANARY }),
        json!({ CANARY: { "length": 8 } }),
    ] {
        let error = parse_args::<Numbers>(&InvokeBody::Json(body))
            .err()
            .expect("malformed arguments");
        assert_eq!(error, AppError::invalid_argument());
        let text = format!("{error:?} {}", serde_json::to_string(&error).unwrap());
        assert!(!text.contains(CANARY), "the rejected value leaked: {text}");
    }
    let error = expect_no_args(&InvokeBody::Json(json!({ CANARY: CANARY }))).unwrap_err();
    assert!(!format!("{error:?}").contains(CANARY));
}

#[test]
fn rf_80_well_formed_arguments_are_read_and_a_secret_argument_is_wrapped() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Typed {
        password: SecretArg,
    }

    let typed: Typed = parse_args(&InvokeBody::Json(json!({ "args": { "password": CANARY } })))
        .expect("well-formed arguments");
    assert_eq!(typed.password.expose(), CANARY);

    assert!(parse_args::<Typed>(&InvokeBody::Json(json!({ "args": { "password": 7 } }))).is_err());
    assert!(
        parse_args::<Typed>(&InvokeBody::Json(
            json!({ "args": { "password": CANARY, "also": CANARY } })
        ))
        .is_err()
    );
}

// ---------- RF-83: every error has a Spanish message without the code ----------

fn all_app_errors() -> Vec<AppError> {
    let mut errors: Vec<AppError> = CORE_ERRORS.into_iter().map(AppError::from).collect();
    errors.push(ClipboardError::Unavailable.into());
    errors.push(AppError::internal());
    errors
}

#[test]
fn rf_83_every_code_has_a_spanish_message_without_the_code() {
    let errors = all_app_errors();
    let codes: BTreeSet<&str> = errors.iter().map(AppError::code).collect();
    assert_eq!(codes.len(), errors.len(), "two errors share a code");
    for code in APP_ONLY_CODES {
        assert!(codes.contains(code), "{code} is not covered");
    }

    for error in &errors {
        let (code, message) = (error.code(), error.message());
        assert!(
            message.chars().count() >= 15 && message.ends_with('.'),
            "{code}: `{message}` is not a sentence for a person"
        );
        for known in &codes {
            assert!(
                !message.contains(known),
                "{code}: the message shows the code {known}"
            );
        }
        assert!(
            !message.contains('\\') && !message.contains(":/") && !message.contains("Error("),
            "{code}: the message carries a path or an internal detail"
        );
        // Spanish, not a leftover English text: every message of the spec has one of these words.
        let lower = message.to_lowercase();
        assert!(
            [
                " la ", " el ", " se ", " no ", " un ", " una ", " de ", " que "
            ]
            .iter()
            .any(|word| format!(" {lower}").contains(word)),
            "{code}: `{message}` does not read as Spanish"
        );
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "code": code, "message": message }),
            "{code}: an error without field data is exactly code and message"
        );
    }
}

#[test]
fn rf_83_conversions_keep_the_code_and_the_message_of_the_core() {
    for error in CORE_ERRORS {
        let app = AppError::from(error);
        assert_eq!(
            (app.code(), app.message()),
            (error.code(), error.message().as_str())
        );
        // The clipboard's wrapper of a core error is the core error.
        assert_eq!(AppError::from(ClipboardError::Core(error)), app);
    }
    assert_eq!(
        AppError::from(ClipboardError::Unavailable).code(),
        "PortapapelesNoDisponible"
    );
}

#[test]
fn rf_83_a_field_error_tells_the_field_and_the_limit() {
    // Plan, "Precisiones" 15 (RF-40): the window must not have to guess which field or which limit.
    let too_long = "n".repeat(100_000);
    let error = almarioks_core::vault::validate_group_name(&too_long)
        .expect_err("a group name of 100 000 characters is too long");
    let limit = error.limit().expect("CampoMuyLargo carries its limit");
    let app = AppError::from(error);

    assert_eq!(app.code(), "CampoMuyLargo");
    assert_eq!((app.field(), app.limit()), (Some("Name"), Some(limit)));
    assert_eq!(
        serde_json::to_value(&app).unwrap(),
        json!({
            "code": "CampoMuyLargo",
            "message": CoreError::CampoMuyLargo.message(),
            "field": "Name",
            "limit": limit,
        })
    );

    let empty = almarioks_core::vault::validate_group_name("   ").expect_err("an empty name");
    assert_eq!(
        serde_json::to_value(AppError::from(empty)).unwrap(),
        json!({
            "code": "NombreVacio",
            "message": CoreError::NombreVacio.message(),
            "field": "Name",
        })
    );
}

#[test]
fn rf_83_the_window_knows_every_code_of_the_core() {
    // `src/ipc/errors.ts` holds the window's own Spanish text per code; a code missing there shows as the
    // generic "Inesperado". Decided on 2026-11-11: the window also has a text for the codes only the app
    // shell sends (`APP_ONLY_CODES`), so they are required here exactly like the core's.
    let source = read("../src/ipc/errors.ts");
    let list = source
        .split_once("export const ERROR_CODES = [")
        .and_then(|(_, rest)| rest.split_once("] as const"))
        .map(|(list, _)| list)
        .expect("ERROR_CODES in src/ipc/errors.ts");
    let window_codes: BTreeSet<&str> = list
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split_once('"'))
        .map(|(code, _)| code)
        .collect();

    let core_codes: BTreeSet<&str> = CORE_ERRORS
        .iter()
        .map(|error| error.code())
        .chain(APP_ONLY_CODES)
        .collect();
    let missing: Vec<&&str> = core_codes.difference(&window_codes).collect();
    assert!(missing.is_empty(), "the window has no text for {missing:?}");

    let window_only: BTreeSet<&str> = ["RespuestaInvalida", "Inesperado"].into_iter().collect();
    let unknown_to_core: Vec<&&str> = window_codes
        .difference(&core_codes)
        .filter(|code| !window_only.contains(**code))
        .collect();
    assert!(
        unknown_to_core.is_empty(),
        "the window lists codes neither the core nor the app sends: {unknown_to_core:?}"
    );
}

// ---------- RF-84: no new network permission ----------

#[test]
fn rf_84_the_window_has_no_network_file_or_shell_permission() {
    for permission in permissions() {
        let is_app_command = permission.starts_with("allow-");
        let is_known_plugin_permission = [
            "core:app:allow-version",
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            "updater:allow-check",
            "updater:allow-download",
            "updater:allow-install",
        ]
        .contains(&permission.as_str());
        assert!(
            is_app_command || is_known_plugin_permission,
            "unexpected permission {permission}"
        );
        for forbidden in [
            "fs:",
            "shell:",
            "http:",
            "dialog:",
            "opener:",
            "websocket:",
            "default",
        ] {
            assert!(
                !permission.contains(forbidden),
                "{permission} opens {forbidden}"
            );
        }
    }

    // The CSP still lets the page talk only to the app's own IPC.
    let config: Value = serde_json::from_str(&read("tauri.conf.json")).expect("tauri.conf.json");
    let csp = &config["app"]["security"]["csp"];
    assert_eq!(csp["connect-src"], json!("ipc: http://ipc.localhost"));
    assert_eq!(csp["default-src"], json!("'self'"));
    assert_eq!(csp["script-src"], json!("'self'"));
    assert_eq!(config["app"]["withGlobalTauri"], json!(false));
    assert_eq!(
        config["app"]["security"]["capabilities"],
        json!(["main-window"])
    );
}

#[test]
fn helper_is_app_error_tells_our_errors_from_tauri_text() {
    // Guards the helper the other tests lean on.
    assert!(is_app_error(&Err(json!({ "code": "X", "message": "y" }))));
    assert!(!is_app_error(&Err(json!("Command x not allowed by ACL"))));
    assert!(!is_app_error(&Ok(json!({ "code": "X", "message": "y" }))));
}
