//! Our own types of the vault content (ADR-0001: no `keepass` type leaves `kdbx`) and the validation of what the
//! owner writes (RF-39, RF-40, RF-52, RF-91, spec "Definiciones").
//!
//! Protected-field rule (spec "Campo protegido", RF-43, RF-76): a field whose value comes protected in the file
//! never travels in [`EntrySummary`] or [`EntryDetail`]. A protected standard field is `None`; an absent or
//! unprotected one is `Some` (absent = empty text). Only `OpenedVault::reveal_field` hands the value out, as a
//! secret that is wiped when dropped.

use std::fmt;

use chrono::{DateTime, SecondsFormat, Utc};
use secrecy::SecretString;
use uuid::Uuid;

use crate::{
    error::{CoreError, CoreResult, Field, FieldError},
    text,
};

/// Most characters of a title (RF-40).
pub const TITLE_MAX_CHARS: usize = 255;
/// Most characters of a user name (RF-40).
pub const USERNAME_MAX_CHARS: usize = 255;
/// Most characters of a URL (RF-40).
pub const URL_MAX_CHARS: usize = 2_048;
/// Most characters of a password (RF-40).
pub const PASSWORD_MAX_CHARS: usize = 1_024;
/// Most characters of the notes (RF-40).
pub const NOTES_MAX_CHARS: usize = 100_000;
/// Most tags of an entry (RF-40).
pub const MAX_TAGS: usize = 50;
/// Most characters of one tag (RF-40).
pub const TAG_MAX_CHARS: usize = 50;
/// Most characters of a group name (RF-52).
pub const GROUP_NAME_MAX_CHARS: usize = 255;

macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Parses the UUID text the window sends (any standard UUID form).
            ///
            /// # Errors
            /// [`CoreError::ArgumentoInvalido`] if the text is not a UUID (RF-80).
            pub fn parse(text: &str) -> CoreResult<Self> {
                Uuid::parse_str(text).map(Self).map_err(|_| CoreError::ArgumentoInvalido)
            }

            /// Wraps a UUID.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// The UUID.
            #[must_use]
            pub const fn uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            /// Lower-case hyphenated UUID, the identifier of the contract (`plan.md`).
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0.hyphenated())
            }
        }
    };
}

uuid_id!(
    /// Identifier of an entry: its UUID in the file.
    EntryId
);
uuid_id!(
    /// Identifier of a group: its UUID in the file.
    GroupId
);

/// An instant in UTC with whole seconds (what KDBX stores).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// 1970-01-01T00:00:00Z, for a file that does not say a time.
    pub const EPOCH: Self = Self(DateTime::UNIX_EPOCH);

    /// The instant `seconds` after 1970-01-01T00:00:00Z. `None` outside the range chrono represents.
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        DateTime::from_timestamp(seconds, 0).map(Self)
    }

    /// Seconds since 1970-01-01T00:00:00Z.
    #[must_use]
    pub fn unix_seconds(self) -> i64 {
        self.0.timestamp()
    }

    /// ISO 8601 in UTC, `2026-11-11T10:00:00Z` (plan "Precisiones del contrato" 11).
    #[must_use]
    pub fn to_iso8601(self) -> String {
        self.datetime().to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    pub(crate) fn datetime(self) -> DateTime<Utc> {
        self.0
    }

    pub(crate) fn from_datetime(datetime: DateTime<Utc>) -> Self {
        // Sub-second parts never come from a KDBX file; dropped so values compare equal after a reopen.
        Self::from_unix_seconds(datetime.timestamp()).unwrap_or(Self::EPOCH)
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_iso8601())
    }
}

/// What a group is to the vault (plan: `kind` = `root` | `normal` | `recycle_bin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupKind {
    /// The root group; its name is the vault's name.
    Root,
    /// Any other group.
    Normal,
    /// The group the vault's metadata names as recycle bin.
    RecycleBin,
}

/// A node of the group tree (RF-50).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupNode {
    /// Group id.
    pub id: GroupId,
    /// Group name, as stored.
    pub name: String,
    /// Entries directly in this group, not in its subgroups (plan "Precisiones" 11).
    pub entry_count: usize,
    /// Root, normal or recycle bin.
    pub kind: GroupKind,
    /// Subgroups in the order the file keeps them.
    pub children: Vec<GroupNode>,
}

/// An entry as a list shows it (RF-42). A protected standard field is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    /// Entry id.
    pub id: EntryId,
    /// Title; `None` if protected.
    pub title: Option<String>,
    /// User name; `None` if protected.
    pub username: Option<String>,
    /// URL; `None` if protected.
    pub url: Option<String>,
    /// Tags, as stored.
    pub tags: Vec<String>,
    /// Last modification (UTC).
    pub modified: Timestamp,
}

/// A field that is not one of the five standard ones (RF-43).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomField {
    /// Field name.
    pub name: String,
    /// Whether its value is protected in the file.
    pub protected: bool,
    /// The value, only if not protected.
    pub value: Option<String>,
}

