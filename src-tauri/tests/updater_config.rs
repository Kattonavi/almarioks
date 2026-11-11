//! Guards the shipped updater configuration (ADR-0003, AMENAZAS U1): reads the REAL
//! `src-tauri/tauri.conf.json`, so a careless edit to the endpoint, key or safety flags fails CI.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;

const ENDPOINT: &str =
    "https://github.com/Kattonavi/almarioks/releases/latest/download/latest.json";
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDkwNzFFNjJCMEVBQTE1QTUKUldTbEZhb09LK1p4a0swVFlUNXphMTlXYldJSGlhOXlDSG5iWEhDRi9IUGcvT2xrSzQ0UnVKVXIK";

fn config() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = std::fs::read_to_string(&path).expect("read tauri.conf.json");
    serde_json::from_str(&text).expect("tauri.conf.json is valid JSON")
}

#[test]
fn rf_h0_updater_config_is_the_shipped_one() {
    let config = config();
    let updater = &config["plugins"]["updater"];

    let endpoints = updater["endpoints"].as_array().expect("endpoints array");
    assert_eq!(endpoints.len(), 1, "exactly one updater endpoint");
    assert_eq!(endpoints[0].as_str(), Some(ENDPOINT));

    assert!(
        matches!(
            updater.get("dangerousInsecureTransportProtocol"),
            None | Some(Value::Bool(false))
        ),
        "dangerousInsecureTransportProtocol must be absent or false"
    );
    assert_eq!(
        updater["requireSignedVersion"].as_bool(),
        Some(true),
        "the signature must carry the version"
    );
    assert_eq!(updater["windows"]["installMode"].as_str(), Some("passive"));
    assert_eq!(updater["pubkey"].as_str(), Some(PUBKEY));

    assert_eq!(
        config["bundle"]["createUpdaterArtifacts"].as_bool(),
        Some(true)
    );
}
