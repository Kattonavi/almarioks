//! The app of the tests: the REAL context (`tauri.conf.json`, the capability and the command allow-list that
//! `build.rs` generates) on Tauri's mock runtime, over a temporary settings file. Calls go through the same IPC
//! entry the window uses, so the ACL, the handler and the argument checks are the shipped ones.
//!
//! Not here on purpose: the single-instance plugin (it would see the test processes, or the installed app, as
//! "another instance" and exit) and the real clipboard.

// Each test binary uses its own subset of these helpers.
#![allow(dead_code)]
// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to the tests).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use almarioks_lib::{
    app_builder,
    clipboard::ClipboardError,
    plugins,
    state::{AppParts, SETTINGS_FILE_NAME},
};
use serde_json::Value;
use tauri::{
    App, Manager, WebviewWindow,
    ipc::{CallbackFn, InvokeBody},
    plugin::Plugin,
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder},
    webview::InvokeRequest,
};

/// Name of the plugin that must never run inside a test process.
pub const SINGLE_INSTANCE: &str = "single-instance";

/// The origin of the app's own window on Windows.
pub const LOCAL_ORIGIN: &str = "http://tauri.localhost";

/// A folder of this test only, removed when the test ends. nextest runs each test in its own process, so the
/// process id plus the tag is unique.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("almarioks-app-test-{}-{tag}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("clean a leftover test folder");
        }
        std::fs::create_dir_all(&dir).expect("create the test folder");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn settings_file(&self) -> PathBuf {
        self.0.join(SETTINGS_FILE_NAME)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a leftover folder in the temp directory is harmless.
        drop(std::fs::remove_dir_all(&self.0));
    }
}

/// The plugins of the app without the single-instance one.
pub fn test_plugins() -> Vec<Box<dyn Plugin<MockRuntime>>> {
    plugins::<MockRuntime>()
        .into_iter()
        .filter(|plugin| plugin.name() != SINGLE_INSTANCE)
        .collect()
}

/// The parts of a test app: settings in `dir`, no clipboard.
pub fn test_parts(dir: &TempDir) -> AppParts {
    AppParts {
        settings_file: dir.settings_file(),
        clipboard: Err(ClipboardError::Unavailable),
    }
}

/// Builds the app with the given parts over the real context. The mock runtime runs no setup hook and creates
/// no window from the configuration, so both are done here: the same `install` the real setup calls, and the
/// `main` window the capability names.
pub fn app_with(parts: AppParts) -> App<MockRuntime> {
    let app = app_builder(mock_builder(), test_plugins())
        .build(almarioks_lib::context())
        .expect("build the test app");
    almarioks_lib::install(app.handle(), parts).expect("install the state");
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .expect("the main window");
    app
}

/// The `main` window, the only one the capability names.
pub fn main_window(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    app.get_webview_window("main").expect("the main window")
}

/// What the window gets back: `Ok(value)` or `Err(rejection)`.
pub type Answer = Result<Value, Value>;

/// Invokes `command` from `window` at `origin` with a raw body.
pub fn invoke_raw(
    window: &WebviewWindow<MockRuntime>,
    origin: &str,
    command: &str,
    body: InvokeBody,
) -> Answer {
    let request = InvokeRequest {
        cmd: command.to_owned(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: origin.parse().expect("origin is a URL"),
        body,
        headers: tauri::http::HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_owned(),
    };
    get_ipc_response(window, request).map(|body| {
        body.deserialize::<Value>()
            .expect("the answer is JSON, never raw bytes")
    })
}

/// Invokes `command` from the main window, as the app's own page, with a JSON body.
pub fn invoke(app: &App<MockRuntime>, command: &str, body: Value) -> Answer {
    invoke_raw(
        &main_window(app),
        LOCAL_ORIGIN,
        command,
        InvokeBody::Json(body),
    )
}

/// Invokes a command the way `src/ipc/client.ts` does when it has data: `invoke(command, { args })`.
pub fn invoke_args(app: &App<MockRuntime>, command: &str, args: Value) -> Answer {
    invoke(app, command, serde_json::json!({ "args": args }))
}

/// Invokes a command the way `src/ipc/client.ts` does when it has no data: `invoke(command)`.
pub fn invoke_plain(app: &App<MockRuntime>, command: &str) -> Answer {
    invoke(app, command, serde_json::json!({}))
}

/// The `code` of an `AppError` answer. Panics if the answer is anything else (success, or Tauri's own text).
pub fn error_code(answer: &Answer) -> String {
    let error = answer
        .as_ref()
        .expect_err("the command was expected to fail");
    error
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("the error is not an AppError object: {error}"))
        .to_owned()
}

/// Whether the answer is an `AppError` object (`{ code, message, … }`), as opposed to Tauri's own text.
pub fn is_app_error(answer: &Answer) -> bool {
    matches!(answer, Err(Value::Object(fields)) if fields.contains_key("code") && fields.contains_key("message"))
}
