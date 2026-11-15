//! Entries, search, copy and "open the URL" (spec 001, sections E, G and I; RF-76).
//!
//! What crosses to the window (constitution rule 6): the metadata of an entry and the values that are not
//! protected. A protected standard field travels as `null`; a protected custom field has no `value`. The only
//! command that hands out a protected value is `entry_reveal`, and only the one field asked for (RF-44).
//! `entry_copy` and `entry_open_url` use a value without it ever being part of an answer.

use almarioks_core::{
    CoreError, Field,
    search::{SearchHit, SearchResult},
    session::{Applied, Session},
    vault::{
        EntryDetail, EntryId, EntryPatch, EntrySummary, GroupId, NewEntry, validate_new_entry,
        validate_patch,
    },
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};
use tauri::{State, ipc::Request};

use super::{
    args::{SecretArg, parse_args, present},
    blocking,
    error::AppError,
    require_open, url, with_field_data,
};
use crate::{clipboard::ClipboardService, lock::LockCenter, state::AppState};

/// `EntrySummary = { id, title, username, url, tags, modified }`; a protected field is `null`.
#[derive(Serialize)]
pub struct EntrySummaryView {
    id: String,
    title: Option<String>,
    username: Option<String>,
    url: Option<String>,
    tags: Vec<String>,
    /// ISO 8601 in UTC.
    modified: String,
}

impl From<EntrySummary> for EntrySummaryView {
    fn from(summary: EntrySummary) -> Self {
        Self {
            id: summary.id.to_string(),
            title: summary.title,
            username: summary.username,
            url: summary.url,
            tags: summary.tags,
            modified: summary.modified.to_iso8601(),
        }
    }
}

/// A custom field: its value only if it is not protected.
#[derive(Serialize)]
pub struct CustomFieldView {
    name: String,
    protected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

/// The name and size of an attachment, never its content.
#[derive(Serialize)]
pub struct AttachmentView {
    name: String,
    size: usize,
}

/// `EntryDetail`: the summary's fields, flat, plus the rest (plan, "Contrato").
#[derive(Serialize)]
pub struct EntryDetailView {
    #[serde(flatten)]
    summary: EntrySummaryView,
    group_id: String,
    notes: Option<String>,
    /// KeePass names of the protected fields that have a value (plan, "Precisiones" 16).
    protected_fields: Vec<String>,
    custom_fields: Vec<CustomFieldView>,
    attachments: Vec<AttachmentView>,
    /// ISO 8601 in UTC.
    created: String,
}

impl From<EntryDetail> for EntryDetailView {
    fn from(detail: EntryDetail) -> Self {
        Self {
            summary: detail.summary.into(),
            group_id: detail.group_id.to_string(),
            notes: detail.notes,
            protected_fields: detail.protected_fields,
            custom_fields: detail
                .custom_fields
                .into_iter()
                .map(|field| CustomFieldView {
                    name: field.name,
                    protected: field.protected,
                    // The core already leaves it out; said again here so this file alone shows the rule.
                    value: field.value.filter(|_| !field.protected),
                })
                .collect(),
            attachments: detail
                .attachments
                .into_iter()
                .map(|attachment| AttachmentView {
                    name: attachment.name,
                    size: attachment.size,
                })
                .collect(),
            created: detail.created.to_iso8601(),
        }
    }
}

/// A search result: the summary's fields, flat, plus `group_path`, root included (plan, "Precisiones" 7, 11).
#[derive(Serialize)]
pub struct SearchHitView {
    #[serde(flatten)]
    summary: EntrySummaryView,
    group_path: Vec<String>,
}

/// `{ value }` of `entry_reveal`. The value stays wrapped until it is written into the answer; no `Debug`.
pub struct RevealView {
    value: SecretString,
}

impl Serialize for RevealView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = serializer.serialize_struct("RevealView", 1)?;
        fields.serialize_field("value", self.value.expose_secret())?;
        fields.end()
    }
}

