//! Opening the URL of an entry in the browser (RF-49).
//!
//! A URL in a vault is untrusted text: another app or another person may have written it. Only `http` and
//! `https` are opened, nothing is added to what was written, and what reaches the system is the URL as the parser
//! understood it, never the raw text. `entry_open_url` (T13) reads the URL of the entry and calls [`open_url`].

use std::fmt;

use almarioks_core::CoreError;
use url::Url;

use super::error::AppError;

/// A URL that passed [`validate_url_to_open`]. The only thing a [`UrlOpener`] accepts.
#[derive(Clone, PartialEq, Eq)]
pub struct UrlToOpen(Url);

impl UrlToOpen {
    /// The URL as the parser wrote it back: it starts with `http://` or `https://` and has a host.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// The URL field can be marked as protected in a vault, so `Debug` does not show it.
impl fmt::Debug for UrlToOpen {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UrlToOpen(..)")
    }
}

/// Decides whether the URL of an entry may be opened (RF-49): trims it, parses it and accepts it only if its
/// scheme is `http` or `https`, in any letter case. Nothing is ever put in front of the text.
///
/// Decisión pendiente (the spec does not say; the conservative choice was taken): besides the scheme, the text
/// must read `scheme://host…` (`https:example.com` and `https:\\example.com`, which a browser would repair, are
/// refused) and must not contain control characters (a parser would silently drop a tab or a line break from
/// the middle of it).
///
/// # Errors
/// `UrlNoPermitida` if the text has no scheme, another scheme, no host, or does not parse.
pub fn validate_url_to_open(raw: &str) -> Result<UrlToOpen, CoreError> {
    let refused = CoreError::UrlNoPermitida;
    let text = raw.trim();
    if text.chars().any(char::is_control) {
        return Err(refused);
    }
    let (scheme, rest) = text.split_once(':').ok_or(refused)?;
    let web = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    if !web || !rest.starts_with("//") {
        return Err(refused);
    }
    let url = Url::parse(text).map_err(|_| refused)?;
    // The parser's verdict, not only ours: its scheme is what the system will see.
    let has_host = url.host_str().is_some_and(|host| !host.is_empty());
    if !matches!(url.scheme(), "http" | "https") || !has_host {
        return Err(refused);
    }
    Ok(UrlToOpen(url))
}

/// The system could not open the URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UrlOpenFailed;

/// Hands a validated URL to the system. Injected so the tests never start a browser.
pub trait UrlOpener: Send + Sync {
    /// Opens `url` with the program the system has for it.
    ///
    /// # Errors
    /// [`UrlOpenFailed`] if the system refused.
    fn open(&self, url: &UrlToOpen) -> Result<(), UrlOpenFailed>;
}

/// The default browser, through `tauri-plugin-opener` called from Rust. Only its `open_url` function is used:
/// the plugin itself is not registered in the app, so it adds no command and no script to the window.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &UrlToOpen) -> Result<(), UrlOpenFailed> {
        tauri_plugin_opener::open_url(url.as_str(), None::<&str>).map_err(|_| UrlOpenFailed)
    }
}

/// Validates `raw` and, only if it passes, opens it (RF-49). Blocking: call it from `spawn_blocking`.
///
/// # Errors
/// `UrlNoPermitida` if the URL may not be opened (nothing is opened); `ErrorInterno` if the system could not
/// open it.
pub fn open_url(opener: &dyn UrlOpener, raw: &str) -> Result<(), AppError> {
    let url = validate_url_to_open(raw)?;
    opener.open(&url).map_err(|_| AppError::internal())
}
