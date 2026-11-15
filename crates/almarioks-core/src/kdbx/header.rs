//! Our own reader of the KDBX outer header (RF-13, RF-16, RF-17, RF-94).
//!
//! Runs BEFORE `keepass` sees the file (lab `docs/lab/boveda-0.1`, P4 a). It is not cryptography: it walks the
//! type-length-value header and the KDF `VariantDictionary`, and never derives nor decrypts. It exists because
//! `keepass` 0.17.0:
//! - derives the key in the same call that reads the header, with whatever memory and iterations the file asks
//!   for (a hostile file reserves gigabytes or runs for hours) — RF-16;
//! - truncates those values with `as u32` before deriving, so limits must be compared in `u64` first;
//! - keeps the LAST occurrence of a repeated KDF field, so every occurrence is checked;
//! - panics on fixed-size fields that are too short (it reads them with `byteorder`): the compression flags
//!   (field 3), the KDBX 3 transform rounds (6) and inner stream id (10), and every typed value of a
//!   `VariantDictionary` (KDF parameters, field 11, and public custom data, field 12) — RF-94.

use std::{collections::HashMap, fmt};

use crate::error::{CoreError, CoreResult};

/// Most memory a vault may ask Argon2 for (RF-16): 1 GiB.
pub const MAX_KDF_MEMORY_BYTES: u64 = 1 << 30;
/// Most Argon2 iterations a vault may ask for (RF-16).
pub const MAX_ARGON2_ITERATIONS: u64 = 100;
/// Most Argon2 lanes a vault may ask for (RF-16).
pub const MAX_ARGON2_PARALLELISM: u64 = 16;
/// Most AES-KDF rounds a vault may ask for (RF-16).
pub const MAX_AES_KDF_ROUNDS: u64 = 100_000_000;
/// Below this Argon2 memory the vault opens with a weak-KDF warning (RF-17): 64 MiB.
pub const STRONG_KDF_MIN_MEMORY_BYTES: u64 = 64 << 20;

const SIGNATURE_1: [u8; 4] = [0x03, 0xd9, 0xa2, 0x9a];
const SIGNATURE_2_KDB: u32 = 0xb54b_fb65;
const SIGNATURE_2_KDBX_PRERELEASE: u32 = 0xb54b_fb66;
const SIGNATURE_2_KDBX: u32 = 0xb54b_fb67;
const VERSION_HEADER_LEN: usize = 12;

// Outer header fields of a fixed size (same numbers in KDBX 3 and 4) and the sizes the format gives them.
const FIELD_CIPHER_ID: u8 = 2;
const FIELD_MASTER_SEED: u8 = 4;
const FIELD_TRANSFORM_SEED: u8 = 5;
const FIELD_ENCRYPTION_IV: u8 = 7;
const CIPHER_ID_LEN: usize = 16;
const SEED_LEN: usize = 32;
const CHACHA20_IV_LEN: usize = 12;
const BLOCK_CIPHER_IV_LEN: usize = 16;
const CIPHER_AES256: [u8; 16] = [
    0x31, 0xc1, 0xf2, 0xe6, 0xbf, 0x71, 0x43, 0x50, 0xbe, 0x58, 0x05, 0x21, 0x6a, 0xfc, 0x5a, 0xff,
];
const CIPHER_TWOFISH: [u8; 16] = [
    0xad, 0x68, 0xf2, 0x9f, 0x57, 0x6f, 0x4b, 0xb9, 0xa3, 0x6a, 0xd4, 0x7a, 0xf9, 0x65, 0x34, 0x6c,
];
const CIPHER_CHACHA20: [u8; 16] = [
    0xd6, 0x03, 0x8a, 0x2b, 0x8b, 0x6f, 0x4c, 0xb5, 0xa5, 0x24, 0x33, 0x9a, 0x31, 0xdb, 0xb5, 0x9a,
];

