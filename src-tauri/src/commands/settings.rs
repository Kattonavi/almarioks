//! Settings and recent vaults (RF-19, RF-20, RF-29, RF-73, RF-82). The rules live in `almarioks_core::settings`;
//! here they only meet the file Tauri points to.

use std::path::Path;

use almarioks_core::settings::{RecentVault, Settings};
use serde::{Deserialize, Serialize};
use tauri::{State, ipc::Request};

use super::{
    args::{expect_no_args, parse_args},
    blocking,
    error::AppError,
};
use crate::state::{AppState, SettingsStore};

/// `settings_set{auto_lock_minutes, clipboard_seconds}`: both at once, all or nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsSetArgs {
    auto_lock_minutes: u32,
    clipboard_seconds: u32,
}

/// `recent_remove{path}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecentRemoveArgs {
    path: String,
}

/// The settings as the window reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SettingsView {
    auto_lock_minutes: u32,
    clipboard_seconds: u32,
}

impl From<&Settings> for SettingsView {
    fn from(settings: &Settings) -> Self {
        Self {
            auto_lock_minutes: settings.auto_lock_minutes(),
            clipboard_seconds: settings.clipboard_seconds(),
        }
    }
}

/// A recent vault and whether its file is still there (RF-20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecentView {
    path: String,
    exists: bool,
}

/// The recent vaults of `settings`, asking the disk whether each is still there. Blocking.
fn recent_views(settings: &Settings) -> Vec<RecentView> {
    settings
        .recent_with_status(Path::exists)
        .into_iter()
        .filter_map(|RecentVault { path, exists }| {
            // The core only stores paths that are valid text, so none is skipped here.
            path.to_str().map(|text| RecentView {
                path: text.to_owned(),
                exists,
            })
        })
        .collect()
}

fn remove_recent(store: &SettingsStore, path: &str) -> Result<Vec<RecentView>, AppError> {
    let settings = store.update(|settings| {
        settings.forget_vault(Path::new(path));
        Ok(())
    })?;
    Ok(recent_views(&settings))
}

/// The lock and clipboard times.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn settings_get(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<SettingsView, AppError> {
    expect_no_args(request.body())?;
    let store = state.settings();
    blocking(move || Ok(SettingsView::from(&store.read()))).await
}

/// Changes both times and writes the settings file.
///
/// # Errors
/// `ArgumentoInvalido` if the arguments are malformed, the lock time is not 1, 5, 15 or 30 minutes or the
/// clipboard time is not between 10 and 120 seconds (nothing changes); `GuardadoFallido` if the file cannot be
/// written (nothing changes).
#[tauri::command]
pub async fn settings_set(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<SettingsView, AppError> {
    let args: SettingsSetArgs = parse_args(request.body())?;
    let store = state.settings();
    blocking(move || {
        let settings = store.update(|settings| {
            settings.set_auto_lock_minutes(args.auto_lock_minutes)?;
            settings.set_clipboard_seconds(args.clipboard_seconds)
        })?;
        Ok(SettingsView::from(&settings))
    })
    .await
}

/// The last vaults opened, the most recent first, each with whether it is still at its path.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn recent_list(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<RecentView>, AppError> {
    expect_no_args(request.body())?;
    let store = state.settings();
    blocking(move || Ok(recent_views(&store.read()))).await
}

/// Takes a vault out of the recent list and answers the list that is left. A path that is not in the list
/// changes nothing.
///
/// # Errors
/// `ArgumentoInvalido` if the arguments are malformed or the path is empty; `GuardadoFallido` if the settings
/// file cannot be written (the list stays as it was).
#[tauri::command]
pub async fn recent_remove(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<RecentView>, AppError> {
    let args: RecentRemoveArgs = parse_args(request.body())?;
    if args.path.is_empty() {
        return Err(AppError::invalid_argument());
    }
    let store = state.settings();
    blocking(move || remove_recent(&store, &args.path)).await
}