/// `{ clears_in_seconds }` of `entry_copy`: `null` for a field that is not protected (plan, "Precisiones" 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CopyView {
    clears_in_seconds: Option<u32>,
}

/// `entry_list{group_id}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryListArgs {
    group_id: String,
}

/// `entry_get{id}`, `entry_delete{id}`, `entry_open_url{id}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryIdArgs {
    id: String,
}

/// `entry_create{group_id, title, username, password, url, notes, tags}`. No `Debug`: it holds a password.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryCreateArgs {
    group_id: String,
    title: String,
    username: String,
    password: SecretArg,
    url: String,
    notes: String,
    tags: Vec<String>,
}

impl EntryCreateArgs {
    /// The entry for the core. Built from borrowed arguments because it is needed twice when a rule rejects it:
    /// once to try, once to learn which field and which limit.
    fn new_entry(&self) -> NewEntry {
        NewEntry {
            title: self.title.clone(),
            username: self.username.clone(),
            password: Some(SecretString::from(self.password.expose().to_owned())),
            url: self.url.clone(),
            notes: self.notes.clone(),
            tags: self.tags.clone(),
        }
    }
}

/// `entry_update{id, …}`: a patch. A field that is absent does not change; one that is present replaces the
/// value, also with the empty text (RF-45). `null` is neither: it is a wrong type. No `Debug`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryUpdateArgs {
    id: String,
    #[serde(default, deserialize_with = "present")]
    title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    username: Option<String>,
    #[serde(default, deserialize_with = "present")]
    password: Option<SecretArg>,
    #[serde(default, deserialize_with = "present")]
    url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    notes: Option<String>,
    #[serde(default, deserialize_with = "present")]
    tags: Option<Vec<String>>,
}

impl EntryUpdateArgs {
    /// The patch for the core; see [`EntryCreateArgs::new_entry`] for why it borrows.
    fn patch(&self) -> EntryPatch {
        EntryPatch {
            title: self.title.clone(),
            username: self.username.clone(),
            password: self
                .password
                .as_ref()
                .map(|password| SecretString::from(password.expose().to_owned())),
            url: self.url.clone(),
            notes: self.notes.clone(),
            tags: self.tags.clone(),
        }
    }
}

/// `entry_move{id, group_id}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryMoveArgs {
    id: String,
    group_id: String,
}

/// `entry_reveal{id, field}` and `entry_copy{id, field}`. `field` is the KeePass name: `Title`, `UserName`,
/// `Password`, `URL`, `Notes` or the name of a custom field (plan, "Precisiones" 16).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryFieldArgs {
    id: String,
    field: String,
}

/// `search{query}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    query: String,
}

/// The entries of a group, in title order (RF-42).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `NoEncontrado`.
#[tauri::command]
pub async fn entry_list(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<EntrySummaryView>, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryListArgs = parse_args(request.body())?;
    let group = GroupId::parse(&args.group_id)?;
    blocking(move || {
        let entries = session.read(|vault| vault.list_entries(group))?;
        Ok(entries.into_iter().map(EntrySummaryView::from).collect())
    })
    .await
}

/// An entry without any protected value (RF-43).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `NoEncontrado`.
#[tauri::command]
pub async fn entry_get(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EntryDetailView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryIdArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    blocking(move || Ok(session.read(|vault| vault.entry(id))?.into())).await
}

/// Creates an entry in a group (RF-38 to RF-41, RF-91) and answers it as `entry_get` would.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`; `TituloVacio`, `CampoMuyLargo`,
/// `CaracterNoPermitido`, `EtiquetaInvalida` (with `field` and `limit`).
#[tauri::command]
pub async fn entry_create(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EntryDetailView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryCreateArgs = parse_args(request.body())?;
    let group = GroupId::parse(&args.group_id)?;
    blocking(move || {
        session
            .mutate(|vault, now| {
                let id = vault.create_entry(group, args.new_entry(), now)?;
                Ok(Applied::Changed(vault.entry(id)?))
            })
            .map(EntryDetailView::from)
            .map_err(|error| with_field_data(error, || validate_new_entry(&args.new_entry())))
    })
    .await
}

