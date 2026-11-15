//! Statistical quality of the generator (RF-66, RF-67, RF-69; second round of the owner, points 4a and 4b).
//!
//! Two checks, each a function that takes ANY generator, so it can be run against a rigged one:
//! - [`check_uniformity`] (4a): every allowed character comes out as often as the others, within a tolerance.
//! - [`check_placement`] (4b): the same at the first and at the last position, each on its own.
//!
//! The mutation tests at the end prove the checks can fail: they reject a generator that gives one character 3 %
//! more than its share, and one that always starts with a lowercase letter.
//!
//! Everything runs on a fixed-seed source (SplitMix64, the same the other generator tests use), so a run is the
//! same on every machine: these tests cannot fail by chance. Passwords are 100 characters or longer: only there is
//! the "one of each type" discard of RF-66 negligible (a candidate of 100 characters lacks a digit once in 130 000).
//! At 8 characters digits do come out more often, and that is correct: what is uniform is the set of valid
//! passwords, not each character.
//!
//! # How the tolerances are chosen (integers only: the workspace warns on float arithmetic)
//! With N draws over an alphabet of n characters, the count of one character is Binomial(N, 1/n): mean N/n and
//! variance N·(n − 1)/n². A tolerance of T % of the mean is t = T·N/(100·n). It is at least 6 standard deviations
//! when t² ≥ 36·variance, that is:
//!
//! ```text
//! T² · N ≥ 360 000 · (n − 1)
//! ```
//!
//! - 4a, all four types (90 characters, and 83 avoiding look-alikes), T = 2: N = 100 000·n, that is 100 000
//!   expected per character, gives 400 000·n ≥ 360 000·(n − 1) for every n: about 6.3 deviations. For the 90
//!   characters that is the 9 000 000 of the owner's brief.
//! - 4a, the other 28 alphabets (each subset of types, with and without look-alikes): 100 000 per character on
//!   all of them is 121 000 000 characters and took 16 s of the 31 s this file added to the suite (measured in
//!   the test profile). They run with N = 40 000·n, and their tolerance is not chosen by hand:
//!   [`percent_for_six_deviations`] gives the smallest whole T that keeps 6 deviations for that N and n, which
//!   is 3 (9·40 000·n ≥ 360 000·(n − 1)). ±3 % cannot see a 3 % rigging; the mutation test runs on the 90.
//! - 4b, T = 15: each password gives ONE draw per position, so N is the number of passwords. 225·N ≥ 360 000·89
//!   needs N ≥ 142 400; the tests draw 150 000 (6.2 deviations).
//!
//! [`skew`] refuses to run with fewer draws than that, so nobody can tighten a tolerance below its sample.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use almarioks_core::{
    CoreResult,
    generator::{
        DIGITS, LOOKALIKES, LOWERCASE, PasswordRules, RandomSource, SYMBOLS, UPPERCASE,
        generate_with,
    },
};
use secrecy::{ExposeSecret, SecretString};

