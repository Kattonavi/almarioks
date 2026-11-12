//! Guards the shipped webview configuration (AMENAZAS W2, RF-77): reads the REAL
//! `src-tauri/tauri.conf.json`, so a careless edit that lets WebView2 remember typed text fails CI.
//!
//! Found by the first run in the real window (`docs/specs/001-boveda/validacion-app.md`, D-1 and D-2):
//! WebView2 wrote what was typed in the forms to `EBWebView/Default/Web Data`, even with
//! `autocomplete="off"`. Private mode stops it (verified on the installed app); general autofill is
//! switched off as a second barrier.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;

fn config() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = std::fs::read_to_string(&path).expect("read tauri.conf.json");
    serde_json::from_str(&text).expect("tauri.conf.json is valid JSON")
}

#[test]
fn rf_77_every_window_runs_in_private_mode_with_general_autofill_off() {
    let config = config();
    let windows = config["app"]["windows"].as_array().expect("windows array");
    assert!(!windows.is_empty(), "at least the main window");
    for window in windows {
        let label = window["label"].as_str().unwrap_or("<no label>");
        assert_eq!(
            window["incognito"].as_bool(),
            Some(true),
            "window `{label}` must run in private mode: WebView2 otherwise stores typed text on disk"
        );
        assert_eq!(
            window["generalAutofillEnabled"].as_bool(),
            Some(false),
            "window `{label}` must switch general autofill off"
        );
    }
}

#[test]
fn rf_77_no_window_is_created_outside_the_configuration() {
    // Windows built in code would not inherit the two settings above. Today there are none: the only
    // window is the configured `main`. This test pins that assumption to the source.
    let lib = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src/lib.rs");
    assert!(
        !lib.contains("WebviewWindowBuilder"),
        "a window built in code must set incognito(true) and general_autofill_enabled(false); update this test with it"
    );
}
