//! Read model of an opened vault (RF-42 to RF-44, RF-50, RF-93): group tree, a group's entries, an entry's detail
//! and revealing one field. `keepass` types become our own here and go no further (ADR-0001).

use chrono::NaiveDateTime;
use keepass::db::{Entry, EntryId as KpEntryId, EntryRef, GroupId as KpGroupId, GroupRef, fields};
use secrecy::SecretString;

use super::open::OpenedVault;
use crate::{
    error::{CoreError, CoreResult},
    vault::{
        AttachmentInfo, CustomField, EntryDetail, EntryId, EntrySummary, GroupId, GroupKind,
        GroupNode, Timestamp, title_order_key,
    },
};

/// The five standard fields, in the order the detail lists them.
const STANDARD_FIELDS: [&str; 5] = [
    fields::TITLE,
    fields::USERNAME,
    fields::PASSWORD,
    fields::URL,
    fields::NOTES,
];

pub(crate) fn kp_entry(id: EntryId) -> KpEntryId {
    KpEntryId::from_uuid(id.uuid())
}

pub(crate) fn kp_group(id: GroupId) -> KpGroupId {
    KpGroupId::from_uuid(id.uuid())
}

pub(crate) fn own_group(id: KpGroupId) -> GroupId {
    GroupId::from_uuid(id.uuid())
}

fn timestamp(time: Option<NaiveDateTime>) -> Option<Timestamp> {
    time.map(|t| Timestamp::from_datetime(t.and_utc()))
}

/// The value a window may see of a field: `None` if protected, the empty text if absent.
fn visible(entry: &Entry, name: &str) -> Option<String> {
    match entry.fields.get(name) {
        None => Some(String::new()),
        Some(value) if value.is_protected() => None,
        Some(value) => Some(value.get().clone()),
    }
}

fn summary(entry: &Entry) -> EntrySummary {
    let modified = timestamp(entry.times.last_modification)
        .or_else(|| timestamp(entry.times.creation))
        .unwrap_or(Timestamp::EPOCH);
    EntrySummary {
        id: EntryId::from_uuid(entry.id().uuid()),
        title: visible(entry, fields::TITLE),
        username: visible(entry, fields::USERNAME),
        url: visible(entry, fields::URL),
        tags: entry.tags.clone(),
        modified,
    }
}

fn detail(entry: &EntryRef<'_>) -> EntryDetail {
    let mut protected_fields: Vec<String> = STANDARD_FIELDS
        .iter()
        .filter(|name| {
            entry
                .fields
                .get(**name)
                .is_some_and(|v| v.is_protected() && !v.get().is_empty())
        })
        .map(|name| (*name).to_owned())
        .collect();
    let mut custom_fields: Vec<CustomField> = entry
        .fields
        .iter()
        .filter(|(name, _)| !STANDARD_FIELDS.contains(&name.as_str()))
        .map(|(name, value)| CustomField {
            name: name.clone(),
            protected: value.is_protected(),
            value: (!value.is_protected()).then(|| value.get().clone()),
        })
        .collect();
    custom_fields.sort_by(|a, b| a.name.cmp(&b.name));
    protected_fields.extend(
        custom_fields
            .iter()
            .filter(|field| {
                field.protected
                    && entry
                        .fields
                        .get(&field.name)
                        .is_some_and(|v| !v.get().is_empty())
            })
            .map(|field| field.name.clone()),
    );
    let mut attachments: Vec<AttachmentInfo> = entry
        .attachments_named()
        .map(|(name, attachment)| AttachmentInfo {
            name: name.to_owned(),
            size: attachment.data.get().len(),
        })
        .collect();
    attachments.sort_by(|a, b| a.name.cmp(&b.name));
    let summary = summary(entry);
    EntryDetail {
        group_id: own_group(entry.parent().id()),
        notes: visible(entry, fields::NOTES),
        protected_fields,
        custom_fields,
        attachments,
        created: timestamp(entry.times.creation).unwrap_or(summary.modified),
        summary,
    }
}

impl OpenedVault {
    /// The vault's name: the name of its root group.
    #[must_use]
    pub fn name(&self) -> String {
        self.db.root().name.clone()
    }

    /// Id of the root group.
    #[must_use]
    pub fn root_id(&self) -> GroupId {
        own_group(self.db.root().id())
    }

    /// Id of the recycle bin, if the vault's metadata names an existing group as such.
    #[must_use]
    pub fn recycle_bin_id(&self) -> Option<GroupId> {
        self.db.recycle_bin().map(|bin| own_group(bin.id()))
    }

    /// The group tree with the number of entries of each group (RF-50).
    #[must_use]
    pub fn group_tree(&self) -> GroupNode {
        self.node(&self.db.root())
    }

    fn node(&self, group: &GroupRef<'_>) -> GroupNode {
        let id = group.id();
        let kind = if id == self.db.root().id() {
            GroupKind::Root
        } else if self.db.meta.recyclebin_uuid == Some(id.uuid()) {
            GroupKind::RecycleBin
        } else {
            GroupKind::Normal
        };
        GroupNode {
            id: own_group(id),
            name: group.name.clone(),
            entry_count: group.entry_ids().count(),
            kind,
            children: group.groups().map(|child| self.node(&child)).collect(),
        }
    }

    /// The entries directly in a group, in title order (RF-42, spec "Orden por título").
    ///
    /// # Errors
    /// [`CoreError::NoEncontrado`] if the group does not exist (RF-93).
    pub fn list_entries(&self, group: GroupId) -> CoreResult<Vec<EntrySummary>> {
        let group = self
            .db
            .group(kp_group(group))
            .ok_or(CoreError::NoEncontrado)?;
        let mut list: Vec<EntrySummary> = group.entries().map(|entry| summary(&entry)).collect();
        list.sort_by_cached_key(|entry| title_order_key(entry.title.as_deref(), entry.id));
        Ok(list)
    }

    /// An entry's detail without any protected value (RF-43).
    ///
    /// # Errors
    /// [`CoreError::NoEncontrado`] if the entry does not exist (RF-93).
    pub fn entry(&self, id: EntryId) -> CoreResult<EntryDetail> {
        let entry = self.db.entry(kp_entry(id)).ok_or(CoreError::NoEncontrado)?;
        Ok(detail(&entry))
    }

    /// The value of one field of one entry, protected or not, as a secret wiped when dropped (RF-44). The name is
    /// the file's key: `Password`, `UserName`, … or a custom field's name (plan "Precisiones" 6).
    ///
    /// # Errors
    /// [`CoreError::NoEncontrado`] if the entry or the field does not exist (RF-93).
    pub fn reveal_field(&self, id: EntryId, field: &str) -> CoreResult<SecretString> {
        let entry = self.db.entry(kp_entry(id)).ok_or(CoreError::NoEncontrado)?;
        let value = entry.fields.get(field).ok_or(CoreError::NoEncontrado)?;
        Ok(SecretString::from(value.get().clone()))
    }
}
