//! Independent validation from the spec: generator (RF-65 to RF-69; example 5), strength anchors ("Fortaleza",
//! RF-7) and settings (RF-19, RF-29, RF-73, RF-82). Character sets are copied from the spec text, not the code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::collections::BTreeMap;

use almarioks_core::{
    CoreError, CoreResult,
    generator::{OsRandom, PasswordRules, RandomSource, generate_password, generate_with},
    settings::Settings,
    strength::{is_weak, password_strength},
};
use secrecy::ExposeSecret;
use spec_support::TempDir;

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.<>?/~|";
const LOOKALIKES: &str = "0Oo1lI|";

/// SplitMix64 with a fixed seed.
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
            chunk.copy_from_slice(bytes.get(..chunk.len()).expect("len"));
        }
        Ok(())
    }
}

struct Constant(u8);

impl RandomSource for Constant {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        buf.fill(self.0);
        Ok(())
    }
}

struct Broken;

impl RandomSource for Broken {
    fn fill(&mut self, _buf: &mut [u8]) -> CoreResult<()> {
        Err(CoreError::GuardadoFallido)
    }
}

fn rules(length: usize, types: [bool; 4], avoid: bool) -> PasswordRules {
    let [lowercase, uppercase, digits, symbols] = types;
    PasswordRules {
        length,
        lowercase,
        uppercase,
        digits,
        symbols,
        avoid_lookalikes: avoid,
    }
}

fn has_any(password: &str, set: &str) -> bool {
    password.chars().any(|c| set.contains(c))
}

/// Checks RF-65 to RF-67 for one password.
fn check(password: &str, r: &PasswordRules) {
    assert_eq!(
        password.chars().count(),
        r.length,
        "RF-65 length: {password:?}"
    );
    let sets = [
        (r.lowercase, LOWER),
        (r.uppercase, UPPER),
        (r.digits, DIGITS),
        (r.symbols, SYMBOLS),
    ];
    for (chosen, set) in sets {
        assert_eq!(
            has_any(password, set),
            chosen,
            "RF-66 / RF-65 type {set:?} in {password:?}"
        );
    }
    for c in password.chars() {
        assert!(
            sets.iter().any(|(chosen, set)| *chosen && set.contains(c)),
            "{c:?} is not allowed"
        );
        if r.avoid_lookalikes {
            assert!(!LOOKALIKES.contains(c), "RF-67: {c:?} in {password:?}");
        }
    }
}

#[test]
fn spec_rf_66_rf_67_example_5_length_8_four_types_avoiding_lookalikes() {
    let r = rules(8, [true; 4], true);
    let mut source = Seeded(5);
    for _ in 0..500 {
        let password = generate_with(&r, &mut source).expect("generate");
        check(password.expose_secret(), &r);
    }
    check(
        generate_password(&r).expect("OS generator").expose_secret(),
        &r,
    );
}

#[test]
fn spec_rf_65_defaults_are_20_characters_of_the_four_types() {
    let r = PasswordRules::default();
    assert_eq!(r, rules(20, [true; 4], false));
    let mut source = OsRandom;
    check(
        generate_with(&r, &mut source)
            .expect("generate")
            .expose_secret(),
        &r,
    );
}

#[test]
fn spec_rf_65_rf_66_every_subset_of_types_at_both_length_limits() {
    let mut source = Seeded(65);
    for mask in 1_u8..16 {
        let types = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
        for length in [8, 128] {
            for avoid in [false, true] {
                let r = rules(length, types, avoid);
                check(
                    generate_with(&r, &mut source)
                        .expect("generate")
                        .expose_secret(),
                    &r,
                );
            }
        }
    }
}

#[test]
fn spec_rf_68_no_type_or_length_outside_8_to_128_is_reglas_imposibles() {
    let mut source = Seeded(68);
    let impossible = |r: PasswordRules, source: &mut Seeded| generate_with(&r, source).map(|_| ());
    assert_eq!(
        impossible(rules(20, [false; 4], false), &mut source),
        Err(CoreError::ReglasImposibles)
    );
    for length in [0, 7, 129, usize::MAX] {
        assert_eq!(
            impossible(rules(length, [true; 4], false), &mut source),
            Err(CoreError::ReglasImposibles)
        );
    }
    assert_eq!(
        generate_password(&rules(7, [true; 4], false)).map(|_| ()),
        Err(CoreError::ReglasImposibles)
    );
}

#[test]
fn spec_rf_69_every_allowed_character_is_equally_likely_within_20_percent() {
    let r = PasswordRules::default();
    let alphabet: Vec<char> = format!("{LOWER}{UPPER}{DIGITS}{SYMBOLS}").chars().collect();
    let mut counts: BTreeMap<char, usize> = alphabet.iter().map(|c| (*c, 0)).collect();
    let mut source = Seeded(2026);
    let mut total = 0_usize;
    while total < 100_000 {
        let password = generate_with(&r, &mut source).expect("generate");
        for c in password.expose_secret().chars() {
            *counts.get_mut(&c).expect("allowed character") += 1;
            total += 1;
        }
    }
    let n = alphabet.len();
    for (c, count) in counts {
        // |count - total/n| <= 20 % of total/n, in integers.
        let scaled = count * n * 10;
        assert!(
            scaled >= total * 8 && scaled <= total * 12,
            "RF-69: {c:?} appeared {count} of {total}"
        );
    }
}

