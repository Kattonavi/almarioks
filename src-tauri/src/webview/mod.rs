//! The browser surface of the window (AMENAZAS W1, RF-77; defects D-9, D-10 and D-11 of
//! `docs/specs/001-boveda/validacion-app.md`).
//!
//! WebView2 is a browser: left alone, its context menu offers "Save as" and "Print", and its accelerator keys
//! print, reload and search the page. With an entry on screen, "Save as" writes its fields to a file in clear.
//! This module switches both off in the webview itself; the page has a second, cheaper barrier of its own
//! (`src/ui/browserSurface.ts`).
//!
//! When:
//! - **At startup**, in the setup hook, before the event loop runs for the first time: nobody has used the
//!   window yet. If it fails, the app does not start ([`close_browser_surface`]).
//! - **Before the reload of every lock** (`lock.rs`), through the same queue as the reload
//!   ([`renew_browser_surface`]). The settings of a webview outlive its pages, so this only repeats what the
//!   startup proved; WebView2 documents that a change made before `NavigationStarting` applies to that navigation.
//!
//! Developer tools are untouched: they are compiled out of release builds.

use std::sync::mpsc::{self, Sender};

use tauri::{AppHandle, Manager, Runtime, Webview, WebviewWindow, webview::PlatformWebview};

use crate::MAIN_WINDOW;

#[cfg(windows)]
pub mod ffi;

/// The two browser features of the webview that this module switches off, as the webview reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserFeatures {
    /// The context menu of the browser: "Back", "Refresh", "Save as", "Print", "Import passwords"...
    pub default_context_menus: bool,
    /// The keys of the browser: reload, print, find on page, downloads, zoom, developer tools...
    pub browser_accelerator_keys: bool,
}

impl BrowserFeatures {
    /// Whether the surface is closed: both features off.
    ///
    /// # Errors
    /// [`SurfaceError::StillEnabled`] if the webview still reports one of them as enabled.
    pub const fn closed(self) -> Result<(), SurfaceError> {
        if self.default_context_menus || self.browser_accelerator_keys {
            Err(SurfaceError::StillEnabled)
        } else {
            Ok(())
        }
    }
}

/// Why the browser surface of the window could not be closed. Carries no content of the page: at most the
/// numeric code the system answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SurfaceError {
    /// The main window does not exist.
    #[error("the main window does not exist")]
    NoWindow,
    /// The webview never ran the request: there is no real webview behind the window, or the call did not come
    /// from the thread of the window.
    #[error("the webview did not run the request to switch its browser features off")]
    NotApplied,
    /// WebView2 answered an error, for example a runtime too old to switch the accelerator keys off.
    #[error("the webview refused to switch its browser features off (code {0:#010x})")]
    Refused(i32),
    /// WebView2 accepted the change and still reports a feature as enabled.
    #[error("the webview still reports a browser feature as enabled")]
    StillEnabled,
    /// This system has no implementation: the app is only built for Windows.
    #[error("switching the browser features off is not implemented on this system")]
    Unsupported,
}

/// How closing the surface went.
pub type Outcome = Result<(), SurfaceError>;

/// Switches the browser features of the main window off and proves it happened (D-9). For the setup hook: the
/// window already exists there and the hook runs on its thread, so the answer is in before this returns.
///
/// # Errors
/// Any [`SurfaceError`]. The caller must not let the app go on: the window would offer "Save as" and "Print".
pub fn close_browser_surface<R: Runtime>(app: &AppHandle<R>) -> Outcome {
    let window = app
        .get_webview_window(MAIN_WINDOW)
        .ok_or(SurfaceError::NoWindow)?;
    answered_now(|reply| request(window.as_ref(), reply))
}

/// Switches the browser features of `window` off again, right before a reload. Never blocks: off the thread of
/// the window the request is only queued, ahead of the reload that follows it.
///
/// Best effort, and it never stops the reload. The settings of a webview outlive its pages, so a failure here
/// leaves what the startup already proved; and the reload is what makes the page forget the vault (RF-32), which
/// matters more.
pub fn renew_browser_surface<R: Runtime>(window: &WebviewWindow<R>) {
    // Nobody waits for this answer: the lock must not.
    let (reply, _nobody) = mpsc::channel();
    _ = request(window.as_ref(), reply);
}

/// Asks `webview` to switch its browser features off on the thread of its window, and to say how it went on
/// `reply`. From that thread Tauri runs it before returning; from any other it only queues it. Never blocks.
fn request<R: Runtime>(webview: &Webview<R>, reply: Sender<Outcome>) -> Outcome {
    webview
        .with_webview(move |platform| {
            // A renewal does not listen: a closed channel is not an error.
            _ = reply.send(switch_off(&platform));
        })
        .map_err(|_| SurfaceError::NotApplied)
}

#[cfg(windows)]
fn switch_off(platform: &PlatformWebview) -> Outcome {
    ffi::switch_browser_features_off(&platform.controller())
        .map_err(|error| SurfaceError::Refused(error.code().0))?
        .closed()
}

/// Only Windows has an implementation (`ffi`). Failing keeps a build for another system from starting with the
/// browser surface open.
#[cfg(not(windows))]
fn switch_off(_platform: &PlatformWebview) -> Outcome {
    Err(SurfaceError::Unsupported)
}

/// Runs `ask` and takes the answer it must have left on its channel by the time it returns. No answer is a
/// failure, never a wait: an answer that is "on its way" proves nothing about the window the person already sees.
///
/// Public so its tests can live in `tests/webview_surface.rs`: the unit tests of this crate cannot start on
/// Windows (`build.rs`, `embed_test_manifest`).
///
/// # Errors
/// What `ask` or the answer says, or [`SurfaceError::NotApplied`] with no answer.
pub fn answered_now(ask: impl FnOnce(Sender<Outcome>) -> Outcome) -> Outcome {
    let (reply, answer) = mpsc::channel();
    ask(reply)?;
    answer.try_recv().unwrap_or(Err(SurfaceError::NotApplied))
}