/// The name and size of an attachment; never its content (RF-43, RF-86).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentInfo {
    /// Attachment name.
    pub name: String,
    /// Size in bytes.
    pub size: usize,
}

/// An entry as its detail view shows it (RF-43): no protected value inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDetail {
    /// The list fields.
    pub summary: EntrySummary,
    /// The group the entry is in.
    pub group_id: GroupId,
    /// Notes; `None` if protected.
    pub notes: Option<String>,
    /// Names of the protected fields that have a value: standard ones first (Title, UserName, Password, URL,
    /// Notes), then custom ones by name. These are the names `reveal_field` takes.
    pub protected_fields: Vec<String>,
    /// Custom fields by name (code point order).
    pub custom_fields: Vec<CustomField>,
    /// Attachments by name (code point order).
    pub attachments: Vec<AttachmentInfo>,
    /// Creation (UTC).
    pub created: Timestamp,
}

/// A new entry (RF-38). The password is a secret: wiped when dropped, never in `Debug`.
#[derive(Debug, Default)]
pub struct NewEntry {
    /// Title; stored trimmed and must not be empty (RF-39).
    pub title: String,
    /// User name.
    pub username: String,
    /// Password, stored exactly as given (RF-41).
    pub password: Option<SecretString>,
    /// URL.
    pub url: String,
    /// Notes.
    pub notes: String,
    /// Tags; each stored trimmed (RF-91).
    pub tags: Vec<String>,
}

/// An edit (RF-45): `None` = leave the field as it is; `Some` = replace it, also by the empty text.
#[derive(Debug, Default)]
pub struct EntryPatch {
    /// New title (trimmed; must not be empty).
    pub title: Option<String>,
    /// New user name.
    pub username: Option<String>,
    /// New password, stored exactly as given.
    pub password: Option<SecretString>,
    /// New URL.
    pub url: Option<String>,
    /// New notes.
    pub notes: Option<String>,
    /// New tag list.
    pub tags: Option<Vec<String>>,
}

/// What deleting a group would remove, counting every level (RF-56, example 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeletePreview {
    /// Entries in the group and all its subgroups.
    pub entries: usize,
    /// Subgroups at every level, the group itself not counted.
    pub subgroups: usize,
}

/// What a delete did (RF-47, RF-48, RF-56, RF-58).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deletion {
    /// Moved into the recycle bin (created first if the vault had none).
    MovedToRecycleBin,
    /// Removed for good and written down in the vault's deleted objects.
    Permanent,
}

/// A group name after validation: trimmed, not empty, at most [`GROUP_NAME_MAX_CHARS`], no forbidden character
/// (RF-52, RF-53).
///
/// # Errors
/// [`CoreError::NombreVacio`], [`CoreError::CampoMuyLargo`] or [`CoreError::CaracterNoPermitido`], in that order.
pub fn check_group_name(name: &str) -> CoreResult<String> {
    validate_group_name(name).map_err(CoreError::from)
}

/// [`check_group_name`] with the data of the error: field [`Field::Name`] and, for `CampoMuyLargo`, the limit
/// (RF-40, plan "Precisiones" 15).
///
/// # Errors
/// The same codes as [`check_group_name`], as a [`FieldError`].
pub fn validate_group_name(name: &str) -> Result<String, FieldError> {
    let name = text::trim(name);
    if name.is_empty() {
        return Err(FieldError::new(CoreError::NombreVacio, Field::Name));
    }
    if text::char_count(name) > GROUP_NAME_MAX_CHARS {
        return Err(FieldError::too_long(Field::Name, GROUP_NAME_MAX_CHARS));
    }
    if text::has_forbidden_char(name) {
        return Err(FieldError::new(CoreError::CaracterNoPermitido, Field::Name));
    }
    Ok(name.to_owned())
}

/// Sort key of the "orden por título": folded title, then UUID (spec "Definiciones"). A protected title sorts as
/// the empty text, so its value does not leak through the order (decision pending).
pub(crate) fn title_order_key(title: Option<&str>, id: EntryId) -> (String, EntryId) {
    (text::fold(title.unwrap_or("")), id)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{EntryId, GroupId, Timestamp};
    use crate::error::CoreError;

    #[test]
    fn ids_parse_and_print_the_uuid() {
        let text = "0f8fad5b-d9cb-469f-a165-70867728950e";
        assert_eq!(EntryId::parse(text).unwrap().to_string(), text);
        assert_eq!(GroupId::parse("nope"), Err(CoreError::ArgumentoInvalido));
    }

    #[test]
    fn timestamps_print_iso_8601_in_utc() {
        let t = Timestamp::from_unix_seconds(1_794_391_200).unwrap();
        assert_eq!(t.to_iso8601(), "2026-11-11T10:00:00Z");
        assert_eq!(Timestamp::EPOCH.to_iso8601(), "1970-01-01T00:00:00Z");
    }
}
