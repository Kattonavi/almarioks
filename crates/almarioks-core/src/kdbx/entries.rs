//! Entries (RF-26, RF-38 to RF-41, RF-45, RF-91) and saving to bytes.
//!
//! Edits never go through the library's `EntryTrack`: its `Drop` always pushes a version, its setters stamp the
//! library's own clock, and `History::add_entry` puts the new version first. Here the diff comes first (no change
//! = nothing touched), the time is the injected `now`, and the history is kept oldest to newest, as KeePass writes
//! it (`CLAUDE.md` → "Reglas específicas"; RF-26).

use chrono::NaiveDateTime;
use keepass::db::{CustomDataValue, Entry, History, Times, Value, fields};
use secrecy::ExposeSecret;

use super::{
    key::VaultKey,
    open::OpenedVault,
    read::{kp_entry, kp_group},
};
use crate::{
    error::{CoreError, CoreResult},
    vault::{CheckedFields, EntryId, EntryPatch, GroupId, NewEntry, Timestamp},
};

pub(crate) fn naive(time: Timestamp) -> NaiveDateTime {
    time.datetime().naive_utc()
}

/// Sets every time of a new entry or group to `now` (the library stamps its own clock on creation).
pub(crate) fn stamp_new(times: &mut Times, now: Timestamp) {
    let now = naive(now);
    times.creation = Some(now);
    times.last_modification = Some(now);
    times.last_access = Some(now);
    times.location_changed = Some(now);
    times.expiry = now;
}

/// Size of one history version, modelled on KeePass' `PwEntry.GetSize` as we understand it: 128 fixed + UTF-16
/// lengths of field names and values, tags, override URL and custom data. Attachments are not counted (the
/// library keeps an entry's attachment list private). Not checked against KeePass' source: an estimate for
/// `HistoryMaxSize` (decision pending).
fn version_size(entry: &Entry) -> u64 {
    let units = |text: &str| u64::try_from(text.encode_utf16().count()).unwrap_or(u64::MAX);
    let mut size: u64 = 128;
    for (name, value) in &entry.fields {
        size = size
            .saturating_add(units(name))
            .saturating_add(units(value.get()));
    }
    for tag in &entry.tags {
        size = size.saturating_add(units(tag));
    }
    if let Some(url) = &entry.override_url {
        size = size.saturating_add(units(url));
    }
    for (key, item) in &entry.custom_data {
        size = size.saturating_add(units(key));
        if let Some(CustomDataValue::String(value)) = &item.value {
            size = size.saturating_add(units(value));
        }
    }
    size
}

/// Drops the oldest versions until the vault's limits hold (RF-26). A missing or negative limit means none, as in
/// KeePass. Returns true if a version was dropped.
fn prune(versions: &mut Vec<Entry>, max_items: Option<isize>, max_size: Option<isize>) -> bool {
    let mut dropped = false;
    if let Some(max) = max_items.and_then(|m| usize::try_from(m).ok()) {
        while versions.len() > max {
            versions.remove(0);
            dropped = true;
        }
    }
    if let Some(max) = max_size.and_then(|m| u64::try_from(m).ok()) {
        let total = |versions: &[Entry]| {
            versions
                .iter()
                .map(version_size)
                .fold(0u64, u64::saturating_add)
        };
        while !versions.is_empty() && total(versions) > max {
            versions.remove(0);
            dropped = true;
        }
    }
    dropped
}

impl OpenedVault {
    /// `SoloLectura` for a KDBX 3.x vault (RF-14): the first check of every change.
    pub(crate) fn ensure_writable(&self) -> CoreResult<()> {
        if self.is_read_only() {
            Err(CoreError::SoloLectura)
        } else {
            Ok(())
        }
    }

