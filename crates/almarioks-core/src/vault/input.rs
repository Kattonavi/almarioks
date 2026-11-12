//! Validation of what the owner writes in an entry (RF-39, RF-40, RF-41, RF-91, spec "Definiciones").

use secrecy::{ExposeSecret, SecretString};

use super::model::{
    EntryPatch, MAX_TAGS, NOTES_MAX_CHARS, NewEntry, PASSWORD_MAX_CHARS, TAG_MAX_CHARS,
    TITLE_MAX_CHARS, URL_MAX_CHARS, USERNAME_MAX_CHARS,
};
use crate::{
    error::{CoreError, CoreResult},
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

    /// Validates a patch, in the order of the spec: `TituloVacio` › `CampoMuyLargo` › `CaracterNoPermitido` ›
    /// `EtiquetaInvalida`. Only the fields present are checked. Limits count characters, not bytes.
    pub(crate) fn check(patch: EntryPatch) -> CoreResult<Self> {
        let title = patch.title.map(|t| text::trim(&t).to_owned());
        if title.as_deref().is_some_and(str::is_empty) {
            return Err(CoreError::TituloVacio);
        }
        let tags: Option<Vec<String>> = patch
            .tags
            .map(|tags| tags.iter().map(|t| text::trim(t).to_owned()).collect());
        let password = patch.password;
        let password_text = password.as_ref().map(ExposeSecret::expose_secret);

        let too_long =
            |value: Option<&str>, max: usize| value.is_some_and(|v| text::char_count(v) > max);
        if too_long(title.as_deref(), TITLE_MAX_CHARS)
            || too_long(patch.username.as_deref(), USERNAME_MAX_CHARS)
            || too_long(patch.url.as_deref(), URL_MAX_CHARS)
            || too_long(password_text, PASSWORD_MAX_CHARS)
            || too_long(patch.notes.as_deref(), NOTES_MAX_CHARS)
            || tags.as_ref().is_some_and(|tags| {
                tags.len() > MAX_TAGS || tags.iter().any(|t| text::char_count(t) > TAG_MAX_CHARS)
            })
        {
            return Err(CoreError::CampoMuyLargo);
        }

        let forbidden = |value: Option<&str>| value.is_some_and(text::has_forbidden_char);
        if forbidden(title.as_deref())
            || forbidden(patch.username.as_deref())
            || forbidden(patch.url.as_deref())
            || forbidden(password_text)
            || forbidden(patch.notes.as_deref())
            || tags
                .as_ref()
                .is_some_and(|tags| tags.iter().any(|t| text::has_forbidden_char(t)))
        {
            return Err(CoreError::CaracterNoPermitido);
        }

        // KeePass and the library split tags on `;`, `,` and tab: a tab inside a tag would not survive a save, so
        // it is rejected like `;` and `,` (decision pending, conservative reading of RF-91).
        if tags.as_ref().is_some_and(|tags| {
            tags.iter()
                .any(|t| t.is_empty() || t.contains([';', ',', '\t']))
        }) {
            return Err(CoreError::EtiquetaInvalida);
        }

        Ok(Self {
            title,
            username: patch.username,
            password,
            url: patch.url,
            notes: patch.notes,
            tags,
        })
    }
}
