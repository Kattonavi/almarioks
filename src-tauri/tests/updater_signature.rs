//! H0 requirement [B] (AMENAZAS U1, ADR-0003): the real `tauri-plugin-updater` rejects bad signatures.
//!
//! The plugin runs on Tauri's mock runtime and talks to a tiny HTTP server owned by each test
//! (127.0.0.1, ephemeral port). Keys are throwaway test keys generated here the way the Tauri CLI 2.12
//! does it (minisign 0.9, base64 of the key/signature box). Nothing is ever installed: only `check()`
//! and `download()`, which verifies the signature before returning the bytes.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::{BufRead, BufReader, Cursor, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
    thread,
    time::Duration,
};

use base64::Engine;
use minisign::KeyPair;
use tauri::{
    App,
    test::{MockRuntime, mock_builder, mock_context, noop_assets},
};
use tauri_plugin_updater::{Error, Update, UpdaterExt};

/// Version the mock app reports, same as the real app's first release.
const CURRENT_VERSION: &str = "0.0.1";
const ARTIFACT: &[u8] = b"pretend this is AlmarioKS_0.0.2_x64-setup.exe";

struct TestKey {
    keypair: KeyPair,
}

impl TestKey {
    fn generate() -> Self {
        Self {
            keypair: KeyPair::generate_unencrypted_keypair().expect("generate test key pair"),
        }
    }

    /// `pubkey` as it goes in tauri.conf.json: base64 of the minisign public key box.
    fn pubkey(&self) -> String {
        let public_box = self
            .keypair
            .pk
            .to_box()
            .expect("public key box")
            .to_string();
        base64::engine::general_purpose::STANDARD.encode(public_box)
    }

    /// `signature` as it goes in latest.json: base64 of the `.sig` box the CLI writes, with the
    /// version the CLI 2.12 records in the trusted comment.
    fn sign(&self, data: &[u8]) -> String {
        self.sign_with_comment(
            data,
            "timestamp:0\tfile:AlmarioKS_0.0.2_x64-setup.exe\tversion:0.0.2",
        )
    }

    fn sign_with_comment(&self, data: &[u8], trusted_comment: &str) -> String {
        let signature = minisign::sign(
            None,
            &self.keypair.sk,
            Cursor::new(data),
            Some(trusted_comment),
            Some("signature from tauri secret key"),
        )
        .expect("sign test artifact");
        base64::engine::general_purpose::STANDARD.encode(signature.to_string())
    }
}

/// Serves `/latest.json` and `/artifact` until the test process ends. Returns the base URL.
fn serve(manifest: impl Fn(&str) -> String + Send + Sync + 'static, artifact: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let addr: SocketAddr = listener.local_addr().expect("local address");
    let base = format!("http://{addr}");
    let base_for_thread = base.clone();
    let manifest = Arc::new(manifest);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request_line = String::new();
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            // Drain the headers.
            let mut header = String::new();
            while reader.read_line(&mut header).is_ok() && header != "\r\n" && !header.is_empty() {
                header.clear();
            }
            let path = request_line.split_whitespace().nth(1).unwrap_or("/");
            let (status, content_type, body) = match path {
                "/latest.json" => (
                    "200 OK",
                    "application/json",
                    manifest(&base_for_thread).into_bytes(),
                ),
                "/artifact" => ("200 OK", "application/octet-stream", artifact.clone()),
                _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });
    base
}

/// Dynamic-format update manifest pointing at the local artifact.
fn manifest(version: &str, signature: String) -> impl Fn(&str) -> String + Send + Sync + 'static {
    let version = version.to_owned();
    move |base| {
        serde_json::json!({
            "version": version,
            "notes": "Prueba",
            "pub_date": "2026-11-11T00:00:00Z",
            "url": format!("{base}/artifact"),
            "signature": signature,
        })
        .to_string()
    }
}

/// Mock app with the real updater plugin. `allow_http` only lifts the transport check so the test can
/// use plain HTTP on 127.0.0.1; the signature check does not depend on it.
fn app(pubkey: &str, allow_http: bool) -> App<MockRuntime> {
    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = CURRENT_VERSION.parse().expect("semver");
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey": pubkey,
            "dangerousInsecureTransportProtocol": allow_http,
            // Same as src-tauri/tauri.conf.json (checked by tests/updater_config.rs).
            "requireSignedVersion": true,
        }),
    );
    mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .expect("build mock app with the updater plugin")
}

fn check(app: &App<MockRuntime>, base: &str) -> Option<Update> {
    let updater = app
        .updater_builder()
        .endpoints(vec![format!("{base}/latest.json").parse().expect("url")])
        .expect("endpoints")
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .build()
        .expect("build updater");
    tauri::async_runtime::block_on(updater.check()).expect("check for updates")
}

fn download(update: &Update) -> Result<Vec<u8>, Error> {
    tauri::async_runtime::block_on(update.download(|_, _| {}, || {}))
}