    /// Creates an entry in a group (RF-38): title (trimmed), user name, password (exactly as given, RF-41), URL,
    /// notes and tags. Only the password is protected (spec "Campo protegido"). Every time is `now`.
    ///
    /// # Errors
    /// In this order: [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`] (group), [`CoreError::TituloVacio`],
    /// [`CoreError::CampoMuyLargo`], [`CoreError::CaracterNoPermitido`], [`CoreError::EtiquetaInvalida`].
    pub fn create_entry(
        &mut self,
        group: GroupId,
        entry: NewEntry,
        now: Timestamp,
    ) -> CoreResult<EntryId> {
        self.ensure_writable()?;
        if self.db.group(kp_group(group)).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        let checked = CheckedFields::from_new(entry)?;
        let mut parent = self
            .db
            .group_mut(kp_group(group))
            .ok_or(CoreError::NoEncontrado)?;
        let mut new = parent.add_entry();
        let plain = [
            (fields::TITLE, checked.title),
            (fields::USERNAME, checked.username),
            (fields::URL, checked.url),
            (fields::NOTES, checked.notes),
        ];
        for (name, value) in plain {
            new.set_unprotected(name, value.unwrap_or_default());
        }
        let password = checked
            .password
            .as_ref()
            .map_or_else(String::new, |p| p.expose_secret().to_owned());
        new.set_protected(fields::PASSWORD, password);
        new.tags = checked.tags.unwrap_or_default();
        stamp_new(&mut new.times, now);
        Ok(EntryId::from_uuid(new.id().uuid()))
    }

    /// Applies a patch (RF-45): an absent field stays, a present one is replaced (also by the empty text). If no
    /// value changes, nothing is touched: neither the date nor the history. Otherwise the previous version goes to
    /// the end of the history (oldest first), the vault's history limits drop the oldest versions (RF-26), and the
    /// modification time is `now`. A field keeps its protection; a field that did not exist is protected only if it
    /// is the password. Returns whether the entry changed.
    ///
    /// # Errors
    /// In this order: [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`], then the rules of
    /// [`Self::create_entry`] for the fields present.
    pub fn update_entry(
        &mut self,
        id: EntryId,
        patch: EntryPatch,
        now: Timestamp,
    ) -> CoreResult<bool> {
        self.ensure_writable()?;
        let entry = self.db.entry(kp_entry(id)).ok_or(CoreError::NoEncontrado)?;
        let checked = CheckedFields::check(patch)?;

        let password = checked.password.as_ref().map(|p| p.expose_secret());
        let wanted: [(&str, Option<&str>); 5] = [
            (fields::TITLE, checked.title.as_deref()),
            (fields::USERNAME, checked.username.as_deref()),
            (fields::PASSWORD, password),
            (fields::URL, checked.url.as_deref()),
            (fields::NOTES, checked.notes.as_deref()),
        ];
        let changes: Vec<(&str, String)> = wanted
            .iter()
            .filter_map(|&(name, value)| {
                let value = value?;
                (entry.get(name).unwrap_or("") != value).then(|| (name, value.to_owned()))
            })
            .collect();
        let new_tags = checked.tags.filter(|tags| *tags != entry.tags);
        if changes.is_empty() && new_tags.is_none() {
            return Ok(false);
        }

        let max_items = self.db.meta.history_max_items;
        let max_size = self.db.meta.history_max_size;
        let mut target = self
            .db
            .entry_mut(kp_entry(id))
            .ok_or(CoreError::NoEncontrado)?;
        let mut previous: Entry = Entry::clone(&target);
        previous.history = None;
        for (name, value) in changes {
            let protected = target
                .fields
                .get(name)
                .map_or(name == fields::PASSWORD, Value::is_protected);
            let value = if protected {
                Value::protected(value)
            } else {
                Value::unprotected(value)
            };
            target.set(name, value);
        }
        if let Some(tags) = new_tags {
            target.tags = tags;
        }
        target.times.last_modification = Some(naive(now));

        let mut versions: Vec<Entry> = target
            .history
            .as_ref()
            .map(|h| h.get_entries().clone())
            .unwrap_or_default();
        versions.push(previous);
        let dropped = prune(&mut versions, max_items, max_size);
        // `add_entry` inserts at the front: feed it newest first so the oldest ends up first.
        let mut history = History::default();
        for version in versions.into_iter().rev() {
            history.add_entry(version);
        }
        target.history = Some(history);
        if dropped {
            // The library's own fix-up after replacing a history: rebuilds attachment references and drops the
            // attachments only the dropped versions used.
            self.db.remove_unused_attachments();
        }
        Ok(true)
    }

    /// The vault encrypted to bytes with its key, ready for the disk (T8 writes them). Salt and seeds are fresh.
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`] for KDBX 3.x (RF-14); [`CoreError::GuardadoFallido`] if the library cannot
    /// write it.
    pub fn to_bytes(&self, key: &VaultKey) -> CoreResult<Vec<u8>> {
        self.ensure_writable()?;
        let mut bytes = Vec::new();
        self.db
            .save(&mut bytes, key.to_database_key()?)
            .map_err(|_| CoreError::GuardadoFallido)?;
        Ok(bytes)
    }
}
