//! Groups (spec 001, section F: RF-50 to RF-58). The rules live in the core; here each command reads its
//! arguments, takes its turn in the session and answers the tree as the window draws it.
//!
//! A change that changes nothing (renaming to the same name, moving to the same parent, emptying an empty bin)
//! is told to the session as such, so it is not saved: a save would replace the `.bak` with an identical copy.

use almarioks_core::{
    session::Applied,
    vault::{DeletePreview, GroupId, GroupKind, GroupNode, validate_group_name},
};
use serde::{Deserialize, Serialize};
use tauri::{State, ipc::Request};

use super::{
    args::{expect_no_args, parse_args},
    blocking,
    error::AppError,
    require_open, with_field_data,
};
use crate::state::AppState;

/// A node of the tree: `{ id, name, entry_count, kind, children }` (plan, "Contrato").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupView {
    id: String,
    name: String,
    /// Entries directly in the group (plan, "Precisiones" 11).
    entry_count: usize,
    /// `root`, `normal` or `recycle_bin`.
    kind: &'static str,
    children: Vec<GroupView>,
}

impl From<GroupNode> for GroupView {
    fn from(node: GroupNode) -> Self {
        Self {
            id: node.id.to_string(),
            name: node.name,
            entry_count: node.entry_count,
            kind: match node.kind {
                GroupKind::Root => "root",
                GroupKind::Normal => "normal",
                GroupKind::RecycleBin => "recycle_bin",
            },
            children: node.children.into_iter().map(Self::from).collect(),
        }
    }
}

/// `{ entries, subgroups }`: what deleting a group takes with it, every level counted (RF-56).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DeletePreviewView {
    entries: usize,
    subgroups: usize,
}

/// `group_create{parent_id, name}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupCreateArgs {
    parent_id: String,
    name: String,
}

/// `group_rename{id, name}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupRenameArgs {
    id: String,
    name: String,
}

/// `group_move{id, parent_id}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMoveArgs {
    id: String,
    parent_id: String,
}

/// `group_delete_preview{id}` and `group_delete{id}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupIdArgs {
    id: String,
}

/// The tree of groups with the number of entries of each (RF-50).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`.
#[tauri::command]
pub async fn group_tree(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    expect_no_args(request.body())?;
    blocking(move || Ok(session.read(|vault| Ok(vault.group_tree()))?.into())).await
}

/// Creates a group inside another (RF-51, RF-52) and answers the new tree.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`; `NombreVacio`, `CampoMuyLargo`,
/// `CaracterNoPermitido` (with `field` and `limit`).
#[tauri::command]
pub async fn group_create(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: GroupCreateArgs = parse_args(request.body())?;
    let parent = GroupId::parse(&args.parent_id)?;
    blocking(move || {
        session
            .mutate(|vault, now| {
                vault.create_group(parent, &args.name, now)?;
                Ok(Applied::Changed(vault.group_tree()))
            })
            .map(GroupView::from)
            .map_err(|error| with_field_data(error, || validate_group_name(&args.name).map(drop)))
    })
    .await
}

/// Renames a group (RF-53) and answers the new tree.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`; `GrupoProtegido` (the recycle bin, RF-57);
/// `NombreVacio`, `CampoMuyLargo`, `CaracterNoPermitido` (with `field` and `limit`).
#[tauri::command]
pub async fn group_rename(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: GroupRenameArgs = parse_args(request.body())?;
    let id = GroupId::parse(&args.id)?;
    blocking(move || {
        session
            .mutate(|vault, now| {
                let before = vault.group_tree();
                vault.rename_group(id, &args.name, now)?;
                let after = vault.group_tree();
                Ok(Applied::when(before != after, after))
            })
            .map(GroupView::from)
            .map_err(|error| with_field_data(error, || validate_group_name(&args.name).map(drop)))
    })
    .await
}

/// Moves a group with its content into another (RF-54) and answers the new tree.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`; `GrupoProtegido` (RF-57);
/// `MovimientoInvalido` (RF-55).
#[tauri::command]
pub async fn group_move(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: GroupMoveArgs = parse_args(request.body())?;
    let id = GroupId::parse(&args.id)?;
    let parent = GroupId::parse(&args.parent_id)?;
    blocking(move || {
        let tree = session.mutate(|vault, now| {
            let before = vault.group_tree();
            vault.move_group(id, parent, now)?;
            let after = vault.group_tree();
            Ok(Applied::when(before != after, after))
        })?;
        Ok(tree.into())
    })
    .await
}

/// How many entries and subgroups deleting the group would take (RF-56).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `NoEncontrado`; `GrupoProtegido` if it cannot be deleted (plan,
/// "Precisiones" 17).
#[tauri::command]
pub async fn group_delete_preview(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<DeletePreviewView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: GroupIdArgs = parse_args(request.body())?;
    let id = GroupId::parse(&args.id)?;
    blocking(move || {
        let DeletePreview { entries, subgroups } =
            session.read(|vault| vault.delete_preview(id))?;
        Ok(DeletePreviewView { entries, subgroups })
    })
    .await
}

/// Deletes a group with its content: into the recycle bin, or for good if it is already there (RF-56, RF-58).
/// Answers the new tree.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`; `GrupoProtegido` (RF-57).
#[tauri::command]
pub async fn group_delete(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: GroupIdArgs = parse_args(request.body())?;
    let id = GroupId::parse(&args.id)?;
    blocking(move || {
        let tree = session.mutate(|vault, now| {
            vault.delete_group(id, now)?;
            Ok(Applied::Changed(vault.group_tree()))
        })?;
        Ok(tree.into())
    })
    .await
}

/// Empties the recycle bin for good (RF-58) and answers the new tree.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`.
#[tauri::command]
pub async fn recycle_bin_empty(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<GroupView, AppError> {
    let session = state.session();
    require_open(&session)?;
    expect_no_args(request.body())?;
    blocking(move || {
        let tree = session.mutate(|vault, now| {
            let removed = vault.empty_recycle_bin(now)?;
            Ok(Applied::when(
                removed != DeletePreview::default(),
                vault.group_tree(),
            ))
        })?;
        Ok(tree.into())
    })
    .await
}
