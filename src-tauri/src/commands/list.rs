// The one list of commands the window may invoke (RF-79). This file is also `include!`d by `build.rs`, which hands
// it to `tauri_build::AppManifest::commands`: every name gets an `allow-<name>` permission (with `-` for `_`) and
// nothing outside the list can be granted. `capabilities/main-window.json` grants them to the `main` window, and
// `tests/commands_contract.rs` checks that the handler, this list and the capability say the same.

/// Commands of this build, in the order of the contract table of `docs/specs/001-boveda/plan.md`.
pub const COMMANDS: &[&str] = &[
    "recent_list",
    "recent_remove",
    "settings_get",
    "settings_set",
];
