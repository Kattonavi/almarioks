//! Opening the URL of an entry (RF-49): which texts may reach the browser, and that nothing else does.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use almarioks_core::CoreError;
use almarioks_lib::commands::{
    AppError, INTERNAL_CODE,
    url::{open_url, validate_url_to_open},
};
use common::FakeOpener;

/// What the system would receive for `raw`.
fn accepted(raw: &str) -> String {
    validate_url_to_open(raw)
        .unwrap_or_else(|error| panic!("`{raw}` should be allowed, got {error:?}"))
        .as_str()
        .to_owned()
}

fn refused(raw: &str) {
    assert_eq!(
        validate_url_to_open(raw).map(|url| url.as_str().to_owned()),
        Err(CoreError::UrlNoPermitida),
        "`{}` must not be opened",
        raw.escape_debug()
    );
}

#[test]
fn rf_49_http_and_https_are_opened() {
    assert_eq!(
        accepted("https://github.com/login"),
        "https://github.com/login"
    );
    assert_eq!(
        accepted("http://intranet.example/"),
        "http://intranet.example/"
    );
    assert_eq!(
        accepted("https://example.com:8443/a/b?q=1&r=dos#parte"),
        "https://example.com:8443/a/b?q=1&r=dos#parte"
    );
    assert_eq!(
        accepted("http://192.168.1.1/admin"),
        "http://192.168.1.1/admin"
    );
    assert_eq!(accepted("https://[::1]:8080/"), "https://[::1]:8080/");
    assert_eq!(accepted("https://localhost"), "https://localhost/");
}

#[test]
fn rf_49_the_scheme_is_accepted_in_any_letter_case() {
    for raw in [
        "HTTPS://github.com/login",
        "Https://github.com/login",
        "hTtPs://github.com/login",
    ] {
        assert_eq!(accepted(raw), "https://github.com/login");
    }
    assert_eq!(
        accepted("HTTP://example.com/Ruta"),
        "http://example.com/Ruta"
    );
}

#[test]
fn rf_49_the_url_is_trimmed_before_it_is_parsed() {
    // "Espacio" of the spec: every Unicode White_Space, not only the ASCII space.
    for raw in [
        "  https://github.com/login  ",
        "\thttps://github.com/login\r\n",
        "\u{00A0}https://github.com/login\u{2003}",
        "\u{3000}https://github.com/login\u{2028}",
    ] {
        assert_eq!(accepted(raw), "https://github.com/login");
    }
}

#[test]
fn rf_49_a_url_without_scheme_is_refused_and_nothing_is_put_in_front() {
    // Spec, edge cases: `github.com/login` is not opened; `https://` is never prepended (D13).
    for raw in [
        "github.com/login",
        "www.github.com",
        "//github.com/login",
        "/login",
        "login",
        "://github.com",
        "github.com:443/login",
        "localhost:8080",
        "",
        "   ",
        "\u{00A0}",
    ] {
        refused(raw);
    }

    let opener = FakeOpener::default();
    let error = open_url(&opener, "github.com/login").expect_err("no scheme");
    assert_eq!(error, AppError::from(CoreError::UrlNoPermitida));
    assert!(opener.opened().is_empty(), "nothing may be opened");
}

#[test]
fn rf_49_any_other_scheme_is_refused() {
    for raw in [
        "javascript:alert(1)",
        "JavaScript://example.com/%0Aalert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///C:/Windows/System32/calc.exe",
        "file://server/share/setup.exe",
        "ftp://example.com/archivo",
        "mailto:alguien@example.com",
        "ms-settings:privacy",
        "search-ms:query=x",
        "vbscript:msgbox(1)",
        "about:blank",
        "chrome://settings",
        "tauri://localhost/",
        "ws://example.com/",
        "httpss://example.com/",
        "http+unix://example.com/",
        "C:\\Windows\\System32\\calc.exe",
        "C:/Windows/System32/calc.exe",
        "\\\\server\\share\\setup.exe",
        "calc.exe",
    ] {
        refused(raw);
    }
}

#[test]
fn rf_49_text_that_only_a_lenient_parser_would_repair_is_refused() {
    // Decisión pendiente (conservative): a browser would turn each of these into an https URL.
    for raw in [
        "https:github.com/login",
        "https:/github.com/login",
        "https:\\\\github.com\\login",
        "https//github.com/login",
        "https ://github.com/login",
        "ht\ttps://github.com/login",
        "https://git\nhub.com/login",
        "https://github.com/login\u{0000}",
        "java\nscript:alert(1)",
        "\u{0001}https://github.com/login",
        "https://",
        "https:///",
        "http://?q=1",
        "https://exa mple.com/",
    ] {
        refused(raw);
    }
}

#[test]
fn rf_49_what_is_opened_is_always_a_web_url_with_a_host() {
    for raw in [
        "https://github.com/login",
        "HTTP://EXAMPLE.COM",
        " https://example.com/a b?q=\"x\"&r=<y> ",
        "https://usuario:clave@example.com/",
        "https://example.com/ruta%20con%20espacios",
        "https://münchen.example/straße",
        "https://example.com/'comillas'`y`$(cosas)",
    ] {
        let opened = accepted(raw);
        assert!(
            opened.starts_with("http://") || opened.starts_with("https://"),
            "{opened}"
        );
        // What reaches the system has no spaces, quotes or angle brackets left to be read as something else.
        assert!(
            !opened.contains([' ', '"', '<', '>']),
            "`{opened}` still has characters a shell could misread"
        );
        assert!(!opened.chars().any(char::is_control));
    }
}

#[test]
fn rf_49_open_url_hands_the_system_only_the_validated_url() {
    let opener = FakeOpener::default();

    open_url(&opener, "  HTTPS://GitHub.com/login  ").expect("a web URL is opened");
    for raw in ["javascript:alert(1)", "file:///C:/x.exe", "github.com", ""] {
        let error = open_url(&opener, raw).expect_err("not a web URL");
        assert_eq!(error.code(), "UrlNoPermitida");
    }

    assert_eq!(opener.opened(), ["https://github.com/login"]);
}

#[test]
fn rf_49_a_browser_that_does_not_open_is_an_internal_error_not_a_refused_url() {
    let opener = FakeOpener::default();
    opener.will_fail();

    let error = open_url(&opener, "https://github.com/login").expect_err("the system refused");

    assert_eq!(error.code(), INTERNAL_CODE);
    assert!(opener.opened().is_empty());
}

#[test]
fn rf_49_the_debug_text_of_a_validated_url_does_not_show_it() {
    // The URL field can be protected in a vault another app wrote (spec, "Campo protegido").
    let url = validate_url_to_open("https://secreto.example/token-CANARIO").unwrap();
    assert!(!format!("{url:?}").contains("CANARIO"));
}