pub(crate) const KDF_ARGON2D: [u8; 16] = [
    0xef, 0x63, 0x6d, 0xdf, 0x8c, 0x29, 0x44, 0x4b, 0x91, 0xf7, 0xa9, 0xa4, 0x03, 0xe3, 0x0a, 0x0c,
];
pub(crate) const KDF_ARGON2ID: [u8; 16] = [
    0x9e, 0x29, 0x8b, 0x19, 0x56, 0xdb, 0x47, 0x73, 0xb2, 0x3d, 0xfc, 0x3e, 0xc6, 0xf0, 0xa1, 0xe6,
];
const KDF_AES_KDBX4: [u8; 16] = [
    0x7c, 0x02, 0xbb, 0x82, 0x79, 0xa7, 0x4a, 0xc0, 0x92, 0x7d, 0x11, 0x4a, 0x00, 0x64, 0x82, 0x38,
];
const KDF_AES_KDBX3: [u8; 16] = [
    0xc9, 0xd9, 0xf3, 0x9a, 0x62, 0x8a, 0x44, 0x60, 0xbf, 0x74, 0x0d, 0x08, 0xc1, 0x8a, 0x4f, 0xea,
];

/// Version of the file format, as written in the header (`KDBX 4.1` is `{ major: 4, minor: 1 }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatVersion {
    /// 3 or 4: the only majors this app opens.
    pub major: u16,
    /// Minor version (4.0, 4.1, 3.1…).
    pub minor: u16,
}

impl fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KDBX {}.{}", self.major, self.minor)
    }
}

/// Argon2 flavour of a vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Argon2Variant {
    /// Argon2d (KeePass' older default).
    D,
    /// Argon2id (what AlmarioKS writes, RF-2).
    Id,
}

/// Key derivation parameters written in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KdfParams {
    /// Argon2d or Argon2id.
    Argon2 {
        /// d or id.
        variant: Argon2Variant,
        /// Memory in bytes, as stored in the file.
        memory_bytes: u64,
        /// Passes over the memory.
        iterations: u64,
        /// Lanes.
        parallelism: u32,
    },
    /// AES-KDF (KDBX 3.x, or KDBX 4 written with it).
    AesKdf {
        /// Rounds of AES.
        rounds: u64,
    },
}

impl KdfParams {
    /// True when the vault resists brute force less than a new AlmarioKS vault (RF-17): any KDF that is not
    /// Argon2id, or Argon2id with less than 64 MiB.
    #[must_use]
    pub fn is_weak(&self) -> bool {
        !matches!(
            *self,
            Self::Argon2 { variant: Argon2Variant::Id, memory_bytes, .. }
                if memory_bytes >= STRONG_KDF_MIN_MEMORY_BYTES
        )
    }

    fn check_limits(&self) -> CoreResult<()> {
        let excessive = match *self {
            Self::Argon2 {
                memory_bytes,
                iterations,
                parallelism,
                ..
            } => {
                memory_bytes > MAX_KDF_MEMORY_BYTES
                    || iterations > MAX_ARGON2_ITERATIONS
                    || u64::from(parallelism) > MAX_ARGON2_PARALLELISM
            }
            Self::AesKdf { rounds } => rounds > MAX_AES_KDF_ROUNDS,
        };
        if excessive {
            Err(CoreError::ParametrosKdfExcesivos)
        } else {
            Ok(())
        }
    }
}

/// What the header says, read without deriving any key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeaderInfo {
    /// File format version.
    pub version: FormatVersion,
    /// KDF the library will use (the last occurrence, like `keepass`).
    pub kdf: KdfParams,
}

impl HeaderInfo {
    /// KDBX 3.x opens read-only (RF-14, ADR-0001).
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.version.major == 3
    }

    /// See [`KdfParams::is_weak`] (RF-17).
    #[must_use]
    pub fn is_kdf_weak(&self) -> bool {
        self.kdf.is_weak()
    }
}

/// The header plus the offsets the tests use to forge files (a hostile header with a recomputed hash).
#[derive(Debug)]
pub(crate) struct ParsedHeader {
    pub(crate) info: HeaderInfo,
    /// Offset just after the END field: in KDBX 4 the header SHA-256 starts here.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "used by the module tests to forge headers")
    )]
    pub(crate) end: usize,
    /// Absolute offset of the value of each KDF `VariantDictionary` item (KDBX 4), last occurrence.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "used by the module tests to forge headers")
    )]
    pub(crate) kdf_item_offsets: HashMap<String, usize>,
}

