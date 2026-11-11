//! `almarioks-release-verify <tauri.conf.json> <installer> <installer.sig> <version>`
//!
//! Exit 0 only if the installer signature verifies against the pubkey of that tauri.conf.json and was
//! made for exactly `<version>`. `release.yml` runs it before publishing.

use std::{fs, process::ExitCode};

use almarioks_release_verify::{pubkey_from_config, verify_release};

const USAGE: &str =
    "usage: almarioks-release-verify <tauri.conf.json> <installer> <installer.sig> <version>";

fn run(args: &[String]) -> Result<String, String> {
    let [config_path, artifact_path, signature_path, version] = args else {
        return Err(USAGE.to_owned());
    };
    let read_text = |path: &String| {
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
    };
    let config = read_text(config_path)?;
    let signature = read_text(signature_path)?;
    let artifact =
        fs::read(artifact_path).map_err(|error| format!("cannot read {artifact_path}: {error}"))?;

    let pubkey = pubkey_from_config(&config).map_err(|error| error.to_string())?;
    verify_release(&pubkey, &artifact, &signature, version)
        .map_err(|error| format!("{artifact_path}: {error}"))?;
    Ok(format!(
        "OK: {artifact_path} is signed for version {version} with the pubkey of {config_path}"
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("FAIL: {message}");
            ExitCode::FAILURE
        }
    }
}
