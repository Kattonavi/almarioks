//! One instance per session (RF-81). Automatic: the single-instance plugin is the first one registered, and the
//! app is assembled from that same list. What it does when a second instance starts needs two real processes
//! and a real window:
//!
//! Manual check, once per release (spec, "Cómo se prueba", row 81): start AlmarioKS twice. The second start
//! opens no window and the first one comes to the front; repeat with the first window minimized and with it
//! behind another app.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use almarioks_lib::{MAIN_WINDOW, plugins, show_main_window};
use common::{SINGLE_INSTANCE, TempDir, app_with, main_window, test_parts, test_plugins};
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

fn names() -> Vec<&'static str> {
    plugins::<MockRuntime>()
        .iter()
        .map(|plugin| plugin.name())
        .collect()
}

#[test]
fn rf_81_single_instance_is_the_first_plugin_registered() {
    let names = names();

    assert_eq!(names.first(), Some(&SINGLE_INSTANCE), "order: {names:?}");
    assert_eq!(
        names
            .iter()
            .filter(|name| **name == SINGLE_INSTANCE)
            .count(),
        1
    );
    // The whole list, so that adding a plugin is a decision someone reads here.
    assert_eq!(names, [SINGLE_INSTANCE, "dialog"]);
}

#[test]
fn rf_81_the_app_is_assembled_from_that_list_and_nothing_goes_before_it() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // `run` hands the real builder, untouched, and the list to `app_builder`…
    assert!(
        code.contains("app_builder(tauri::Builder::default(), plugins())"),
        "`run` must register `plugins()` on a fresh builder"
    );
    // …which registers the list before anything else, in order…
    assert!(code.contains(".fold(base, tauri::Builder::plugin_boxed)"));
    // …and no plugin is registered on the builder anywhere else. (The updater is registered later, on the
    // running app, by `install`.)
    assert_eq!(code.matches("plugin_boxed").count(), 1);
    assert_eq!(
        code.matches(".plugin(").count(),
        1,
        "only `install` registers a plugin by hand"
    );
    assert!(code.contains("app.plugin(tauri_plugin_updater::Builder::new().build())"));
}

#[test]
fn rf_81_the_tests_never_start_the_single_instance_plugin() {
    // It would take the production mutex: a test process could exit thinking it is a second instance, or wake
    // up the installed app.
    let names: Vec<&str> = test_plugins().iter().map(|plugin| plugin.name()).collect();
    assert_eq!(names, ["dialog"]);
}

#[test]
fn rf_81_showing_the_main_window_works_with_and_without_a_window() {
    let dir = TempDir::new("rf81-show");
    let app = app_with(test_parts(&dir));
    assert_eq!(main_window(&app).label(), MAIN_WINDOW);
    show_main_window(app.handle());

    // The second start can arrive before the window exists, or after it was closed: nothing to do, no panic.
    let bare = mock_builder()
        .build(mock_context(noop_assets()))
        .expect("an app without windows");
    show_main_window(bare.handle());
}
