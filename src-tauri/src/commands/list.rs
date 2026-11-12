// The one list of commands the window may invoke (RF-79). This file is also `include!`d by `build.rs`, which hands
// it to `tauri_build::AppManifest::commands`: every name gets an `allow-<name>` permission (with `-` for `_`) and
// nothing outside the list can be granted. `capabilities/main-window.json` grants them to the `main` window, and
// `tests/commands_contract.rs` checks that the handler, this list and the capability say the same.

/// Commands of this build, in the order of the contract table of `docs/specs/001-boveda/plan.md`.
pub const COMMANDS: &[&str] = &[
    "vault_status",
    "vault_create",
    "vault_open",
    "vault_lock",
    "vault_close",
    "vault_convert",
    "vault_retry_save",
    "vault_save_as",
    "vault_reload",
    "pick_vault_to_open",
    "pick_vault_to_create",
    "pick_key_file",
    "pick_new_key_file",
    "recent_list",
    "recent_remove",
    "password_strength",
    "group_tree",
    "group_create",
    "group_rename",
    "group_move",
    "group_delete_preview",
    "group_delete",
    "recycle_bin_empty",
    "entry_list",
    "entry_get",
    "entry_create",
    "entry_update",
    "entry_move",
    "entry_delete",
    "entry_reveal",
    "entry_copy",
    "entry_open_url",
    "search",
    "generate_password",
    "settings_get",
    "settings_set",
    "activity",
    // Not a row of that table yet: the window calls it right before it installs an update (review 0.1, C-1).
    "app_prepare_update",
    // Not a row of that table yet either: where a new vault is proposed to go, and its key file (RF-97, RF-99).
    "vault_create_location",
];
