//! The WebView2 settings behind [`super::close_browser_surface`]: the second and last module with `unsafe`.
//!
//! Tauri 2.12 does not expose these two settings (neither in `tauri.conf.json` nor in its window builder), although
//! `wry` has them. The only way left is the COM interface of the webview, which `Webview::with_webview` hands
//! over on the thread of the window:
//! - `ICoreWebView2Settings::put_AreDefaultContextMenusEnabled(FALSE)`: no default context menu ("Save as",
//!   "Print", "Refresh", "Back", "Import passwords"...).
//! - `ICoreWebView2Settings3::put_AreBrowserAcceleratorKeysEnabled(FALSE)`: no accelerator keys "that access
//!   features specific to a web browser" (find on page, print, reload, zoom, developer tools, back and forward).
//!   Movement and text editing keys stay: Home, End, Ctrl+X, Ctrl+C, Ctrl+V, Ctrl+A, Ctrl+Z.
//!
//! Microsoft's reference says that a change made after `NavigationStarting` applies from the next top-level
//! navigation; the settings object lives as long as the webview, so a reload keeps them.

// One of the two files of the workspace allowed to use `unsafe` (workspace lint `unsafe_code = "deny"`,
// constitution rule 9), with `clipboard/ffi.rs`: WebView2 is a COM API. Every `unsafe` block says why it is sound.
#![allow(unsafe_code)]

use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Settings, ICoreWebView2Settings3,
};
use windows::core::{BOOL, Interface};

use super::BrowserFeatures;

/// Switches off the default context menus and the browser accelerator keys of the webview, and returns what the
/// webview says about both afterwards.
///
/// Takes the controller by reference: COM interfaces are not `Send`, so the only place that has one is the
/// thread of the window, inside `with_webview`. That is the thread WebView2 must be called from.
///
/// # Errors
/// The code WebView2 answered. A runtime too old to have `ICoreWebView2Settings3` is an error as well: the
/// accelerator keys could not be switched off.
pub fn switch_browser_features_off(
    controller: &ICoreWebView2Controller,
) -> windows::core::Result<BrowserFeatures> {
    let (settings, keys) = settings_of(controller)?;
    // SAFETY: `settings` and `keys` are live COM interfaces of the webview of `controller`, called on the thread
    // that owns it (see the function's contract). Both calls only take a boolean by value.
    unsafe {
        settings.SetAreDefaultContextMenusEnabled(false)?;
        keys.SetAreBrowserAcceleratorKeysEnabled(false)?;
    }
    read(&settings, &keys)
}

/// What the webview says about its browser features right now. Read-only: for the check after a change and for
/// the tests.
///
/// # Errors
/// The code WebView2 answered.
pub fn browser_features(
    controller: &ICoreWebView2Controller,
) -> windows::core::Result<BrowserFeatures> {
    let (settings, keys) = settings_of(controller)?;
    read(&settings, &keys)
}

fn settings_of(
    controller: &ICoreWebView2Controller,
) -> windows::core::Result<(ICoreWebView2Settings, ICoreWebView2Settings3)> {
    // SAFETY: `controller` is a live COM interface, called on the thread that owns it. Both calls take no
    // arguments and return a new reference that the wrapper releases when dropped.
    let settings = unsafe { controller.CoreWebView2()?.Settings() }?;
    // `QueryInterface`, behind a safe wrapper. It fails on a runtime older than this interface.
    let keys = settings.cast::<ICoreWebView2Settings3>()?;
    Ok((settings, keys))
}

fn read(
    settings: &ICoreWebView2Settings,
    keys: &ICoreWebView2Settings3,
) -> windows::core::Result<BrowserFeatures> {
    // Start from "enabled": if a getter left its output untouched, the check after the change fails closed.
    let mut context_menus = BOOL::from(true);
    let mut accelerator_keys = BOOL::from(true);
    // SAFETY: both pointers come from live, writable locals of the size the interface expects (`BOOL`), and
    // WebView2 only writes to them during the call.
    unsafe {
        settings.AreDefaultContextMenusEnabled(&raw mut context_menus)?;
        keys.AreBrowserAcceleratorKeysEnabled(&raw mut accelerator_keys)?;
    }
    Ok(BrowserFeatures {
        default_context_menus: context_menus.as_bool(),
        browser_accelerator_keys: accelerator_keys.as_bool(),
    })
}
