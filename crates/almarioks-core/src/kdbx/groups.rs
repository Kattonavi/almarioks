//! Moving and deleting entries and groups; the recycle bin and the deleted objects (RF-46 to RF-48, RF-51 to
//! RF-58, examples 3 and 4).
//!
//! Library traps avoided (lab `docs/lab/boveda-0.1`, P4 c and d): `EntryTrack::move_to` adds a history version,
//! so entries move with `EntryMut::move_to` and `location_changed` by hand; the tracked removals stamp the
//! library's clock, so deleted objects are written here with the injected `now`; the recycle bin is never
//! removed (`GroupTrack::remove` would leave `meta.recyclebin_uuid` pointing at nothing).

use keepass::db::{EntryId as KpEntryId, GroupId as KpGroupId, MoveGroupError};

use super::{
    entries::{naive, stamp_new},
    open::OpenedVault,
    read::{kp_entry, kp_group, own_group},
};
use crate::{
    error::{CoreError, CoreResult},
    vault::{DeletePreview, Deletion, EntryId, GroupId, Timestamp, check_group_name},
};

/// Name of the recycle bin this app creates.
pub const RECYCLE_BIN_NAME: &str = "Papelera";
/// KeePass' built-in recycle bin icon.
const RECYCLE_BIN_ICON: usize = 43;

impl OpenedVault {
    fn parent_of(&self, group: KpGroupId) -> Option<KpGroupId> {
        self.db.group(group)?.parent().map(|parent| parent.id())
    }

    /// True if `group` is `ancestor` or lies below it. Bounded by the number of groups.
    fn is_within(&self, group: KpGroupId, ancestor: KpGroupId) -> bool {
        let mut current = Some(group);
        for _ in 0..=self.db.num_groups() {
            match current {
                Some(id) if id == ancestor => return true,
                Some(id) => current = self.parent_of(id),
                None => return false,
            }
        }
        false
    }

    fn bin(&self) -> Option<KpGroupId> {
        self.db.recycle_bin().map(|bin| bin.id())
    }

    /// The group is the recycle bin or lies inside it.
    fn in_bin(&self, group: KpGroupId) -> bool {
        self.bin().is_some_and(|bin| self.is_within(group, bin))
    }

    /// The group is the recycle bin or one of its ancestors (RF-57).
    fn holds_bin(&self, group: KpGroupId) -> bool {
        self.bin().is_some_and(|bin| self.is_within(bin, group))
    }

    fn entry_parent(&self, entry: KpEntryId) -> Option<KpGroupId> {
        self.db.entry(entry).map(|e| e.parent().id())
    }

    /// The recycle bin, created under the root and enabled if the vault has none (RF-47). An existing bin that the
    /// metadata disables is enabled again.
    fn ensure_recycle_bin(&mut self, now: Timestamp) -> KpGroupId {
        if let Some(bin) = self.bin() {
            if self.db.meta.recyclebin_enabled != Some(true) {
                self.db.meta.recyclebin_enabled = Some(true);
                self.db.meta.recyclebin_changed = Some(naive(now));
            }
            return bin;
        }
        let mut root = self.db.root_mut();
        let mut bin = root.add_group();
        bin.name = RECYCLE_BIN_NAME.to_owned();
        bin.set_icon_builtin(RECYCLE_BIN_ICON);
        bin.enable_searching = Some(false);
        bin.enable_autotype = Some(false);
        stamp_new(&mut bin.times, now);
        let id = bin.id();
        self.db.meta.recyclebin_uuid = Some(id.uuid());
        self.db.meta.recyclebin_enabled = Some(true);
        self.db.meta.recyclebin_changed = Some(naive(now));
        id
    }

    /// Every entry and group of a subtree, the group itself included.
    fn subtree(&self, group: KpGroupId) -> (Vec<KpEntryId>, Vec<KpGroupId>) {
        let (mut entries, mut groups) = (Vec::new(), Vec::new());
        let mut pending = vec![group];
        while let Some(id) = pending.pop() {
            if let Some(node) = self.db.group(id) {
                groups.push(id);
                entries.extend(node.entry_ids());
                pending.extend(node.group_ids());
            }
        }
        (entries, groups)
    }