/// Deterministic source for tests only (SplitMix64).
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl RandomSource for Seeded {
    fn fill(&mut self, buf: &mut [u8]) -> CoreResult<()> {
        for chunk in buf.chunks_mut(8) {
            for (slot, byte) in chunk.iter_mut().zip(self.next().to_le_bytes()) {
                *slot = byte;
            }
        }
        Ok(())
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

/// The characters RF-65 and RF-67 allow for these rules.
fn alphabet(rules: &PasswordRules) -> Vec<u8> {
    [
        (rules.lowercase, LOWERCASE),
        (rules.uppercase, UPPERCASE),
        (rules.digits, DIGITS),
        (rules.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .flat_map(|(_, set)| set.bytes())
    .filter(|b| !(rules.avoid_lookalikes && LOOKALIKES.as_bytes().contains(b)))
    .collect()
}

fn count(items: usize) -> u64 {
    u64::try_from(items).unwrap()
}

// ---------- The sample ----------

/// How many times each byte was seen.
struct Tally {
    seen: [u64; 256],
    total: u64,
}

impl Tally {
    const fn new() -> Self {
        Self {
            seen: [0; 256],
            total: 0,
        }
    }

    fn add(&mut self, byte: u8) {
        if let Some(slot) = self.seen.get_mut(usize::from(byte)) {
            *slot += 1;
        }
        self.total += 1;
    }

    fn of(&self, byte: u8) -> u64 {
        self.seen.get(usize::from(byte)).copied().unwrap_or(0)
    }
}

/// What a run of a generator gave: every character, and the first and the last of each password on their own.
struct Sample {
    anywhere: Tally,
    first: Tally,
    last: Tally,
    wrong_length: u64,
}

fn sample(length: usize, passwords: u64, mut generate: impl FnMut() -> SecretString) -> Sample {
    let mut sample = Sample {
        anywhere: Tally::new(),
        first: Tally::new(),
        last: Tally::new(),
        wrong_length: 0,
    };
    for _ in 0..passwords {
        let password = generate();
        let bytes = password.expose_secret().as_bytes();
        if bytes.len() != length {
            sample.wrong_length += 1;
        }
        for &byte in bytes {
            sample.anywhere.add(byte);
        }
        if let (Some(&first), Some(&last)) = (bytes.first(), bytes.last()) {
            sample.first.add(first);
            sample.last.add(last);
        }
    }
    sample
}

// ---------- The checks ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Anywhere,
    First,
    Last,
}

/// What a check found wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flaw {
    /// Some passwords did not have the length asked for.
    WrongLength { passwords: u64 },
    /// A character that the rules do not allow came out.
    Foreign { place: Place, byte: u8, seen: u64 },
    /// An allowed character came out more or less often than the tolerance admits.
    Skewed {
        place: Place,
        character: char,
        seen: u64,
        total: u64,
    },
}

/// Fewest draws for which T % of the expected count is at least 6 standard deviations: T²·N ≥ 360 000·(n − 1).
fn draws_for_six_deviations(alphabet_size: u64, percent: u64) -> u64 {
    (360_000 * (alphabet_size - 1)).div_ceil(percent * percent)
}

/// Smallest whole tolerance, in percent, that is at least 6 standard deviations for `draws` draws over
/// `alphabet_size` characters: the least T with T²·N ≥ 360 000·(n − 1).
fn percent_for_six_deviations(alphabet_size: u64, draws: u64) -> u64 {
    (1..=100)
        .find(|percent| draws_for_six_deviations(alphabet_size, *percent) <= draws)
        .expect("a sample this small supports no tolerance")
}

/// Whether `seen` of `total` is within `percent` of an even share among `alphabet_size` characters:
/// |seen − total/n| ≤ T/100 · total/n, in integers: 100·|seen·n − total| ≤ T·total.
fn within(seen: u64, total: u64, alphabet_size: u64, percent: u64) -> bool {
    100 * (seen * alphabet_size).abs_diff(total) <= percent * total
}

/// Every flaw of one tally against an even share of `alphabet`.
///
/// # Panics
/// If the tally has fewer draws than [`draws_for_six_deviations`]: with so few, an honest generator would trip
/// the tolerance by chance, and the answer would mean nothing.
fn skew(tally: &Tally, alphabet: &[u8], percent: u64, place: Place) -> Vec<Flaw> {
    let size = count(alphabet.len());
    let needed = draws_for_six_deviations(size, percent);
    assert!(
        tally.total >= needed,
        "{place:?}: {} draws are too few for ±{percent} % over {size} characters (6 deviations need {needed})",
        tally.total
    );
    let mut flaws = Vec::new();
    for byte in u8::MIN..=u8::MAX {
        let seen = tally.of(byte);
        if !alphabet.contains(&byte) {
            if seen > 0 {
                flaws.push(Flaw::Foreign { place, byte, seen });
            }
        } else if !within(seen, tally.total, size, percent) {
            flaws.push(Flaw::Skewed {
                place,
                character: char::from(byte),
                seen,
                total: tally.total,
            });
        }
    }
    flaws
}

fn verdict(flaws: Vec<Flaw>) -> Result<(), Vec<Flaw>> {
    if flaws.is_empty() { Ok(()) } else { Err(flaws) }
}

impl Sample {
    fn length_flaws(&self) -> Vec<Flaw> {
        if self.wrong_length == 0 {
            Vec::new()
        } else {
            vec![Flaw::WrongLength {
                passwords: self.wrong_length,
            }]
        }
    }

    /// 4a: each character of `alphabet` within `percent` of its even share, over all the characters drawn.
    fn uniformity(&self, alphabet: &[u8], percent: u64) -> Result<(), Vec<Flaw>> {
        let mut flaws = self.length_flaws();
        flaws.extend(skew(&self.anywhere, alphabet, percent, Place::Anywhere));
        verdict(flaws)
    }

    /// 4b: the same at the first position and at the last one, each on its own.
    fn placement(&self, alphabet: &[u8], percent: u64) -> Result<(), Vec<Flaw>> {
        let mut flaws = self.length_flaws();
        flaws.extend(skew(&self.first, alphabet, percent, Place::First));
        flaws.extend(skew(&self.last, alphabet, percent, Place::Last));
        verdict(flaws)
    }
}

/// 4a against any generator: `passwords` passwords of `length` characters, each character of `alphabet` within
/// `percent` of its even share.
fn check_uniformity(
    alphabet: &[u8],
    length: usize,
    passwords: u64,
    percent: u64,
    generate: impl FnMut() -> SecretString,
) -> Result<(), Vec<Flaw>> {
    sample(length, passwords, generate).uniformity(alphabet, percent)
}

/// 4b against any generator: the first and the last character of `passwords` passwords, each position on its own.
fn check_placement(
    alphabet: &[u8],
    length: usize,
    passwords: u64,
    percent: u64,
    generate: impl FnMut() -> SecretString,
) -> Result<(), Vec<Flaw>> {
    sample(length, passwords, generate).placement(alphabet, percent)
}

// ---------- The generators ----------

/// The generator of the product, fed by a fixed-seed source instead of the system's.
fn real(rules: PasswordRules, seed: u64) -> impl FnMut() -> SecretString {
    let mut source = Seeded(seed);
    move || generate_with(&rules, &mut source).expect("generate")
}

/// RIGGED (mutation of 4a): the real generator, except that every character is replaced by `favoured` with
/// probability 0.03/(n − 1). `favoured` then comes out with probability 1/n + (1 − 1/n)·0.03/(n − 1) = 1.03/n,
/// 3 % more than its share; each other character loses 0.03/(n − 1) of its own (0.034 % with 90 characters).
fn favouring(
    rules: PasswordRules,
    seed: u64,
    favoured: u8,
    alphabet_size: u64,
) -> impl FnMut() -> SecretString {
    let mut honest = real(rules, seed);
    let mut dice = Seeded(!seed);
    // The dice are 64 uniform bits: "below 2^64 · 3/(100·(n − 1))" happens with that probability.
    let threshold = u64::try_from((3_u128 << 64) / (100 * u128::from(alphabet_size - 1))).unwrap();
    move || {
        let password = honest();
        let rigged: String = password
            .expose_secret()
            .bytes()
            .map(|byte| {
                if dice.next() < threshold {
                    char::from(favoured)
                } else {
                    char::from(byte)
                }
            })
            .collect();
        SecretString::from(rigged)
    }
}

/// RIGGED (mutation of 4b): the real generator, except that the first character is always a lowercase letter,
/// drawn evenly among the 26. It is what "place one of each type first, then fill the rest" does to the first
/// position. Every other position is untouched.
fn lowercase_first(rules: PasswordRules, seed: u64) -> impl FnMut() -> SecretString {
    let mut honest = real(rules, seed);
    let mut dice = Seeded(!seed);
    let letters = LOWERCASE.as_bytes();
    move || {
        let password = honest();
        let mut bytes = password.expose_secret().as_bytes().to_vec();
        // 2^64 is not a multiple of 26, but the modulo bias is 26/2^64: nothing a sample can see.
        let index = usize::try_from(dice.next() % count(letters.len())).unwrap();
        if let (Some(first), Some(&letter)) = (bytes.first_mut(), letters.get(index)) {
            *first = letter;
        }
        SecretString::from(String::from_utf8(bytes).expect("ASCII stays ASCII"))
    }
}

// ---------- 4a: uniformity ----------

/// Expected appearances of each character in the runs with the four types: N = 100 000·n.
const PER_CHARACTER: u64 = 100_000;
/// The same in the runs of the 28 smaller alphabets: N = 40 000·n (see the top of the file).
const PER_CHARACTER_IN_SUBSETS: u64 = 40_000;
/// Tolerance of 4a, in percent of the even share.
const UNIFORMITY_PERCENT: u64 = 2;
/// Tolerance of 4b.
const PLACEMENT_PERCENT: u64 = 15;
/// Passwords in a placement run (at least 142 400 for 90 characters, see the top of the file).
const PLACEMENT_PASSWORDS: u64 = 150_000;

const ALL_TYPES: [bool; 4] = [true; 4];

#[test]
fn rf_69_each_of_the_90_characters_is_within_2_percent_of_its_share_over_9_million_characters() {
    let r = rules(100, ALL_TYPES, false);
    let alphabet = alphabet(&r);
    assert_eq!(alphabet.len(), 90);
    let passwords = PER_CHARACTER * 90 / 100;
    assert_eq!(passwords * 100, 9_000_000);

    assert_eq!(
        check_uniformity(
            &alphabet,
            100,
            passwords,
            UNIFORMITY_PERCENT,
            real(r, 0x5EED_0001)
        ),
        Ok(())
    );
}

#[test]
fn rf_67_rf_69_avoiding_lookalikes_each_of_the_83_characters_is_within_2_percent_at_500_characters()
{
    // The longest password the generator makes: 16 600 of 500 characters are the 8 300 000 that 83 characters need.
    let r = rules(500, ALL_TYPES, true);
    let alphabet = alphabet(&r);
    assert_eq!(alphabet.len(), 83);
    let passwords = PER_CHARACTER * 83 / 500;
    assert_eq!(passwords * 500, 8_300_000);

    assert_eq!(
        check_uniformity(
            &alphabet,
            500,
            passwords,
            UNIFORMITY_PERCENT,
            real(r, 0x5EED_0002)
        ),
        Ok(())
    );
}

/// The 14 subsets of types that are neither empty nor all four (those two are the tests above), with and without
/// look-alikes: 28 alphabets, from 8 characters (digits without `0` and `1`) to 64.
fn uniformity_of_subsets(avoid: bool) {
    let mut checked = 0;
    for mask in 1_u8..15 {
        let types = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0];
        let r = rules(100, types, avoid);
        let alphabet = alphabet(&r);
        let size = count(alphabet.len());
        let passwords = PER_CHARACTER_IN_SUBSETS * size / 100;
        // The tolerance comes from the size of the sample, not from what makes the test pass.
        let percent = percent_for_six_deviations(size, passwords * 100);
        assert_eq!(percent, 3, "{r:?}");
        let seed = 0x5EED_1000 + u64::from(mask) + if avoid { 0x100 } else { 0 };
        assert_eq!(
            check_uniformity(&alphabet, 100, passwords, percent, real(r, seed)),
            Ok(()),
            "{r:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, 14);
}

#[test]
fn rf_69_every_subset_of_types_is_within_3_percent_with_40_000_expected_per_character() {
    uniformity_of_subsets(false);
}

#[test]
fn rf_67_rf_69_every_subset_of_types_avoiding_lookalikes_is_within_3_percent() {
    uniformity_of_subsets(true);
}

// ---------- 4b: placement ----------

#[test]
fn rf_66_rf_69_the_first_and_the_last_character_are_each_within_15_percent_of_an_even_share() {
    let r = rules(100, ALL_TYPES, false);
    let alphabet = alphabet(&r);
    assert_eq!(alphabet.len(), 90);
    assert!(PLACEMENT_PASSWORDS >= draws_for_six_deviations(90, PLACEMENT_PERCENT));
    assert_eq!(
        check_placement(
            &alphabet,
            100,
            PLACEMENT_PASSWORDS,
            PLACEMENT_PERCENT,
            real(r, 0x5EED_0003)
        ),
        Ok(())
    );
}

// ---------- Mutation: the checks reject a rigged generator ----------

#[test]
fn rf_69_the_uniformity_check_rejects_a_generator_that_gives_one_character_3_percent_more() {
    let r = rules(100, ALL_TYPES, false);
    let alphabet = alphabet(&r);
    let favoured = b'k';
    // Same seed, length, size and tolerance as the test of the real generator: the only difference is the rigging.
    let rigged = favouring(r, 0x5EED_0001, favoured, 90);

    let rigged = sample(100, 90_000, rigged);
    let flaws = rigged
        .uniformity(&alphabet, UNIFORMITY_PERCENT)
        .expect_err("rigged");

    let [
        Flaw::Skewed {
            place: Place::Anywhere,
            character: 'k',
            seen,
            total: 9_000_000,
        },
    ] = flaws.as_slice()
    else {
        panic!("expected only the favoured character out of tolerance, got {flaws:?}");
    };
    assert!(
        *seen > 102_000,
        "it is flagged for being too frequent: {seen}"
    );
    eprintln!("favoured character: {seen} of 9 000 000 (even share 100 000, limit 102 000)");

    // Why 4a exists: the tolerance the suite had before (±20 %) accepts this same rigged sample.
    assert_eq!(rigged.uniformity(&alphabet, 20), Ok(()));
}

#[test]
fn rf_66_the_placement_check_rejects_a_generator_that_always_puts_a_lowercase_letter_first() {
    let r = rules(100, ALL_TYPES, false);
    let alphabet = alphabet(&r);
    // Same seed, length, size and tolerance as the test of the real generator.
    let rigged = sample(100, PLACEMENT_PASSWORDS, lowercase_first(r, 0x5EED_0003));

    let flaws = rigged
        .placement(&alphabet, PLACEMENT_PERCENT)
        .expect_err("rigged");

    // Every one of the 90 characters is off at the first position (26 letters far too often, 64 never), and
    // nothing is off at the last one.
    assert_eq!(flaws.len(), 90, "{flaws:?}");
    for flaw in &flaws {
        let Flaw::Skewed {
            place: Place::First,
            character,
            seen,
            total,
        } = *flaw
        else {
            panic!("expected only flaws at the first position, got {flaw:?}");
        };
        assert_eq!(total, PLACEMENT_PASSWORDS);
        if character.is_ascii_lowercase() {
            assert!(
                seen * 90 > 3 * total,
                "{character}: more than 3 times its share"
            );
        } else {
            assert_eq!(seen, 0, "{character} never comes first");
        }
    }
    assert_eq!(
        skew(&rigged.last, &alphabet, PLACEMENT_PERCENT, Place::Last),
        Vec::new()
    );
}

// ---------- The checks themselves ----------

#[test]
fn the_checks_flag_wrong_lengths_foreign_characters_and_refuse_samples_that_are_too_small() {
    let r = rules(100, [false, false, true, false], false);
    let digits = alphabet(&r);
    assert_eq!(digits, b"0123456789");
    // 1 000 000 draws over 10 characters, ±2 %: the sample is big enough (810 000 needed).
    assert_eq!(draws_for_six_deviations(10, 2), 810_000);
    assert_eq!(draws_for_six_deviations(90, 2), 8_010_000);
    assert_eq!(draws_for_six_deviations(90, 15), 142_400);
    // And the other way round: the tightest whole tolerance a sample supports.
    assert_eq!(percent_for_six_deviations(90, 9_000_000), 2);
    assert_eq!(percent_for_six_deviations(90, 8_000_000), 3);
    assert_eq!(percent_for_six_deviations(8, 320_000), 3);
    assert_eq!(percent_for_six_deviations(90, 150_000), 15);

    // A generator of the wrong length and with a letter among the digits.
    let mut honest = real(r, 7);
    let wrong = move || {
        let password = honest();
        let text: String = password.expose_secret().chars().skip(1).collect();
        SecretString::from(format!("{text}xx"))
    };
    let flaws = check_uniformity(&digits, 100, 10_000, UNIFORMITY_PERCENT, wrong).expect_err("bad");
    assert!(flaws.contains(&Flaw::WrongLength { passwords: 10_000 }));
    assert!(flaws.contains(&Flaw::Foreign {
        place: Place::Anywhere,
        byte: b'x',
        seen: 20_000,
    }));

    // The edge of the tolerance, in integers: 2 % of 100 000 is 2 000.
    assert!(within(102_000, 9_000_000, 90, 2) && within(98_000, 9_000_000, 90, 2));
    assert!(!within(102_001, 9_000_000, 90, 2) && !within(97_999, 9_000_000, 90, 2));

    // Too few draws for the tolerance: the check refuses instead of answering by chance.
    let small = sample(100, 100, real(r, 8));
    let refused = std::panic::catch_unwind(|| small.uniformity(b"0123456789", 2));
    assert!(refused.is_err(), "10 000 draws cannot support ±2 %");
}
