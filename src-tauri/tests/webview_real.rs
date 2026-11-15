//! The browser surface against a REAL WebView2 (AMENAZAS W1, RF-77; D-9): the one test of the app that starts the
//! real runtime. Its window is never shown, loads `about:blank` and keeps its profile in a temporary folder.
//!
//! It proves what the mock runtime cannot: that the gate of the setup hook runs on the thread of the window,
//! that WebView2 takes both settings, and that they are still off after a reload. It does not prove what the
//! person sees (no menu, no keys): that is checked on the installed app.

#![cfg(windows)]
// Tests may panic on purpose: a failed expect is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::expect_used)]

use std::{sync::mpsc, time::Duration};

use almarioks_lib::{
    MAIN_WINDOW,
    webview::{BrowserFeatures, SurfaceError, close_browser_surface, ffi},
};
use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const ON: BrowserFeatures = BrowserFeatures {
    default_context_menus: true,
    browser_accelerator_keys: true,
};
const OFF: BrowserFeatures = BrowserFeatures {
    default_context_menus: false,
    browser_accelerator_keys: false,
};

/// What the webview says right now. The test is the thread of the window, so the answer is in on return.
fn features<R: Runtime>(window: &WebviewWindow<R>) -> BrowserFeatures {
    let (reply, answer) = mpsc::channel();
    window
        .with_webview(move |platform| {
            _ = reply.send(
                ffi::browser_features(&platform.controller()).map_err(|error| error.code().0),
            );
        })
        .expect("the request reaches the webview");
    answer
        .try_recv()
        .expect("the webview ran the request before returning")
        .expect("WebView2 answers its settings")
}

#[test]
fn rf_77_a_real_webview_starts_with_the_surface_closed_and_keeps_it_after_a_reload() {
    let profile =
        std::env::temp_dir().join(format!("almarioks-surface-test-{}", std::process::id()));
    let mut context = almarioks_lib::context::<tauri::Wry>();
    // The configured window would be visible and would use the profile of the installed app: the test makes
    // its own, at the same moment `run` finds the configured one (inside the setup hook, just created).
    for window in &mut context.config_mut().app.windows {
        window.create = false;
    }

    let (report, reported) = mpsc::channel::<(BrowserFeatures, Result<(), SurfaceError>)>();
    let profile_of_the_window = profile.clone();
    let mut app = tauri::Builder::default()
        .any_thread()
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(
                app,
                MAIN_WINDOW,
                WebviewUrl::External("about:blank".parse()?),
            )
            .visible(false)
            .incognito(true)
            .data_directory(profile_of_the_window)
            .build()?;
            let before = features(&window);
            report.send((before, close_browser_surface(app.handle())))?;
            Ok(())
        })
        .build(context)
        .expect("the real runtime starts");

    // The setup hook runs on the first iteration.
    #[allow(deprecated)] // `run_return` would not come back: nothing closes the hidden window.
    app.run_iteration(|_, _| {});
    let (before, gate) = reported.try_recv().expect("the setup hook ran");
    assert_eq!(before, ON, "WebView2 starts with both features on");
    assert_eq!(
        gate,
        Ok(()),
        "the gate of the setup hook passes on a real webview"
    );

    let window = app
        .get_webview_window(MAIN_WINDOW)
        .expect("the window of the test");
    assert_eq!(features(&window), OFF);

    // What a lock does: renew, then reload.
    almarioks_lib::reload_main_webview(app.handle());
    for _ in 0..25 {
        #[allow(deprecated)]
        app.run_iteration(|_, _| {});
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(features(&window), OFF, "the settings outlive the reload");

    drop(window);
    app.cleanup_before_exit();
    drop(app);
    // Best effort: WebView2 holds its files for an instant after the window is gone. The folder is in the
    // temporary directory and holds the profile of a private-mode `about:blank`.
    for _ in 0..20 {
        if std::fs::remove_dir_all(&profile).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