    fn remove_entry_permanently(&mut self, entry: KpEntryId, now: Timestamp) {
        if let Some(target) = self.db.entry_mut(entry) {
            target.remove();
            self.db
                .deleted_objects
                .insert(entry.uuid(), Some(naive(now)));
        }
    }

    /// Removes a group with its content for good and writes down every entry and group (RF-58). Never the bin.
    fn remove_group_permanently(&mut self, group: KpGroupId, now: Timestamp) {
        let (entries, groups) = self.subtree(group);
        if let Some(target) = self.db.group_mut(group) {
            target.remove();
        }
        let when = Some(naive(now));
        for uuid in entries
            .iter()
            .map(|e| e.uuid())
            .chain(groups.iter().map(|g| g.uuid()))
        {
            self.db.deleted_objects.insert(uuid, when);
        }
    }

    /// Moves an entry to another group without adding a history version (RF-46). Moving it to its own group does
    /// nothing.
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], then [`CoreError::NoEncontrado`] if the entry or the group does not exist.
    pub fn move_entry(&mut self, id: EntryId, group: GroupId, now: Timestamp) -> CoreResult<()> {
        self.ensure_writable()?;
        let (entry, group) = (kp_entry(id), kp_group(group));
        let current = self.entry_parent(entry).ok_or(CoreError::NoEncontrado)?;
        if self.db.group(group).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        if current == group {
            return Ok(());
        }
        let mut target = self.db.entry_mut(entry).ok_or(CoreError::NoEncontrado)?;
        target.move_to(group).map_err(|_| CoreError::NoEncontrado)?;
        target.times.location_changed = Some(naive(now));
        Ok(())
    }

    /// Deletes an entry: into the recycle bin (RF-47), or for good if it is already there, written down in the
    /// deleted objects (RF-48).
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], then [`CoreError::NoEncontrado`].
    pub fn delete_entry(&mut self, id: EntryId, now: Timestamp) -> CoreResult<Deletion> {
        self.ensure_writable()?;
        let entry = kp_entry(id);
        let parent = self.entry_parent(entry).ok_or(CoreError::NoEncontrado)?;
        if self.in_bin(parent) {
            self.remove_entry_permanently(entry, now);
            return Ok(Deletion::Permanent);
        }
        let bin = self.ensure_recycle_bin(now);
        let mut target = self.db.entry_mut(entry).ok_or(CoreError::NoEncontrado)?;
        target.move_to(bin).map_err(|_| CoreError::NoEncontrado)?;
        target.times.location_changed = Some(naive(now));
        Ok(Deletion::MovedToRecycleBin)
    }

    /// Creates a group with that name (trimmed) inside `parent` (RF-51, RF-52).
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`], then [`CoreError::NombreVacio`],
    /// [`CoreError::CampoMuyLargo`], [`CoreError::CaracterNoPermitido`].
    pub fn create_group(
        &mut self,
        parent: GroupId,
        name: &str,
        now: Timestamp,
    ) -> CoreResult<GroupId> {
        self.ensure_writable()?;
        let parent = kp_group(parent);
        if self.db.group(parent).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        let name = check_group_name(name)?;
        let mut parent = self.db.group_mut(parent).ok_or(CoreError::NoEncontrado)?;
        let mut group = parent.add_group();
        group.name = name;
        stamp_new(&mut group.times, now);
        Ok(own_group(group.id()))
    }

    /// Renames a group; the name is stored trimmed (RF-52, RF-53).
    ///
    /// The root may be renamed even though it holds the recycle bin, and so may any other group that holds it
    /// (RF-57); only the bin itself keeps its name.
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`], [`CoreError::GrupoProtegido`] for the recycle bin
    /// (RF-57), then the name's rules of [`Self::create_group`].
    pub fn rename_group(&mut self, id: GroupId, name: &str, now: Timestamp) -> CoreResult<()> {
        self.ensure_writable()?;
        let group = kp_group(id);
        if self.db.group(group).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        if self.bin() == Some(group) {
            return Err(CoreError::GrupoProtegido);
        }
        let name = check_group_name(name)?;
        let mut target = self.db.group_mut(group).ok_or(CoreError::NoEncontrado)?;
        if target.name != name {
            target.name = name;
            target.times.last_modification = Some(naive(now));
        }
        Ok(())
    }

    /// Moves a group with all its content into `parent` (RF-54). Moving it to its current parent does nothing.
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`], [`CoreError::GrupoProtegido`] for the root or the
    /// recycle bin (RF-57), [`CoreError::MovimientoInvalido`] into itself or a descendant (RF-55). A group that
    /// holds the bin moves with it, like any other content.
    pub fn move_group(&mut self, id: GroupId, parent: GroupId, now: Timestamp) -> CoreResult<()> {
        self.ensure_writable()?;
        let (group, parent) = (kp_group(id), kp_group(parent));
        if self.db.group(group).is_none() || self.db.group(parent).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        if group == self.db.root().id() || self.bin() == Some(group) {
            return Err(CoreError::GrupoProtegido);
        }
        if self.is_within(parent, group) {
            return Err(CoreError::MovimientoInvalido);
        }
        if self.parent_of(group) == Some(parent) {
            return Ok(());
        }
        let mut target = self.db.group_mut(group).ok_or(CoreError::NoEncontrado)?;
        target.move_to(parent).map_err(|error| match error {
            MoveGroupError::CannotMoveRoot => CoreError::GrupoProtegido,
            MoveGroupError::NotFound(_) => CoreError::NoEncontrado,
            _ => CoreError::MovimientoInvalido,
        })?;
        target.times.location_changed = Some(naive(now));
        Ok(())
    }

    /// How many entries and subgroups deleting a group would take, counting every level (RF-56, example 4).
    ///
    /// # Errors
    /// [`CoreError::NoEncontrado`]; [`CoreError::GrupoProtegido`] for the root, the recycle bin or a group that
    /// holds it, which cannot be deleted (RF-57).
    pub fn delete_preview(&self, id: GroupId) -> CoreResult<DeletePreview> {
        let group = kp_group(id);
        if self.db.group(group).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        if group == self.db.root().id() || self.holds_bin(group) {
            return Err(CoreError::GrupoProtegido);
        }
        let (entries, groups) = self.subtree(group);
        Ok(DeletePreview {
            entries: entries.len(),
            subgroups: groups.len().saturating_sub(1),
        })
    }

    /// Deletes a group with its content: into the recycle bin (RF-56), or for good if it is already there, writing
    /// down every entry and group in the deleted objects (RF-58).
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`], [`CoreError::NoEncontrado`], [`CoreError::GrupoProtegido`] for the root, the
    /// recycle bin or a group that holds it (RF-57).
    pub fn delete_group(&mut self, id: GroupId, now: Timestamp) -> CoreResult<Deletion> {
        self.ensure_writable()?;
        let group = kp_group(id);
        if self.db.group(group).is_none() {
            return Err(CoreError::NoEncontrado);
        }
        if group == self.db.root().id() || self.holds_bin(group) {
            return Err(CoreError::GrupoProtegido);
        }
        if self.in_bin(group) {
            self.remove_group_permanently(group, now);
            return Ok(Deletion::Permanent);
        }
        let bin = self.ensure_recycle_bin(now);
        let mut target = self.db.group_mut(group).ok_or(CoreError::NoEncontrado)?;
        target
            .move_to(bin)
            .map_err(|_| CoreError::MovimientoInvalido)?;
        target.times.location_changed = Some(naive(now));
        Ok(Deletion::MovedToRecycleBin)
    }

    /// Empties the recycle bin for good, writing down every entry and group it held (RF-58). The bin stays. A
    /// vault without a bin has nothing to empty. Returns what was removed.
    ///
    /// # Errors
    /// [`CoreError::SoloLectura`].
    pub fn empty_recycle_bin(&mut self, now: Timestamp) -> CoreResult<DeletePreview> {
        self.ensure_writable()?;
        let Some(bin) = self.bin() else {
            return Ok(DeletePreview::default());
        };
        let (entries, groups) = self.subtree(bin);
        let (direct_entries, direct_groups): (Vec<KpEntryId>, Vec<KpGroupId>) =
            match self.db.group(bin) {
                Some(node) => (node.entry_ids().collect(), node.group_ids().collect()),
                None => (Vec::new(), Vec::new()),
            };
        for entry in direct_entries {
            self.remove_entry_permanently(entry, now);
        }
        for group in direct_groups {
            self.remove_group_permanently(group, now);
        }
        Ok(DeletePreview {
            entries: entries.len(),
            subgroups: groups.len().saturating_sub(1),
        })
    }
}
