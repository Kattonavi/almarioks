//! The browser surface of the window (AMENAZAS W1, RF-77; D-9 of `docs/specs/001-boveda/validacion-app.md`): the
//! decisions around the WebView2 settings, without a real webview.
//!
//! What these tests cannot see is the webview itself: that the context menu and the keys of the browser are
//! gone is checked on the installed app, once per release.

// Tests may panic on purpose: a failed expect is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::expect_used)]

use almarioks_lib::{
    MAIN_WINDOW,
    webview::{
        BrowserFeatures, SurfaceError, answered_now, close_browser_surface, renew_browser_surface,
    },
};
use tauri::{WebviewUrl, WebviewWindowBuilder, test::mock_app};

const OFF: BrowserFeatures = BrowserFeatures {
    default_context_menus: false,
    browser_accelerator_keys: false,
};

/// `E_NOINTERFACE`: what a runtime too old to switch the accelerator keys off answers.
const E_NOINTERFACE: i32 = -2_147_467_262;

#[test]
fn rf_77_the_surface_is_closed_only_with_both_browser_features_off() {
    assert_eq!(OFF.closed(), Ok(()));
    for open in [
        BrowserFeatures {
            default_context_menus: true,
            ..OFF
        },
        BrowserFeatures {
            browser_accelerator_keys: true,
            ..OFF
        },
        BrowserFeatures {
            default_context_menus: true,
            browser_accelerator_keys: true,
        },
    ] {
        assert_eq!(open.closed(), Err(SurfaceError::StillEnabled), "{open:?}");
    }
}

#[test]
fn rf_77_the_gate_passes_only_with_an_answer_that_is_already_in() {
    assert_eq!(
        answered_now(|reply| {
            reply.send(Ok(())).expect("the gate is listening");
            Ok(())
        }),
        Ok(())
    );
}

#[test]
fn rf_77_the_gate_reports_what_the_webview_answered() {
    for refusal in [
        SurfaceError::Refused(E_NOINTERFACE),
        SurfaceError::StillEnabled,
        SurfaceError::Unsupported,
    ] {
        assert_eq!(
            answered_now(|reply| {
                reply.send(Err(refusal)).expect("the gate is listening");
                Ok(())
            }),
            Err(refusal)
        );
    }
}

#[test]
fn rf_77_the_gate_fails_when_the_request_was_dropped_without_running() {
    assert_eq!(answered_now(|_reply| Ok(())), Err(SurfaceError::NotApplied));
}

#[test]
fn rf_77_the_gate_fails_when_the_request_is_only_queued() {
    // What happens off the thread of the window: the request is alive, and nobody has run it yet.
    let mut queued = None;
    assert_eq!(
        answered_now(|reply| {
            queued = Some(reply);
            Ok(())
        }),
        Err(SurfaceError::NotApplied)
    );
    assert!(queued.is_some());
}

#[test]
fn rf_77_the_gate_fails_when_the_request_could_not_be_sent() {
    assert_eq!(
        answered_now(|_reply| Err(SurfaceError::NoWindow)),
        Err(SurfaceError::NoWindow)
    );
}

#[test]
fn rf_77_without_a_main_window_the_surface_is_not_closed() {
    let app = mock_app();
    assert_eq!(
        close_browser_surface(app.handle()),
        Err(SurfaceError::NoWindow)
    );
}

#[test]
fn rf_77_a_window_with_no_real_webview_does_not_pass_for_closed() {
    // The runtime of the tests accepts the request and never runs it: that must not read as "done".
    let app = mock_app();
    WebviewWindowBuilder::new(&app, MAIN_WINDOW, WebviewUrl::default())
        .build()
        .expect("a window on the mock runtime");
    assert_eq!(
        close_browser_surface(app.handle()),
        Err(SurfaceError::NotApplied)
    );
}

#[test]
fn rf_77_an_error_of_the_surface_carries_at_most_a_code() {
    assert_eq!(
        SurfaceError::Refused(E_NOINTERFACE).to_string(),
        "the webview refused to switch its browser features off (code 0x80004002)"
    );
}

#[test]
fn rf_77_the_real_app_closes_the_surface_first_and_again_before_every_reload() {
    // `run` cannot be called from a test, and the runtime of the tests reloads nothing: the source is what pins
    // that the gate is wired, before the state (and with it the commands) exists, and that the reload of a lock
    // renews the settings before it reloads.
    let lib = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src/lib.rs");

    let run = lib
        .split_once("pub fn run()")
        .map(|(_, body)| body)
        .expect("`run` exists");
    let gate = run.find("webview::close_browser_surface(app.handle())?;");
    let install = run.find("install(app.handle()");
    assert!(
        gate.is_some(),
        "the setup hook closes the surface, and its error stops the start"
    );
    assert!(gate < install, "the gate comes first in the setup hook");

    let reload = lib
        .split_once("pub fn reload_main_webview")
        .map(|(_, body)| body)
        .expect("`reload_main_webview` exists");
    let renew = reload.find("webview::renew_browser_surface(&window);");
    let reloads = reload.find("window.reload()");
    assert!(renew.is_some(), "the reload of a lock renews the settings");
    assert!(
        renew < reloads,
        "the settings go before the reload they must apply to"
    );
}

#[test]
fn rf_77_renewing_the_surface_never_blocks_nor_fails_without_a_real_webview() {
    let app = mock_app();
    let window = WebviewWindowBuilder::new(&app, MAIN_WINDOW, WebviewUrl::default())
        .build()
        .expect("a window on the mock runtime");
    renew_browser_surface(&window);
    // And the reload that follows it in the app goes on as before.
    almarioks_lib::reload_main_webview(app.handle());
}