/// Reads the version and the KDF parameters of a vault without deriving its key, and rejects what would make
/// the library panic or exhaust the machine.
///
/// # Errors
/// - [`CoreError::FormatoNoSoportado`]: KDB (KeePass 1.x), KDBX 2.x, or a major version other than 3 and 4.
/// - [`CoreError::ArchivoDanado`]: not a KDBX file, truncated header, a field of the wrong size, a malformed
///   `VariantDictionary`, or KDF parameters that are missing or unknown.
/// - [`CoreError::ParametrosKdfExcesivos`]: any occurrence of the KDF asks for more than the limits of RF-16.
pub fn read_header(bytes: &[u8]) -> CoreResult<HeaderInfo> {
    parse(bytes).map(|parsed| parsed.info)
}

pub(crate) fn parse(data: &[u8]) -> CoreResult<ParsedHeader> {
    if take(data, 0, 4)? != SIGNATURE_1 {
        return Err(CoreError::ArchivoDanado);
    }
    let major = match u32_at(data, 4)? {
        SIGNATURE_2_KDB | SIGNATURE_2_KDBX_PRERELEASE => return Err(CoreError::FormatoNoSoportado),
        SIGNATURE_2_KDBX => {
            let major = u16_at(data, 10)?;
            if major != 3 && major != 4 {
                return Err(CoreError::FormatoNoSoportado);
            }
            major
        }
        _ => return Err(CoreError::ArchivoDanado),
    };
    let version = FormatVersion {
        major,
        minor: u16_at(data, 8)?,
    };

    let mut pos = VERSION_HEADER_LEN;
    let mut kdfs = Vec::new();
    let mut kdf_item_offsets = HashMap::new();
    let mut cipher: Option<&[u8]> = None;
    let mut iv_len: Option<usize> = None;
    loop {
        let field = *data.get(pos).ok_or(CoreError::ArchivoDanado)?;
        let (len, value_pos) = if major == 4 {
            (to_usize(u32_at(data, add(pos, 1)?)?)?, add(pos, 5)?)
        } else {
            (usize::from(u16_at(data, add(pos, 1)?)?), add(pos, 3)?)
        };
        let value = take(data, value_pos, len)?;
        pos = add(value_pos, len)?;
        match (major, field) {
            (_, 0) => break,
            (_, 3) | (3, 10) if len != 4 => return Err(CoreError::ArchivoDanado),
            // RF-94: fields of a fixed size. The library does not panic on these, but with a seed or an IV of
            // the wrong size it derives anyway and answers "wrong key" for a file that is simply damaged.
            (_, FIELD_CIPHER_ID) => {
                if len != CIPHER_ID_LEN {
                    return Err(CoreError::ArchivoDanado);
                }
                cipher = Some(value);
            }
            (_, FIELD_MASTER_SEED) | (3, FIELD_TRANSFORM_SEED) if len != SEED_LEN => {
                return Err(CoreError::ArchivoDanado);
            }
            (_, FIELD_ENCRYPTION_IV) => iv_len = Some(len),
            (3, 6) => kdfs.push(KdfParams::AesKdf {
                rounds: u64_of(value)?,
            }),
            (4, 11) => {
                kdf_item_offsets.clear();
                let items = walk_dictionary(value, value_pos, &mut kdf_item_offsets)?;
                kdfs.push(kdf_from_items(&items)?);
            }
            (4, 12) => {
                walk_dictionary(value, value_pos, &mut HashMap::new())?;
            }
            _ => {}
        }
    }
    // The IV must have the size its cipher needs; an unknown cipher is the library's to reject.
    if let (Some(cipher), Some(iv_len)) = (cipher, iv_len) {
        let expected = if cipher == CIPHER_CHACHA20 {
            Some(CHACHA20_IV_LEN)
        } else if cipher == CIPHER_AES256 || cipher == CIPHER_TWOFISH {
            Some(BLOCK_CIPHER_IV_LEN)
        } else {
            None
        };
        if expected.is_some_and(|expected| expected != iv_len) {
            return Err(CoreError::ArchivoDanado);
        }
    }
    // Every occurrence is checked: the library keeps the last one if the field repeats.
    for kdf in &kdfs {
        kdf.check_limits()?;
    }
    let kdf = *kdfs.last().ok_or(CoreError::ArchivoDanado)?;
    Ok(ParsedHeader {
        info: HeaderInfo { version, kdf },
        end: pos,
        kdf_item_offsets,
    })
}

