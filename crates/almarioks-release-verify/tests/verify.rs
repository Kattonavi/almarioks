//! Runs the real `almarioks-release-verify` binary with throwaway minisign keys made like the Tauri
//! CLI 2.12 makes them (base64 of the key and signature boxes, `version:` in the trusted comment).

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use base64::Engine;
use minisign::KeyPair;

const ARTIFACT: &[u8] = b"pretend this is AlmarioKS_0.0.2_x64-setup.exe";
const SIGNED_FOR_002: &str = "timestamp:0\tfile:AlmarioKS_0.0.2_x64-setup.exe\tversion:0.0.2";

fn b64(text: String) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

fn keypair() -> KeyPair {
    KeyPair::generate_unencrypted_keypair().expect("generate test key pair")
}

fn pubkey(keys: &KeyPair) -> String {
    b64(keys.pk.to_box().expect("public key box").to_string())
}

fn sign(keys: &KeyPair, data: &[u8], trusted_comment: &str) -> String {
    let signature = minisign::sign(
        None,
        &keys.sk,
        Cursor::new(data),
        Some(trusted_comment),
        Some("signature from tauri secret key"),
    )
    .expect("sign test artifact");
    b64(signature.to_string())
}

/// Writes the three input files in a fresh directory and runs the verifier on them.
fn run(name: &str, pubkey: &str, artifact: &[u8], signature: &str, version: &str) -> Output {
    let dir: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).expect("create test dir");
    let config = dir.join("tauri.conf.json");
    let installer = dir.join("AlmarioKS_0.0.2_x64-setup.exe");
    let sig = dir.join("AlmarioKS_0.0.2_x64-setup.exe.sig");
    let config_json = serde_json::json!({ "plugins": { "updater": { "pubkey": pubkey } } });
    std::fs::write(&config, config_json.to_string()).expect("write config");
    std::fs::write(&installer, artifact).expect("write installer");
    std::fs::write(&sig, signature).expect("write signature");
    Command::new(env!("CARGO_BIN_EXE_almarioks-release-verify"))
        .args([config.as_os_str(), installer.as_os_str(), sig.as_os_str()])
        .arg(version)
        .output()
        .expect("run almarioks-release-verify")
}

fn assert_fails(output: &Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "must fail, stderr: {stderr}");
    assert!(
        stderr.contains(expected),
        "stderr should mention {expected:?}: {stderr}"
    );
}

#[test]
fn good_signature_for_the_right_version_passes() {
    let keys = keypair();
    let output = run(
        "good",
        &pubkey(&keys),
        ARTIFACT,
        &sign(&keys, ARTIFACT, SIGNED_FOR_002),
        "0.0.2",
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("OK:"));
}

#[test]
fn signature_from_another_key_fails() {
    let baked = keypair();
    let other = keypair();
    let output = run(
        "other-key",
        &pubkey(&baked),
        ARTIFACT,
        &sign(&other, ARTIFACT, SIGNED_FOR_002),
        "0.0.2",
    );
    assert_fails(&output, "does not verify");
}

#[test]
fn tampered_installer_fails() {
    let keys = keypair();
    let signature = sign(&keys, ARTIFACT, SIGNED_FOR_002);
    let mut tampered = ARTIFACT.to_vec();
    if let Some(last) = tampered.last_mut() {
        *last ^= 0x01;
    }
    let output = run("tampered", &pubkey(&keys), &tampered, &signature, "0.0.2");
    assert_fails(&output, "does not verify");
}

#[test]
fn signature_without_version_fails() {
    let keys = keypair();
    let signature = sign(
        &keys,
        ARTIFACT,
        "timestamp:0\tfile:AlmarioKS_0.0.2_x64-setup.exe",
    );
    let output = run("no-version", &pubkey(&keys), ARTIFACT, &signature, "0.0.2");
    assert_fails(&output, "carries no version");
}

#[test]
fn signature_for_another_version_fails() {
    let keys = keypair();
    let signature = sign(&keys, ARTIFACT, SIGNED_FOR_002);
    let output = run(
        "other-version",
        &pubkey(&keys),
        ARTIFACT,
        &signature,
        "0.0.3",
    );
    assert_fails(&output, "made for version 0.0.2, expected 0.0.3");
}

#[test]
fn missing_arguments_print_usage_and_fail() {
    let output = Command::new(env!("CARGO_BIN_EXE_almarioks-release-verify"))
        .output()
        .expect("run almarioks-release-verify");
    assert_fails(&output, "usage:");
}
