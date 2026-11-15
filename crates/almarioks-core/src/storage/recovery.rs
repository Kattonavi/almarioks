//! The recovery file (RF-33): when locking with changes that could not be saved, the copy that is already encrypted
//! goes to `<vault>.recuperacion-<AAAAMMDD-HHMMSS>.kdbx`, never over another recovery file.
//!
//! It receives only encrypted bytes: by then the vault and the keys were already released (D12). The 5 s limit of
//! RF-33 belongs to the session (T9), which runs this on its own thread.

use std::{
    fmt,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{disk::Disk, save::with_suffix};
use crate::error::{CoreError, CoreResult};

/// How many names are tried (`…-HHMMSS.kdbx`, `…-HHMMSS-2.kdbx`, …) before giving up.
const MAX_RECOVERY_NAMES: u32 = 100;

/// A date and time to the second, for the name of a recovery file. The clock enters as a parameter (plan, "Regla de
/// dependencia"). Decision pending: whether the name uses UTC ([`Timestamp::from_system_time_utc`]) or the person's
/// local time (the shell would build it with [`Timestamp::new`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

const fn is_leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Civil date of a day counted from 1970-01-01 (H. Hinnant, "chrono-compatible low-level date algorithms").
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

impl Timestamp {
    /// A validated date and time (years 1970 to 9999).
    ///
    /// # Errors
    /// [`CoreError::ArgumentoInvalido`] if any field is out of range.
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> CoreResult<Self> {
        let valid = (1970..=9999).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hour < 24
            && minute < 60
            && second < 60;
        if valid {
            Ok(Self {
                year,
                month,
                day,
                hour,
                minute,
                second,
            })
        } else {
            Err(CoreError::ArgumentoInvalido)
        }
    }

    /// The UTC date and time of `time`, to the second.
    ///
    /// # Errors
    /// [`CoreError::ArgumentoInvalido`] before 1970 or after 9999.
    pub fn from_system_time_utc(time: SystemTime) -> CoreResult<Self> {
        let seconds = time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CoreError::ArgumentoInvalido)?
            .as_secs();
        let seconds = i64::try_from(seconds).map_err(|_| CoreError::ArgumentoInvalido)?;
        let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
        let of_day = seconds.rem_euclid(86_400);
        let field = |value: i64| u8::try_from(value).map_err(|_| CoreError::ArgumentoInvalido);
        Self::new(
            u16::try_from(year).map_err(|_| CoreError::ArgumentoInvalido)?,
            field(month)?,
            field(day)?,
            field(of_day / 3_600)?,
            field(of_day % 3_600 / 60)?,
            field(of_day % 60)?,
        )
    }
}

/// `AAAAMMDD-HHMMSS`, as RF-33 names the file.
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Writes `encrypted` next to `vault_path` as `<vault>.recuperacion-<AAAAMMDD-HHMMSS>.kdbx` and returns its path
/// (RF-33). If that name exists it tries `…-2.kdbx`, `…-3.kdbx` and so on: it never replaces a file. It reads the file
/// back and compares it byte for byte; it cannot decrypt it, because it never gets a key (D12).
///
/// # Errors
/// [`CoreError::GuardadoFallido`] if the file cannot be written or does not read back as written (the file it
/// created is removed), or if the first [`MAX_RECOVERY_NAMES`] names are all taken.
pub fn write_recovery(
    disk: &dyn Disk,
    vault_path: &Path,
    encrypted: &[u8],
    now: Timestamp,
) -> CoreResult<PathBuf> {
    for attempt in 1..=MAX_RECOVERY_NAMES {
        let suffix = if attempt == 1 {
            format!(".recuperacion-{now}.kdbx")
        } else {
            format!(".recuperacion-{now}-{attempt}.kdbx")
        };
        let candidate = with_suffix(vault_path, &suffix);
        match disk.write_new_synced(&candidate, encrypted) {
            Ok(()) => {
                if disk.read(&candidate).is_ok_and(|back| back == encrypted) {
                    return Ok(candidate);
                }
                // Only the file this call created.
                drop(disk.remove(&candidate));
                return Err(CoreError::GuardadoFallido);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(_) => return Err(CoreError::GuardadoFallido),
        }
    }
    Err(CoreError::GuardadoFallido)
}