fn assert_signature_error(result: Result<Vec<u8>, Error>, expected: &str) {
    match result {
        Err(Error::Minisign(error)) => assert!(
            error.to_string().contains(expected),
            "unexpected minisign error: {error}"
        ),
        Err(other) => panic!("expected a signature error, got: {other}"),
        Ok(bytes) => panic!("a bad signature was accepted ({} bytes)", bytes.len()),
    }
}

#[test]
fn rf_h0_valid_signature_is_accepted() {
    let key = TestKey::generate();
    let base = serve(manifest("0.0.2", key.sign(ARTIFACT)), ARTIFACT.to_vec());
    let app = app(&key.pubkey(), true);

    let update = check(&app, &base).expect("0.0.2 is offered to 0.0.1");
    assert_eq!(update.version, "0.0.2");
    assert_eq!(update.current_version, CURRENT_VERSION);

    let bytes = download(&update).expect("signed artifact verifies");
    assert_eq!(bytes, ARTIFACT);
}

#[test]
fn rf_h0_tampered_artifact_is_rejected() {
    let key = TestKey::generate();
    let mut tampered = ARTIFACT.to_vec();
    if let Some(first) = tampered.first_mut() {
        *first ^= 0x01;
    }
    let base = serve(manifest("0.0.2", key.sign(ARTIFACT)), tampered);
    let app = app(&key.pubkey(), true);

    let update = check(&app, &base).expect("0.0.2 is offered");
    assert_signature_error(download(&update), "signature verification failed");
}

#[test]
fn rf_h0_signature_from_another_key_is_rejected() {
    let trusted = TestKey::generate();
    let attacker = TestKey::generate();
    let base = serve(
        manifest("0.0.2", attacker.sign(ARTIFACT)),
        ARTIFACT.to_vec(),
    );
    let app = app(&trusted.pubkey(), true);

    let update = check(&app, &base).expect("0.0.2 is offered");
    assert_signature_error(download(&update), "different key");
}

#[test]
fn rf_h0_same_or_older_version_is_not_offered() {
    let key = TestKey::generate();
    for version in [CURRENT_VERSION, "0.0.0"] {
        let base = serve(manifest(version, key.sign(ARTIFACT)), ARTIFACT.to_vec());
        let app = app(&key.pubkey(), true);
        assert!(
            check(&app, &base).is_none(),
            "version {version} must not be offered to {CURRENT_VERSION}"
        );
    }
}

/// With the production setting (no `dangerousInsecureTransportProtocol`), an `http://` endpoint is an
/// error in release builds. Debug builds only print a warning (plugin `config.rs`, `validate_endpoints`),
/// so this asserts both sides. The PR CI runs the debug side; the release side runs in `release.yml`
/// with `cargo test --release` (locally: `cargo test -p almarioks --test updater_signature --release`).
#[test]
fn rf_h0_http_endpoint_is_rejected_in_release() {
    let key = TestKey::generate();
    let app = app(&key.pubkey(), false);
    let result = app
        .updater_builder()
        .endpoints(vec!["http://127.0.0.1:9/latest.json".parse().expect("url")]);
    if cfg!(debug_assertions) {
        assert!(result.is_ok(), "debug builds only warn about http");
    } else {
        assert!(
            matches!(result, Err(Error::InsecureTransportProtocol)),
            "release builds must reject http endpoints"
        );
    }
}

#[test]
fn rf_h0_signature_without_version_is_rejected() {
    let key = TestKey::generate();
    // A valid signature from the right key, but made without the version (pre-2.12 CLI style).
    let signature =
        key.sign_with_comment(ARTIFACT, "timestamp:0\tfile:AlmarioKS_0.0.2_x64-setup.exe");
    let base = serve(manifest("0.0.2", signature), ARTIFACT.to_vec());
    let app = app(&key.pubkey(), true);

    let update = check(&app, &base).expect("0.0.2 is offered");
    match download(&update) {
        Err(Error::MissingSignedVersion) => {}
        Err(other) => panic!("expected MissingSignedVersion, got: {other}"),
        Ok(_) => panic!("a signature without version was accepted"),
    }
}

#[test]
fn rf_h0_signature_for_another_version_is_rejected() {
    let key = TestKey::generate();
    // Genuine artifact and key, but signed as 0.0.3 while the manifest announces 0.0.2
    // (a replayed or relabeled release).
    let signature = key.sign_with_comment(
        ARTIFACT,
        "timestamp:0\tfile:AlmarioKS_0.0.3_x64-setup.exe\tversion:0.0.3",
    );
    let base = serve(manifest("0.0.2", signature), ARTIFACT.to_vec());
    let app = app(&key.pubkey(), true);

    let update = check(&app, &base).expect("0.0.2 is offered");
    match download(&update) {
        Err(Error::SignedVersionMismatch { signed, announced }) => {
            assert_eq!((signed.as_str(), announced.as_str()), ("0.0.3", "0.0.2"));
        }
        Err(other) => panic!("expected SignedVersionMismatch, got: {other}"),
        Ok(_) => panic!("a signature for another version was accepted"),
    }
}
