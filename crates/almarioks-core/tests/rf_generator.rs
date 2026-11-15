//! Generator and strength through the public API (RF-7, RF-65 to RF-70).
//!
//! The statistical and property tests use a fixed-seed source (SplitMix64), so they are the same on every run.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use almarioks_core::{
    CoreError, CoreResult,
    generator::{
        DIGITS, LOOKALIKES, LOWERCASE, MAX_LENGTH, MIN_LENGTH, OsRandom, PasswordRules,
        RandomSource, SYMBOLS, UPPERCASE, generate_password, generate_with,
    },
    strength::{is_weak, password_strength},
};
use secrecy::ExposeSecret;

/// Deterministic source for tests only (SplitMix64).
struct Seeded(u64);

impl RandomSource for Seeded {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        for chunk in buf.chunks_mut(8) {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            let bytes = z.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Ok(())
    }
}

/// A source that repeats a fixed list of bytes.
struct Script {
    bytes: Vec<u8>,
    next: usize,
}

impl RandomSource for Script {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        for slot in buf {
            *slot = self.bytes[self.next % self.bytes.len()];
            self.next += 1;
        }
        Ok(())
    }
}

/// A source that is down.
struct Broken;

impl RandomSource for Broken {
    fn fill(&mut self, _buf: &mut [u8]) -> CoreResult<()> {
        Err(CoreError::GuardadoFallido)
    }
}

fn rules(
    length: usize,
    lower: bool,
    upper: bool,
    digits: bool,
    symbols: bool,
    avoid: bool,
) -> PasswordRules {
    PasswordRules {
        length,
        lowercase: lower,
        uppercase: upper,
        digits,
        symbols,
        avoid_lookalikes: avoid,
    }
}

