//! Verifies a release artifact exactly as the AlmarioKS updater will at runtime, before it is published:
//! the `.sig` (base64 of the minisign signature box, as written by the Tauri CLI) must verify against the
//! `plugins.updater.pubkey` of the given `tauri.conf.json`, and its trusted comment must carry
//! `version:<version>`. No Tauri, no network.

#![forbid(unsafe_code)]

use std::fmt;

use base64::Engine;
use minisign_verify::{PublicKey, Signature};

/// Why a release does not pass.
#[derive(Debug)]
pub enum VerifyError {
    /// `tauri.conf.json` is not JSON or has no `plugins.updater.pubkey` string.
    Config(String),
    /// The pubkey or the signature is not valid base64 / UTF-8 / minisign.
    Encoding(String),
    /// The signature does not match the artifact or was made with another key.
    Signature(String),
    /// The trusted comment carries no `version:` field.
    MissingVersion,
    /// The trusted comment carries another version.
    VersionMismatch { signed: String, expected: String },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(detail) => write!(f, "invalid tauri.conf.json: {detail}"),
            Self::Encoding(detail) => write!(f, "invalid key or signature encoding: {detail}"),
            Self::Signature(detail) => write!(
                f,
                "signature does not verify against the pubkey in tauri.conf.json: {detail}"
            ),
            Self::MissingVersion => write!(
                f,
                "the signature carries no version (trusted comment without `version:`)"
            ),
            Self::VersionMismatch { signed, expected } => write!(
                f,
                "the signature was made for version {signed}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for VerifyError {}

fn decode_base64_text(what: &str, value: &str) -> Result<String, VerifyError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|error| VerifyError::Encoding(format!("{what} is not base64: {error}")))?;
    String::from_utf8(bytes)
        .map_err(|error| VerifyError::Encoding(format!("{what} is not UTF-8: {error}")))
}

/// Reads `plugins.updater.pubkey` from the text of a `tauri.conf.json`.
pub fn pubkey_from_config(config_json: &str) -> Result<String, VerifyError> {
    let config: serde_json::Value = serde_json::from_str(config_json)
        .map_err(|error| VerifyError::Config(error.to_string()))?;
    config
        .pointer("/plugins/updater/pubkey")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| VerifyError::Config("missing plugins.updater.pubkey".into()))
}

/// The `version` field of a trusted comment written by the Tauri CLI
/// (`timestamp:…\tfile:…\tversion:…`).
fn signed_version(trusted_comment: &str) -> Option<&str> {
    trusted_comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
}

/// Checks `artifact` against `signature_b64` (content of the `.sig` file) with the base64 `pubkey`
/// of `tauri.conf.json`, and that the signature was made for exactly `version`.
pub fn verify_release(
    pubkey_b64: &str,
    artifact: &[u8],
    signature_b64: &str,
    version: &str,
) -> Result<(), VerifyError> {
    let public_key = PublicKey::decode(&decode_base64_text("pubkey", pubkey_b64)?)
        .map_err(|error| VerifyError::Encoding(format!("pubkey: {error}")))?;
    let signature = Signature::decode(&decode_base64_text("signature", signature_b64)?)
        .map_err(|error| VerifyError::Encoding(format!("signature: {error}")))?;

    // Same call as the updater plugin (legacy signatures allowed, as at runtime).
    public_key
        .verify(artifact, &signature, true)
        .map_err(|error| VerifyError::Signature(error.to_string()))?;

    // The trusted comment is covered by the global signature: only read it after `verify`.
    match signed_version(signature.trusted_comment()) {
        None => Err(VerifyError::MissingVersion),
        Some(signed) if signed == version => Ok(()),
        Some(signed) => Err(VerifyError::VersionMismatch {
            signed: signed.to_owned(),
            expected: version.to_owned(),
        }),
    }
}
