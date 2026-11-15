//! Password strength (T11, RF-7, RF-70): the 0 to 4 score of `zxcvbn`, as it is.
//!
//! It is an estimator, not cryptography. The text is neither stored nor logged.

/// Highest score.
pub const MAX_SCORE: u8 = 4;

/// Below this score the master password is "weak" and the window warns without blocking (RF-7).
pub const WEAK_BELOW: u8 = 3;

/// The `zxcvbn` score of `password`, from 0 (guessable at once) to 4 (very hard to guess).
#[must_use]
pub fn password_strength(password: &str) -> u8 {
    u8::from(zxcvbn::zxcvbn(password, &[]).score())
}

/// Whether the score is under [`WEAK_BELOW`] (RF-7).
#[must_use]
pub const fn is_weak(score: u8) -> bool {
    score < WEAK_BELOW
}
