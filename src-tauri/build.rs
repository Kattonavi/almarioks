fn main() {
    // No app commands yet. Listing them here (empty for now) makes every future command need an explicit
    // `allow-<command>` permission in a capability, instead of being open to every window by default.
    // https://v2.tauri.app/security/capabilities/
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("tauri-build failed: {error:#}");
        std::process::exit(1);
    }
    embed_test_manifest();
}

/// tauri-build embeds the Common-Controls v6 manifest only in the app binary. Test binaries that link
/// `tauri` then load comctl32 v5 and abort at startup with 0xc0000139 (entry point not found).
/// Same manifest, linked into integration tests only.
fn embed_test_manifest() {
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if !windows_msvc {
        return;
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("windows-test-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
