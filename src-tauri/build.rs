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
}