fn allowed(rules: &PasswordRules) -> Vec<(bool, Vec<char>)> {
    [
        (rules.lowercase, LOWERCASE),
        (rules.uppercase, UPPERCASE),
        (rules.digits, DIGITS),
        (rules.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(on, set)| {
        let chars = set
            .chars()
            .filter(|c| !(rules.avoid_lookalikes && LOOKALIKES.contains(*c)))
            .collect();
        (on, chars)
    })
    .collect()
}

#[test]
fn rf_65_symbols_are_exactly_the_28_of_the_spec_and_defaults_are_20_and_all_four() {
    assert_eq!(SYMBOLS, "!@#$%^&*()-_=+[]{};:,.<>?/~|");
    assert_eq!(SYMBOLS.chars().count(), 28);
    let defaults = PasswordRules::default();
    assert_eq!(defaults.length, 20);
    assert!(defaults.lowercase && defaults.uppercase && defaults.digits && defaults.symbols);
    assert!(!defaults.avoid_lookalikes);
    let password = generate_with(&defaults, &mut Seeded(1)).unwrap();
    assert_eq!(password.expose_secret().chars().count(), 20);
}

#[test]
fn rf_65_property_exact_length_only_allowed_characters_for_many_rule_combinations() {
    let mut source = Seeded(0x00C0_FFEE);
    let mut combinations = 0;
    for mask in 1_u8..16 {
        for avoid in [false, true] {
            for length in [8, 9, 20, 64, 127, 128] {
                let r = rules(
                    length,
                    mask & 1 != 0,
                    mask & 2 != 0,
                    mask & 4 != 0,
                    mask & 8 != 0,
                    avoid,
                );
                let classes = allowed(&r);
                let all: Vec<char> = classes
                    .iter()
                    .flat_map(|(_, c)| c.iter().copied())
                    .collect();
                for _ in 0..20 {
                    let password = generate_with(&r, &mut source).unwrap();
                    let text = password.expose_secret();
                    assert_eq!(text.chars().count(), length, "{r:?}");
                    assert!(text.chars().all(|c| all.contains(&c)), "{r:?}: {text}");
                    // RF-66: one of each chosen type.
                    for (_, class) in &classes {
                        assert!(text.chars().any(|c| class.contains(&c)), "{r:?}: {text}");
                    }
                }
                combinations += 1;
            }
        }
    }
    assert_eq!(combinations, 15 * 2 * 6);
}

#[test]
fn rf_66_every_chosen_type_is_present_even_at_the_shortest_length() {
    let r = rules(8, true, true, true, true, false);
    let mut source = Seeded(7);
    for _ in 0..2_000 {
        let password = generate_with(&r, &mut source).unwrap();
        let text = password.expose_secret();
        assert!(text.chars().any(|c| LOWERCASE.contains(c)));
        assert!(text.chars().any(|c| UPPERCASE.contains(c)));
        assert!(text.chars().any(|c| DIGITS.contains(c)));
        assert!(text.chars().any(|c| SYMBOLS.contains(c)));
    }
}

#[test]
fn rf_66_and_rf_67_example_5_length_8_four_types_without_lookalikes() {
    let r = rules(8, true, true, true, true, true);
    let mut source = Seeded(2026);
    for _ in 0..500 {
        let password = generate_with(&r, &mut source).unwrap();
        let text = password.expose_secret();
        assert_eq!(text.chars().count(), 8);
        assert!(text.chars().any(|c| c.is_ascii_lowercase()));
        assert!(text.chars().any(|c| c.is_ascii_uppercase()));
        assert!(text.chars().any(|c| c.is_ascii_digit()));
        assert!(text.chars().any(|c| SYMBOLS.contains(c)));
        for banned in ['0', 'O', 'o', '1', 'l', 'I', '|'] {
            assert!(!text.contains(banned), "{banned} in {text}");
        }
    }
}

#[test]
fn rf_67_lookalikes_are_off_by_default_and_the_alphabet_shrinks_from_90_to_83() {
    // Only digits plus a script that would pick every index once: with look-alikes allowed `0` and `1` show up.
    let r = rules(10, false, false, true, false, false);
    let password = generate_with(
        &r,
        &mut Script {
            bytes: (0..10).collect(),
            next: 0,
        },
    )
    .unwrap();
    assert_eq!(password.expose_secret(), "0123456789");
    let r = rules(8, false, false, true, false, true);
    let password = generate_with(
        &r,
        &mut Script {
            bytes: (0..8).collect(),
            next: 0,
        },
    )
    .unwrap();
    assert_eq!(password.expose_secret(), "23456789");
}

#[test]
fn rf_68_no_type_or_length_out_of_range_gives_reglas_imposibles() {
    let mut source = Seeded(3);
    let none = rules(20, false, false, false, false, false);
    assert_eq!(
        generate_with(&none, &mut source).unwrap_err(),
        CoreError::ReglasImposibles
    );
    for length in [0, 1, MIN_LENGTH - 1, MAX_LENGTH + 1, 1_000, usize::MAX] {
        let r = rules(length, true, true, true, true, false);
        let error = generate_with(&r, &mut source).unwrap_err();
        assert_eq!(error, CoreError::ReglasImposibles, "length {length}");
        assert_eq!(error.code(), "ReglasImposibles");
    }
    for length in [MIN_LENGTH, MAX_LENGTH] {
        assert!(
            generate_with(
                &rules(length, true, false, false, false, false),
                &mut source
            )
            .is_ok()
        );
    }
}

#[test]
fn rf_69_no_modulo_bias_high_bytes_are_rejected() {
    // 26 letters: the limit is 234, so 255, 254... are discarded. A biased generator would turn 255 into 'v'.
    let r = rules(8, true, false, false, false, false);
    let bytes = vec![
        255, 0, 254, 1, 253, 2, 252, 3, 251, 4, 250, 5, 249, 6, 248, 7,
    ];
    let password = generate_with(&r, &mut Script { bytes, next: 0 }).unwrap();
    assert_eq!(password.expose_secret(), "abcdefgh");
    // The edge: 233 is the last accepted byte (233 % 26 = 25 = 'z'), 234 is the first rejected.
    let bytes = vec![233, 234, 0, 1, 2, 3, 4, 5, 6];
    let password = generate_with(&r, &mut Script { bytes, next: 0 }).unwrap();
    assert_eq!(password.expose_secret(), "zabcdefg");
}

#[test]
fn rf_69_each_allowed_character_appears_with_a_frequency_within_20_percent_of_uniform() {
    let r = rules(100, true, true, true, true, false);
    let mut source = Seeded(0x5EED_5EED);
    let mut counts = std::collections::BTreeMap::<char, u64>::new();
    let passwords: u64 = 1_000;
    for _ in 0..passwords {
        let password = generate_with(&r, &mut source).unwrap();
        for c in password.expose_secret().chars() {
            *counts.entry(c).or_default() += 1;
        }
    }
    let total: u64 = 100 * passwords;
    assert_eq!(total, 100_000);
    let alphabet: Vec<char> = [LOWERCASE, UPPERCASE, DIGITS, SYMBOLS]
        .iter()
        .flat_map(|s| s.chars())
        .collect();
    let n = u64::try_from(alphabet.len()).unwrap();
    assert_eq!(n, 90);
    assert_eq!(
        counts.len(),
        alphabet.len(),
        "a character never appeared or one outside the alphabet did"
    );
    for c in &alphabet {
        let seen = counts[c];
        // Within +-20 % of total / n, in integers: 0.8 * total <= seen * n <= 1.2 * total.
        assert!(
            4 * total <= 5 * seen * n && 5 * seen * n <= 6 * total,
            "{c}: {seen} of {total}"
        );
    }
}

#[test]
fn rf_69_a_discarded_candidate_is_replaced_whole_and_a_dead_source_fails() {
    // A source of zeros only gives 'a': the first candidate lacks the uppercase letter and so does every other one,
    // so the generator gives up instead of looping forever.
    let r = rules(8, true, true, false, false, false);
    let error = generate_with(
        &r,
        &mut Script {
            bytes: vec![0],
            next: 0,
        },
    )
    .unwrap_err();
    assert_eq!(error, CoreError::GuardadoFallido);
    assert_eq!(
        generate_with(&PasswordRules::default(), &mut Broken).unwrap_err(),
        CoreError::GuardadoFallido
    );
}

#[test]
fn rf_69_the_operating_system_source_makes_different_valid_passwords() {
    let r = PasswordRules::default();
    let a = generate_password(&r).unwrap();
    let b = generate_password(&r).unwrap();
    assert_ne!(a.expose_secret(), b.expose_secret());
    assert_eq!(a.expose_secret().chars().count(), 20);
    OsRandom.fill(&mut [0_u8; 16]).unwrap();
}

#[test]
fn rf_07_and_rf_70_strength_anchors() {
    assert_eq!(password_strength("123456"), 0);
    let r = PasswordRules::default();
    let password = generate_with(&r, &mut Seeded(11)).unwrap();
    assert_eq!(password_strength(password.expose_secret()), 4);
    assert!(is_weak(0) && is_weak(2));
    assert!(!is_weak(3) && !is_weak(4));
}

#[test]
fn rf_70_generated_passwords_do_not_show_their_text_in_debug() {
    let password = generate_with(&PasswordRules::default(), &mut Seeded(5)).unwrap();
    let shown = format!("{password:?}");
    assert!(!shown.contains(password.expose_secret()));
}