#[test]
fn spec_rf_69_a_source_that_can_never_satisfy_one_of_each_type_gives_no_password() {
    let r = PasswordRules::default();
    match generate_with(&r, &mut Constant(0)) {
        Ok(password) => check(password.expose_secret(), &r),
        Err(error) => assert_ne!(error, CoreError::Bloqueada),
    }
    assert!(
        generate_with(&r, &mut Broken).is_err(),
        "a broken source never yields a password"
    );
}

#[test]
fn spec_fortaleza_anchors_and_rf_7_weak_below_3() {
    assert_eq!(password_strength("123456"), 0);
    assert_eq!(
        password_strength("q7#Vp!2xL9@mZr$4Tb&K"),
        4,
        "20 random characters of the four types"
    );
    let generated = generate_password(&PasswordRules::default()).expect("generate");
    assert_eq!(password_strength(generated.expose_secret()), 4);
    assert!(is_weak(0) && is_weak(2));
    assert!(!is_weak(3) && !is_weak(4));
}

// ----- Settings -----

fn parsed(json: &str) -> (u32, u32) {
    let s = Settings::from_bytes(json.as_bytes());
    (s.auto_lock_minutes(), s.clipboard_seconds())
}

#[test]
fn spec_rf_82_damaged_settings_fall_back_to_defaults_field_by_field() {
    let defaults = Settings::default();
    assert_eq!(
        (defaults.auto_lock_minutes(), defaults.clipboard_seconds()),
        (5, 20),
        "D1, D2"
    );
    assert!(defaults.recent_vaults().is_empty());
    for broken in [
        &b""[..],
        b"no es json",
        b"\xFF\xFE\x00",
        b"[]",
        b"null",
        b"{\"auto_lock_minutes\":",
    ] {
        assert_eq!(Settings::from_bytes(broken), defaults, "{broken:?}");
    }
    assert_eq!(
        parsed(r#"{"auto_lock_minutes": 15, "clipboard_seconds": 120}"#),
        (15, 120)
    );
    assert_eq!(
        parsed(r#"{"auto_lock_minutes": 7, "clipboard_seconds": 9}"#),
        (5, 20)
    );
    assert_eq!(
        parsed(r#"{"auto_lock_minutes": 0}"#),
        (5, 20),
        "there is no \"never\""
    );
    assert_eq!(
        parsed(r#"{"auto_lock_minutes": "15", "clipboard_seconds": 10}"#),
        (5, 10)
    );
    assert_eq!(
        parsed(r#"{"auto_lock_minutes": 30, "clipboard_seconds": -20}"#),
        (30, 20)
    );
    assert_eq!(parsed(r#"{"clipboard_seconds": 10.5}"#), (5, 20));
    assert_eq!(
        parsed(r#"{"clipboard_seconds": 121, "auto_lock_minutes": 1}"#),
        (1, 20)
    );
}

#[test]
fn spec_rf_29_rf_73_only_the_allowed_values_can_be_set() {
    let mut s = Settings::default();
    for minutes in [1, 5, 15, 30] {
        assert_eq!(s.set_auto_lock_minutes(minutes), Ok(()));
    }
    for minutes in [0, 2, 31, u32::MAX] {
        assert_eq!(
            s.set_auto_lock_minutes(minutes),
            Err(CoreError::ArgumentoInvalido)
        );
    }
    assert_eq!(s.set_clipboard_seconds(10), Ok(()));
    assert_eq!(s.set_clipboard_seconds(120), Ok(()));
    assert_eq!(
        s.set_clipboard_seconds(9),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!(
        s.set_clipboard_seconds(121),
        Err(CoreError::ArgumentoInvalido)
    );
    assert_eq!((s.auto_lock_minutes(), s.clipboard_seconds()), (30, 120));
}

#[test]
fn spec_rf_19_remembers_the_last_5_vaults_most_recent_first_and_survives_a_round_trip() {
    let dir = TempDir::new("rf19");
    let mut s = Settings::default();
    for n in 1..=6 {
        s.remember_vault(&dir.join(&format!("b{n}.kdbx")));
    }
    s.remember_vault(&dir.join("b3.kdbx"));
    let expected: Vec<_> = ["b3", "b6", "b5", "b4", "b2"]
        .iter()
        .map(|n| dir.join(&format!("{n}.kdbx")))
        .collect();
    assert_eq!(s.recent_vaults(), expected.as_slice());
    let file = dir.join("ajustes.json");
    s.save(&file).expect("save");
    assert_eq!(Settings::load(&file), s);
    assert_eq!(Settings::from_bytes(s.to_text().as_bytes()), s);
    assert_eq!(
        Settings::load(&dir.join("no-existe.json")),
        Settings::default(),
        "RF-82"
    );
}