fn kdf_from_items(items: &HashMap<String, &[u8]>) -> CoreResult<KdfParams> {
    let get = |key: &str| items.get(key).copied().ok_or(CoreError::ArchivoDanado);
    let uuid = get("$UUID")?;
    if uuid == KDF_ARGON2D || uuid == KDF_ARGON2ID {
        Ok(KdfParams::Argon2 {
            variant: if uuid == KDF_ARGON2ID {
                Argon2Variant::Id
            } else {
                Argon2Variant::D
            },
            memory_bytes: u64_of(get("M")?)?,
            iterations: u64_of(get("I")?)?,
            parallelism: u32_of(get("P")?)?,
        })
    } else if uuid == KDF_AES_KDBX4 || uuid == KDF_AES_KDBX3 {
        Ok(KdfParams::AesKdf {
            rounds: u64_of(get("R")?)?,
        })
    } else {
        // Unknown KDF: the library cannot open it either.
        Err(CoreError::ArchivoDanado)
    }
}

/// Walks a `VariantDictionary` and checks the size of every typed value (the library would panic on a short
/// one). Returns the items by key; a repeated key keeps the last value, like the library.
fn walk_dictionary<'a>(
    dict: &'a [u8],
    base: usize,
    offsets: &mut HashMap<String, usize>,
) -> CoreResult<HashMap<String, &'a [u8]>> {
    if u16_at(dict, 0)? & 0xff00 != 0x0100 {
        return Err(CoreError::ArchivoDanado);
    }
    let mut pos = 2;
    let mut items = HashMap::new();
    loop {
        let kind = *dict.get(pos).ok_or(CoreError::ArchivoDanado)?;
        if kind == 0 {
            break;
        }
        let key_len = to_usize(u32_at(dict, add(pos, 1)?)?)?;
        let key = take(dict, add(pos, 5)?, key_len)?;
        let value_len_pos = add(add(pos, 5)?, key_len)?;
        let value_len = to_usize(u32_at(dict, value_len_pos)?)?;
        let value_pos = add(value_len_pos, 4)?;
        let value = take(dict, value_pos, value_len)?;
        let expected = match kind {
            0x04 | 0x0c => Some(4),
            0x05 | 0x0d => Some(8),
            0x08 => Some(1),
            0x18 | 0x42 => None,
            _ => return Err(CoreError::ArchivoDanado),
        };
        if expected.is_some_and(|size| size != value_len) {
            return Err(CoreError::ArchivoDanado);
        }
        let name = String::from_utf8_lossy(key).into_owned();
        offsets.insert(name.clone(), add(base, value_pos)?);
        items.insert(name, value);
        pos = add(value_pos, value_len)?;
    }
    Ok(items)
}

fn add(a: usize, b: usize) -> CoreResult<usize> {
    a.checked_add(b).ok_or(CoreError::ArchivoDanado)
}

fn to_usize(value: u32) -> CoreResult<usize> {
    usize::try_from(value).map_err(|_| CoreError::ArchivoDanado)
}

fn take(data: &[u8], pos: usize, len: usize) -> CoreResult<&[u8]> {
    data.get(pos..add(pos, len)?)
        .ok_or(CoreError::ArchivoDanado)
}

fn u16_at(data: &[u8], pos: usize) -> CoreResult<u16> {
    let bytes = take(data, pos, 2)?
        .try_into()
        .map_err(|_| CoreError::ArchivoDanado)?;
    Ok(u16::from_le_bytes(bytes))
}

fn u32_at(data: &[u8], pos: usize) -> CoreResult<u32> {
    u32_of(take(data, pos, 4)?)
}

fn u32_of(bytes: &[u8]) -> CoreResult<u32> {
    let bytes = bytes.try_into().map_err(|_| CoreError::ArchivoDanado)?;
    Ok(u32::from_le_bytes(bytes))
}

fn u64_of(bytes: &[u8]) -> CoreResult<u64> {
    let bytes = bytes.try_into().map_err(|_| CoreError::ArchivoDanado)?;
    Ok(u64::from_le_bytes(bytes))
}
