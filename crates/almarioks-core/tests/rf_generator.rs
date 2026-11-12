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
    kdbx::{MasterPassword, VaultKey, open},
    strength::{is_weak, password_strength},
    vault::{NewEntry, Timestamp},
};
use secrecy::{ExposeSecret, SecretString};
use std::{fs, path::PathBuf};

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
        Err(CoreError::GeneradorNoDisponible)
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
    // RF-109 (owner's second round): neither case is a failed save; both say the generator is not available.
    assert_eq!(error, CoreError::GeneradorNoDisponible);
    assert_eq!(
        generate_with(&PasswordRules::default(), &mut Broken).unwrap_err(),
        CoreError::GeneradorNoDisponible
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

// ---------- Second round of the owner (2026-11-12): 8 to 500, and RF-109 ----------

#[test]
fn rf_65_rf_68_lengths_8_and_500_generate_and_7_and_501_do_not() {
    assert_eq!((MIN_LENGTH, MAX_LENGTH), (8, 500));
    assert_eq!(PasswordRules::default().length, 20);
    let mut source = Seeded(0x0500);
    for length in [8, 500] {
        let r = rules(length, true, true, true, true, false);
        for password in [
            generate_with(&r, &mut source).unwrap(),
            generate_password(&r).unwrap(),
        ] {
            assert_eq!(password.expose_secret().chars().count(), length);
        }
    }
    for length in [7, 501] {
        let r = rules(length, true, true, true, true, false);
        assert_eq!(
            generate_with(&r, &mut source).unwrap_err(),
            CoreError::ReglasImposibles,
            "length {length}"
        );
        assert_eq!(
            generate_password(&r).unwrap_err(),
            CoreError::ReglasImposibles,
            "length {length}"
        );
    }
}

#[test]
fn rf_65_rf_66_every_subset_of_types_at_8_and_500_with_and_without_lookalikes() {
    let every_type = [LOWERCASE, UPPERCASE, DIGITS, SYMBOLS];
    let mut source = Seeded(0x0850_0000);
    let mut combinations = 0;
    for mask in 1_u8..16 {
        for avoid in [false, true] {
            for length in [8, 500] {
                let r = rules(
                    length,
                    mask & 1 != 0,
                    mask & 2 != 0,
                    mask & 4 != 0,
                    mask & 8 != 0,
                    avoid,
                );
                let chosen = [r.lowercase, r.uppercase, r.digits, r.symbols];
                for _ in 0..25 {
                    let password = generate_with(&r, &mut source).unwrap();
                    let text = password.expose_secret();
                    assert_eq!(text.chars().count(), length, "{r:?}");
                    assert_eq!(text.len(), length, "one byte per character: {r:?}");
                    // A chosen type is there (RF-66) and a type that was not chosen is not (RF-65).
                    for (set, on) in every_type.iter().zip(chosen) {
                        assert_eq!(text.chars().any(|c| set.contains(c)), on, "{r:?}: {set}");
                    }
                    assert!(
                        text.chars()
                            .all(|c| every_type.iter().any(|set| set.contains(c))),
                        "{r:?}"
                    );
                    if avoid {
                        assert!(!text.chars().any(|c| LOOKALIKES.contains(c)), "{r:?}");
                    }
                }
                combinations += 1;
            }
        }
    }
    assert_eq!(combinations, 15 * 2 * 2);
}

/// A source that works for `healthy_fills` calls and is down from then on. It counts every call, so a test can
/// tell whether the generator kept asking after the failure. Calls marked in `all_rejected` answer only the
/// byte 255, which no alphabet accepts: the candidate stays incomplete and the generator has to ask again.
struct FailsAfter {
    inner: Seeded,
    healthy_fills: usize,
    all_rejected: bool,
    calls: usize,
}

impl FailsAfter {
    fn new(healthy_fills: usize, all_rejected: bool) -> Self {
        Self {
            inner: Seeded(0x0109),
            healthy_fills,
            all_rejected,
            calls: 0,
        }
    }
}

impl RandomSource for FailsAfter {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        self.calls += 1;
        if self.calls > self.healthy_fills {
            return Err(CoreError::GeneradorNoDisponible);
        }
        if self.all_rejected {
            buf.fill(255);
            Ok(())
        } else {
            self.inner.fill(buf)
        }
    }
}

