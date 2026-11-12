//! Password generator (T11, RF-65 to RF-69).
//!
//! Every character comes from a [`RandomSource`] (the operating system's generator in the product, see
//! [`OsRandom`]) and is drawn with the same probability among the allowed characters: one index over the union of
//! the chosen types, bytes rejected above the largest multiple of the alphabet size (no modulo bias, RF-69). If the
//! result lacks a character of some chosen type it is discarded whole and a new one is drawn (RF-66); there is no
//! "place one of each type and fill the rest" step, which would skew the distribution.
//!
//! The password is returned as a [`SecretString`], wiped when dropped. Discarded candidates and byte buffers are
//! wiped too.

use secrecy::SecretString;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{CoreError, CoreResult};

/// Shortest password the generator makes (RF-65).
pub const MIN_LENGTH: usize = 8;
/// Longest password the generator makes (RF-65).
pub const MAX_LENGTH: usize = 128;
/// Length when the person does not choose one (RF-65).
pub const DEFAULT_LENGTH: usize = 20;

/// Lowercase letters (RF-65).
pub const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
/// Uppercase letters (RF-65).
pub const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// Digits (RF-65).
pub const DIGITS: &str = "0123456789";
/// Symbols, exactly the 28 of RF-65.
pub const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.<>?/~|";
/// Characters that "avoid look-alikes" removes (RF-67).
pub const LOOKALIKES: &str = "0Oo1lI|";

/// Most candidates drawn before giving up. With a healthy source the chance of needing even a few is tiny; the
/// bound only stops a broken source (for instance one that always returns the same byte) from looping forever.
const MAX_ATTEMPTS: usize = 10_000;

/// Where the random bytes come from. The product uses [`OsRandom`]; tests inject a fixed-seed source.
pub trait RandomSource {
    /// Fills `buf` with random bytes.
    ///
    /// # Errors
    /// The source could not provide the bytes.
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()>;
}

/// The operating system's random generator (`getrandom`).
#[derive(Debug, Clone, Copy, Default)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        // Same mapping as the key file generator in `kdbx`; the spec has no code for "no randomness available".
        getrandom::fill(buf).map_err(|_| CoreError::GuardadoFallido)
    }
}

/// What the person asked for (RF-65, RF-67).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordRules {
    /// Length in characters, 8 to 128.
    pub length: usize,
    /// Include `a-z`.
    pub lowercase: bool,
    /// Include `A-Z`.
    pub uppercase: bool,
    /// Include `0-9`.
    pub digits: bool,
    /// Include the symbols of [`SYMBOLS`].
    pub symbols: bool,
    /// Leave out `0 O o 1 l I |` (RF-67).
    pub avoid_lookalikes: bool,
}

impl Default for PasswordRules {
    fn default() -> Self {
        Self {
            length: DEFAULT_LENGTH,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            avoid_lookalikes: false,
        }
    }
}

impl PasswordRules {
    /// The allowed characters of each chosen type, look-alikes already removed.
    fn classes(&self) -> Vec<Vec<u8>> {
        let chosen = [
            (self.lowercase, LOWERCASE),
            (self.uppercase, UPPERCASE),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ];
        chosen
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, set)| {
                set.bytes()
                    .filter(|b| !(self.avoid_lookalikes && LOOKALIKES.as_bytes().contains(b)))
                    .collect()
            })
            .collect()
    }
}

/// Generates a password with the operating system's random generator.
///
/// # Errors
/// `ReglasImposibles` if no type is chosen or the length is outside 8 to 128 (RF-68); `GuardadoFallido` if the
/// system generator fails (decisión pendiente: the spec has no code for it).
pub fn generate_password(rules: &PasswordRules) -> CoreResult<SecretString> {
    generate_with(rules, &mut OsRandom)
}

/// Generates a password from an injected source (RF-69).
///
/// # Errors
/// Same as [`generate_password`]; also if the source keeps failing the "one of each type" check
/// ([`MAX_ATTEMPTS`] candidates).
pub fn generate_with(
    rules: &PasswordRules,
    source: &mut impl RandomSource,
) -> CoreResult<SecretString> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&rules.length) {
        return Err(CoreError::ReglasImposibles);
    }
    let classes = rules.classes();
    if classes.is_empty() {
        return Err(CoreError::ReglasImposibles);
    }
    let alphabet: Vec<u8> = classes.iter().flatten().copied().collect();
    let size = alphabet.len();
    // Largest multiple of `size` that fits in a byte: bytes from here up are rejected (no modulo bias).
    let limit = 256 - 256 % size;

    for _ in 0..MAX_ATTEMPTS {
        let mut candidate = String::with_capacity(rules.length);
        let mut bytes = Zeroizing::new(vec![0_u8; rules.length]);
        while candidate.len() < rules.length {
            source.fill(bytes.as_mut_slice()).inspect_err(|_| {
                candidate.zeroize();
            })?;
            for &byte in bytes.iter() {
                if candidate.len() == rules.length {
                    break;
                }
                let index = usize::from(byte);
                if index < limit
                    && let Some(&ch) = alphabet.get(index % size)
                {
                    candidate.push(char::from(ch));
                }
            }
        }
        let has_every_class = classes
            .iter()
            .all(|class| candidate.bytes().any(|b| class.contains(&b)));
        if has_every_class {
            return Ok(SecretString::from(candidate));
        }
        candidate.zeroize();
    }
    Err(CoreError::GuardadoFallido)
}