/// Changes the fields the request brings and no other (RF-45) and answers the entry as `entry_get` would.
///
/// # Errors
/// As [`entry_create`], for the fields present.
#[tauri::command]
pub async fn entry_update(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EntryDetailView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryUpdateArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    blocking(move || {
        session
            .mutate(|vault, now| {
                let changed = vault.update_entry(id, args.patch(), now)?;
                Ok(Applied::when(changed, vault.entry(id)?))
            })
            .map(EntryDetailView::from)
            .map_err(|error| with_field_data(error, || validate_patch(&args.patch())))
    })
    .await
}

/// Moves an entry to another group (RF-46).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`.
#[tauri::command]
pub async fn entry_move(state: State<'_, AppState>, request: Request<'_>) -> Result<(), AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryMoveArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    let group = GroupId::parse(&args.group_id)?;
    blocking(move || {
        session.mutate(|vault, now| {
            // Moving an entry to the group it is in changes nothing, and then nothing is saved.
            let was_in = vault.entry(id).map(|entry| entry.group_id).ok();
            vault.move_entry(id, group, now)?;
            Ok(Applied::when(was_in != Some(group), ()))
        })?;
        Ok(())
    })
    .await
}

/// Deletes an entry: into the recycle bin, or for good if it is already there (RF-47, RF-48).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `NoEncontrado`.
#[tauri::command]
pub async fn entry_delete(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<(), AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryIdArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    blocking(move || {
        session.mutate(|vault, now| {
            vault.delete_entry(id, now)?;
            Ok(Applied::Changed(()))
        })?;
        Ok(())
    })
    .await
}

/// Hands the window the value of ONE field of ONE entry, because the person asked to see it (RF-44). One of the
/// two commands allowed to answer a protected value (RF-76, ADR-0002).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `NoEncontrado` (the entry or the field).
#[tauri::command]
pub async fn entry_reveal(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<RevealView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryFieldArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    blocking(move || {
        let value = session.read(|vault| vault.reveal_field(id, &args.field))?;
        Ok(RevealView { value })
    })
    .await
}

/// Whether copying `field` starts the count that clears the clipboard. Every protected field does (spec, "Campo
/// protegido"): the password always, a standard field the vault marks as protected (it is `None` in the detail),
/// a protected custom field. A name that is not known counts as protected: when in doubt, the clipboard is cleared.
fn is_protected(detail: &EntryDetail, field: &str) -> bool {
    if field == Field::Password.name() {
        return true;
    }
    let standard = [
        (Field::Title, detail.summary.title.is_none()),
        (Field::UserName, detail.summary.username.is_none()),
        (Field::Url, detail.summary.url.is_none()),
        (Field::Notes, detail.notes.is_none()),
    ];
    if let Some((_, protected)) = standard.iter().find(|(name, _)| name.name() == field) {
        return *protected;
    }
    detail
        .custom_fields
        .iter()
        .find(|custom| custom.name == field)
        .is_none_or(|custom| custom.protected)
}

