//! Validation of what the owner writes in an entry (RF-39, RF-40, RF-41, RF-91, spec "Definiciones").
//!
//! The rules run on borrowed text and answer a [`FieldError`] (code, field and limit, RF-40); the vault's
//! operations turn it into the plain [`CoreError`] code. The window learns the field and the limit from
//! [`validate_new_entry`] or [`validate_patch`] AFTER an operation answered one of these codes, so the precedence
//! of the spec (`SoloLectura` > `NoEncontrado` > domain rule) is kept.

use secrecy::{ExposeSecret, SecretString};

use super::model::{
    EntryPatch, MAX_TAGS, NOTES_MAX_CHARS, NewEntry, PASSWORD_MAX_CHARS, TAG_MAX_CHARS,
    TITLE_MAX_CHARS, URL_MAX_CHARS, USERNAME_MAX_CHARS,
};
use crate::{
    error::{CoreError, CoreResult, Field, FieldError},
    text,
};

/// The fields of an entry after validation: title and tags trimmed; the rest exactly as given (RF-41).
#[derive(Default)]
pub(crate) struct CheckedFields {
    pub(crate) title: Option<String>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<SecretString>,
    pub(crate) url: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
}

/// Borrowed view of the fields to check; `None` = not in the request.
struct Fields<'a> {
    title: Option<&'a str>,
    username: Option<&'a str>,
    password: Option<&'a str>,
    url: Option<&'a str>,
    notes: Option<&'a str>,
    tags: Option<&'a [String]>,
}

impl<'a> Fields<'a> {
    fn of_new(entry: &'a NewEntry) -> Self {
        Self {
            title: Some(&entry.title),
            username: Some(&entry.username),
            // A missing password is the empty one, which breaks no rule.
            password: Some(
                entry
                    .password
                    .as_ref()
                    .map_or("", ExposeSecret::expose_secret),
            ),
            url: Some(&entry.url),
            notes: Some(&entry.notes),
            tags: Some(&entry.tags),
        }
    }

    fn of_patch(patch: &'a EntryPatch) -> Self {
        Self {
            title: patch.title.as_deref(),
            username: patch.username.as_deref(),
            password: patch.password.as_ref().map(ExposeSecret::expose_secret),
            url: patch.url.as_deref(),
            notes: patch.notes.as_deref(),
            tags: patch.tags.as_deref(),
        }
    }

    /// The rules, in the order of the spec: `TituloVacio` > `CampoMuyLargo` > `CaracterNoPermitido` >
    /// `EtiquetaInvalida`. Within one code, the first field in the order title, user name, URL, password, notes,
    /// tags (decision pending: the spec names one field per error). Limits count characters, not bytes; the title
    /// and the tags are checked trimmed, as they are stored.
    fn check(&self) -> Result<(), FieldError> {
        let title = self.title.map(text::trim);
        if title.is_some_and(str::is_empty) {
            return Err(FieldError::new(CoreError::TituloVacio, Field::Title));
        }
        let tags: Option<Vec<&str>> = self
            .tags
            .map(|tags| tags.iter().map(|t| text::trim(t)).collect());

        let plain = [
            (Field::Title, title, TITLE_MAX_CHARS),
            (Field::UserName, self.username, USERNAME_MAX_CHARS),
            (Field::Url, self.url, URL_MAX_CHARS),
            (Field::Password, self.password, PASSWORD_MAX_CHARS),
            (Field::Notes, self.notes, NOTES_MAX_CHARS),
        ];
        for (field, value, max) in plain {
            if value.is_some_and(|v| text::char_count(v) > max) {
                return Err(FieldError::too_long(field, max));
            }
        }
        if let Some(tags) = &tags {
            // Both limits of the tags are 50 (RF-40); the count is checked first (decision pending).
            if tags.len() > MAX_TAGS {
                return Err(FieldError::too_long(Field::Tags, MAX_TAGS));
            }
            if tags.iter().any(|t| text::char_count(t) > TAG_MAX_CHARS) {
                return Err(FieldError::too_long(Field::Tags, TAG_MAX_CHARS));
            }
        }

        for (field, value, _) in plain {
            if value.is_some_and(text::has_forbidden_char) {
                return Err(FieldError::new(CoreError::CaracterNoPermitido, field));
            }
        }
        if tags
            .as_ref()
            .is_some_and(|tags| tags.iter().any(|t| text::has_forbidden_char(t)))
        {
            return Err(FieldError::new(CoreError::CaracterNoPermitido, Field::Tags));
        }

        // KeePass and the library split tags on `;`, `,` and tab: a tab inside a tag would not survive a save, so
        // it is rejected like `;` and `,` (decision pending, conservative reading of RF-91).
        if tags.as_ref().is_some_and(|tags| {
            tags.iter()
                .any(|t| t.is_empty() || t.contains([';', ',', '\t']))
        }) {
            return Err(FieldError::new(CoreError::EtiquetaInvalida, Field::Tags));
        }
        Ok(())
    }
}

/// Checks a new entry without creating it and says which rule, field and limit it breaks (RF-39, RF-40, RF-91).
///
/// # Errors
/// The first [`FieldError`] in the order of the spec.
pub fn validate_new_entry(entry: &NewEntry) -> Result<(), FieldError> {
    Fields::of_new(entry).check()
}

/// Checks an edit without applying it; only the fields present are checked (RF-45).
///
/// # Errors
/// The first [`FieldError`] in the order of the spec.
pub fn validate_patch(patch: &EntryPatch) -> Result<(), FieldError> {
    Fields::of_patch(patch).check()
}

impl CheckedFields {
    /// Validates a new entry: every field is present (a missing password is the empty one).
    pub(crate) fn from_new(entry: NewEntry) -> CoreResult<Self> {
        Self::check(EntryPatch {
            title: Some(entry.title),
            username: Some(entry.username),
            password: Some(
                entry
                    .password
                    .unwrap_or_else(|| SecretString::from(String::new())),
            ),
            url: Some(entry.url),
            notes: Some(entry.notes),
            tags: Some(entry.tags),
        })
    }

    /// Validates a patch ([`validate_patch`]) and keeps the values to store: title and tags trimmed.
    pub(crate) fn check(patch: EntryPatch) -> CoreResult<Self> {
        validate_patch(&patch)?;
        Ok(Self {
            title: patch.title.map(|t| text::trim(&t).to_owned()),
            username: patch.username,
            password: patch.password,
            url: patch.url,
            notes: patch.notes,
            tags: patch
                .tags
                .map(|tags| tags.iter().map(|t| text::trim(t).to_owned()).collect()),
        })
    }
}