#[test]
fn rf_109_a_source_that_goes_down_gives_no_password_its_own_code_and_is_not_asked_again() {
    assert_eq!(
        CoreError::GeneradorNoDisponible.code(),
        "GeneradorNoDisponible"
    );
    assert_eq!(
        CoreError::GeneradorNoDisponible.message(),
        "No se pudo generar la contraseña. Inténtalo otra vez."
    );
    assert_ne!(
        CoreError::GeneradorNoDisponible,
        CoreError::GuardadoFallido,
        "it is not the code of a failed save"
    );

    // Down from the first byte, down with a candidate half made, and down after bytes that were all rejected: the
    // answer is always the error, never a short or padded password, and the failing call is the last one.
    for (healthy_fills, all_rejected) in [(0, false), (1, true), (3, true)] {
        for length in [8, 20, 500] {
            let r = rules(length, true, true, true, true, false);
            let mut source = FailsAfter::new(healthy_fills, all_rejected);
            let result = generate_with(&r, &mut source);
            assert_eq!(
                result.map(|_| ()),
                Err(CoreError::GeneradorNoDisponible),
                "{healthy_fills} healthy calls, length {length}"
            );
            assert_eq!(
                source.calls,
                healthy_fills + 1,
                "the generator kept asking a source that is down"
            );
        }
    }

    // Half a candidate of real characters, then down: nothing of it comes back.
    let r = rules(500, true, true, true, true, false);
    let mut source = FailsAfter::new(1, false);
    assert_eq!(
        generate_with(&r, &mut source).map(|_| ()),
        Err(CoreError::GeneradorNoDisponible),
        "one call of 500 bytes cannot fill 500 characters: 76 of every 256 bytes are rejected"
    );
    assert_eq!(source.calls, 2);

    // A source that is healthy long enough still gives a password: the counter is not what fails.
    let mut source = FailsAfter::new(usize::MAX, false);
    let password = generate_with(&r, &mut source).unwrap();
    assert_eq!(password.expose_secret().chars().count(), 500);
}

#[test]
fn rf_65_a_500_character_password_survives_saving_and_reopening_a_vault_keepass_wrote() {
    const MASTER: &str = "lab-Maestra-2026";
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("keepass2")
        .join("f-pass.kdbx");
    let key = VaultKey::new(Some(MasterPassword::new(MASTER.to_owned())), None).unwrap();
    let mut vault = open(&fs::read(fixture).unwrap(), &key).unwrap();

    let r = rules(MAX_LENGTH, true, true, true, true, false);
    let generated = generate_with(&r, &mut Seeded(0x01F4)).unwrap();
    let text = generated.expose_secret().to_owned();
    assert_eq!(text.chars().count(), 500);

    let root = vault.root_id();
    let id = vault
        .create_entry(
            root,
            NewEntry {
                title: "Generada 500".to_owned(),
                password: Some(SecretString::from(text.clone())),
                ..NewEntry::default()
            },
            Timestamp::from_unix_seconds(1_794_391_200).unwrap(),
        )
        .unwrap();
    // What "show" and "copy" read (`entry_reveal`, `entry_copy`), before and after the disk format.
    assert_eq!(
        vault.reveal_field(id, "Password").unwrap().expose_secret(),
        text
    );
    let saved = vault.to_bytes(&key).unwrap();
    let reopened = open(&saved, &key).unwrap();
    let revealed = reopened.reveal_field(id, "Password").unwrap();
    assert_eq!(revealed.expose_secret().chars().count(), 500);
    assert_eq!(revealed.expose_secret(), text);

    // The same bytes read by the library alone, without the core's wrapper: what another KeePass reader gets.
    let database =
        keepass::Database::parse(&saved, keepass::DatabaseKey::new().with_password(MASTER))
            .unwrap();
    let root = database.root();
    let entry = root.entry_by_name("Generada 500").unwrap();
    assert_eq!(entry.get_password(), Some(text.as_str()));

    // The meter reads it too (RF-70): `zxcvbn` looks at the first 100 characters only.
    assert_eq!(password_strength(&text), 4);
}