/// Reads the field and puts it in the clipboard in ONE turn of the session, so the value never leaves the core's
/// hands and copy and lock are served one at a time (RF-71, RF-90; plan, "Concurrencia y bloqueo").
///
/// The race with a lock, case by case:
/// - The lock comes before the turn: the session answers `Bloqueada` and nothing is read.
/// - The lock comes while the value is on its way to the clipboard: the clipboard service takes it back and
///   answers `Bloqueada`.
/// - The lock comes during the turn but its clipboard cleaning ran before the copy started: the copy lands, the
///   session answers `Bloqueada` because the turn ended, and the value is taken back here.
/// - The lock comes after: it clears the clipboard like any lock (RF-31).
///
/// The clipboard call is the one thing done inside the turn that is not memory: it is a short system call, and
/// doing it outside would let a value read before a lock be copied after it.
fn copy_field(
    session: &Session,
    center: &LockCenter,
    clipboard: &ClipboardService,
    clear_after_seconds: u32,
    id: EntryId,
    field: &str,
) -> Result<CopyView, AppError> {
    let copied = session.read(|vault| {
        let protected = is_protected(&vault.entry(id)?, field);
        let value = vault.reveal_field(id, field)?;
        Ok(if protected {
            clipboard
                .copy(value, clear_after_seconds)
                .map(|receipt| Some(receipt.clears_in_seconds))
        } else {
            clipboard.copy_without_timer(value).map(|()| None)
        })
    });
    match copied {
        Ok(Ok(clears_in_seconds)) => Ok(CopyView { clears_in_seconds }),
        Ok(Err(refused)) => Err(refused.into()),
        Err(CoreError::Bloqueada) => {
            center.clear_clipboard();
            Err(CoreError::Bloqueada.into())
        }
        Err(other) => Err(other.into()),
    }
}

/// Copies a field to the clipboard from Rust: the value never passes through the window (RF-71). A protected
/// field is cleared after the seconds of the settings and the answer says how many (RF-73, RF-74); a field that
/// is not protected is copied with no count and answers `clears_in_seconds: null`.
///
/// # Errors
/// `Bloqueada` (also when a lock was asked for while copying: nothing is left in the clipboard, RF-90);
/// `ArgumentoInvalido`; `PortapapelesNoDisponible`; `NoEncontrado` (the entry or the field).
#[tauri::command]
pub async fn entry_copy(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<CopyView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryFieldArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    let clipboard = state.clipboard()?;
    let (center, settings) = (state.lock_center(), state.settings());
    blocking(move || {
        // The settings may touch the disk: read before taking the turn, never inside it.
        let seconds = settings.read().clipboard_seconds();
        copy_field(&session, &center, &clipboard, seconds, id, &args.field)
    })
    .await
}

/// Opens the URL of an entry in the browser, from Rust, only if it is `http` or `https` (RF-49). The URL is read
/// inside the session and never answered: it may be a protected field.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `NoEncontrado`; `UrlNoPermitida` (nothing is opened); `ErrorInterno` if the
/// system could not open it.
#[tauri::command]
pub async fn entry_open_url(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<(), AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: EntryIdArgs = parse_args(request.body())?;
    let id = EntryId::parse(&args.id)?;
    let opener = state.url_opener();
    blocking(move || {
        let address = session.read(|vault| {
            // The entry must exist (`NoEncontrado`); one without a URL field has the empty URL, which is refused.
            vault.entry(id)?;
            match vault.reveal_field(id, Field::Url.name()) {
                Err(CoreError::NoEncontrado) => Ok(SecretString::from(String::new())),
                other => other,
            }
        })?;
        url::open_url(opener.as_ref(), address.expose_secret())
    })
    .await
}

/// The entries outside the recycle bin where every word of the query appears, with the path of their group
/// (RF-59 to RF-63). A query that is empty after trimming filters nothing (RF-92): the answer is the empty list
/// and the window goes on showing the current group.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`.
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<SearchHitView>, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: SearchArgs = parse_args(request.body())?;
    blocking(move || {
        let hits = match session.read(|vault| Ok(vault.search(&args.query)))? {
            SearchResult::NoFilter => Vec::new(),
            SearchResult::Matches(hits) => hits,
        };
        Ok(hits
            .into_iter()
            .map(
                |SearchHit {
                     summary,
                     group_path,
                 }| SearchHitView {
                    summary: summary.into(),
                    group_path,
                },
            )
            .collect())
    })
    .await
}
