//! Converting the open KDBX 3.x vault to KDBX 4 (RF-15), as a command of the session: only the session has the
//! key, so only it can write the same content in the new format.
//!
//! Built from the pieces `save_as` and `reload` already use, with their rule: the disk is never touched while a
//! secret is held. The original file is only read.

use std::{path::Path, sync::Arc};

use super::{Session, Status, describe, gate::hold, live::Facts};
use crate::{
    error::{CoreError, CoreResult},
    kdbx::{self, NewVaultKdf},
    storage::{read_vault, save_vault_as},
};

impl Session {
    /// Writes the open KDBX 3.x vault as a KDBX 4 file at `new_path`, where nothing may exist, with the KDF of a
    /// new vault and the same key, and goes on there: the session is no longer read-only and later changes are
    /// saved to the new file. The original file stays byte for byte as it was (RF-15).
    ///
    /// If it fails, the session stays on the file it had, read-only as before.
    ///
    /// While the content is converted the key is held for the work of about three derivations (open the 3.x
    /// bytes, write them as KDBX 4, check the result), not one: a lock asked for meanwhile answers every other
    /// command `Bloqueada` at once all the same, and the secrets are dropped when the conversion step ends.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`]; [`CoreError::ArgumentoInvalido`] if the open vault is already KDBX 4 (there is
    /// nothing to convert); [`CoreError::Ocupada`] while a save is being written; [`CoreError::ArchivoExiste`];
    /// those of [`read_vault`] and [`kdbx::convert_to_kdbx4`]; [`CoreError::GuardadoFallido`].
    pub fn convert(&self, new_path: &Path, kdf: NewVaultKdf) -> CoreResult<Status> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        if !hold(&shared.facts).read_only {
            return Err(CoreError::ArgumentoInvalido);
        }
        let confirmed = shared.retire()?;
        let old_path = hold(&shared.facts).path.clone();

        // As in `save_as`: from here on only the way to find the secrets, never the secrets, across the disk.
        let secrets = Arc::downgrade(&live.secrets);
        let weak = Arc::downgrade(&live);
        drop(live);
        let converted = (|| {
            let (source, _) = read_vault(self.disk.as_ref(), &old_path)?;
            let bytes = {
                let secrets = secrets.upgrade().ok_or(CoreError::Bloqueada)?;
                kdbx::convert_to_kdbx4(&source, secrets.key(), kdf)?
            };
            let for_check = secrets.clone();
            let verify = move |written: &[u8]| {
                let secrets = for_check.upgrade().ok_or(CoreError::Bloqueada)?;
                kdbx::open(written, secrets.key()).map(drop)
            };
            let fingerprint = save_vault_as(self.disk.as_ref(), new_path, &bytes, &verify)
                .map_err(|error| error.code())?;
            let secrets = secrets.upgrade().ok_or(CoreError::Bloqueada)?;
            let vault = kdbx::open(&bytes, secrets.key())?;
            Ok((vault, fingerprint))
        })();

        let live = weak.upgrade().ok_or(CoreError::Bloqueada)?;
        if shared.turn.ended() {
            // The converted file, if it was written, is a complete vault with the same key: it stays.
            return Err(CoreError::Bloqueada);
        }
        match converted {
            Ok((vault, fingerprint)) => {
                let queue = self.start_queue(new_path, fingerprint, &live.secrets)?;
                *hold(&shared.facts) = Facts::of(new_path, &vault);
                *hold(&live.vault) = vault;
                shared.install(queue)?;
                shared.settled(&live);
                Ok(Status::Open(describe(&shared, None)))
            }
            Err(error) => {
                shared.install(self.start_queue(&old_path, confirmed, &live.secrets)?)?;
                Err(error)
            }
        }
    }
}
