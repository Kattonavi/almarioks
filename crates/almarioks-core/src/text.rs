//! Text rules of spec 001 ("Definiciones"): character, space, forbidden character, folding and title order.
//!
//! Shared by the vault model (validation, order of a group's entries) and by the search (T10). Folding is only
//! for comparing and ordering, never for storing.

use std::cmp::Ordering;

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Number of characters (Unicode code points) of a text. Length limits count these, not bytes.
#[must_use]
pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

/// Removes the characters with the Unicode `White_Space` property from both ends ("Recortar").
///
/// `char::is_whitespace` is exactly the `White_Space` property, so this is `str::trim`.
#[must_use]
pub fn trim(text: &str) -> &str {
    text.trim()
}

/// A character the owner may not write: controls U+0000 to U+001F and U+007F to U+009F, except tab and line feed.
#[must_use]
pub fn is_forbidden_char(c: char) -> bool {
    (c <= '\u{1F}' && c != '\t' && c != '\n') || ('\u{7F}'..='\u{9F}').contains(&c)
}

/// True if the text has at least one [forbidden character](is_forbidden_char).
#[must_use]
pub fn has_forbidden_char(text: &str) -> bool {
    text.chars().any(is_forbidden_char)
}

/// Folds a text: lower case, compatibility decomposition (NFKD), then combining marks removed, in that order (spec
/// "Plegar"). `Canción` and `cancion` fold the same, and so do `ñ` and `n`.
///
/// One pass is not idempotent: NFKD can give a capital (U+210C `ℌ` → `H`) and lower case can give a combining
/// mark (U+0130 `İ` → `i` + U+0307). The pass runs twice, so `fold(fold(x)) == fold(x)` for every text (checked
/// over every `char` and over seeded random texts in the tests).
#[must_use]
pub fn fold(text: &str) -> String {
    // ASCII has no decomposition and no marks, and its lower case is ASCII: the same result, much faster.
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }
    fold_once(&fold_once(text))
}

fn fold_once(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .nfkd()
        .filter(|&c| !is_combining_mark(c))
        .collect()
}

/// Compares two already folded texts by code point. UTF-8 byte order is code point order, so this is the plain
/// `str` order; it exists to name the rule.
#[must_use]
pub fn cmp_folded(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

#[cfg(test)]
mod tests {
    use super::{char_count, cmp_folded, fold, has_forbidden_char, is_forbidden_char, trim};
    use std::cmp::Ordering;

    #[test]
    fn fold_follows_the_examples_of_the_spec() {
        assert_eq!(fold("Canción"), fold("cancion"));
        assert_eq!(fold("ñ"), "n");
        assert_eq!(fold("CANCION"), "cancion");
        assert_eq!(fold("Años"), "anos");
        // NFKD: the "fi" ligature decomposes into two letters.
        assert_eq!(fold("\u{FB01}n"), "fin");
    }

    #[test]
    fn fold_of_a_compatibility_capital_is_lower_case() {
        // U+210C BLACK-LETTER CAPITAL H: no lower case of its own, and NFKD gives "H".
        assert_eq!(fold("\u{210C}"), "h");
        assert_eq!(fold("\u{210C}ola"), fold("Hola"));
        // U+0130: lower case gives "i" + U+0307, a combining mark.
        assert_eq!(fold("\u{130}"), "i");
    }

    #[test]
    fn fold_is_idempotent_for_every_char() {
        for c in (0..=0x10_FFFF_u32).filter_map(char::from_u32) {
            let once = fold(c.encode_utf8(&mut [0; 4]));
            assert_eq!(fold(&once), once, "U+{:04X}", u32::from(c));
        }
    }

    /// SplitMix64, a fixed-seed source for the property below.
    struct Seeded(u64);

    impl Seeded {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        /// A char, half of the time from the first 0x3000 code points (Latin, Greek, marks, letterlike symbols).
        fn char(&mut self) -> char {
            let wide = self.next();
            let range = if wide.is_multiple_of(2) {
                0x3000
            } else {
                0x11_0000
            };
            loop {
                let code = u32::try_from(self.next() % range).unwrap_or(0);
                if let Some(c) = char::from_u32(code) {
                    return c;
                }
            }
        }
    }

    #[test]
    fn fold_is_idempotent_on_seeded_random_texts() {
        let mut source = Seeded(0x00F0_01D0_u64);
        for _ in 0..20_000 {
            let len = usize::try_from(source.next() % 12).unwrap_or(0);
            let text: String = (0..len).map(|_| source.char()).collect();
            let once = fold(&text);
            assert_eq!(fold(&once), once, "{text:?}");
        }
    }

    #[test]
    fn characters_are_code_points_not_bytes() {
        assert_eq!(char_count("😀"), 1);
        assert_eq!(char_count("ñandú"), 5);
    }

    #[test]
    fn trim_removes_unicode_white_space() {
        // U+3000 ideographic space and U+00A0 no-break space have the White_Space property.
        assert_eq!(trim("\u{3000}\u{A0} Banco \t\n"), "Banco");
    }

    #[test]
    fn forbidden_characters_are_controls_except_tab_and_line_feed() {
        assert!(!is_forbidden_char('\t'));
        assert!(!is_forbidden_char('\n'));
        assert!(is_forbidden_char('\r'));
        assert!(is_forbidden_char('\u{0}'));
        assert!(is_forbidden_char('\u{7F}'));
        assert!(is_forbidden_char('\u{9F}'));
        assert!(!is_forbidden_char('\u{A0}'));
        assert!(has_forbidden_char("a\u{1B}b"));
        assert!(!has_forbidden_char("línea 1\nlínea 2\tfin"));
    }

    #[test]
    fn folded_order_is_by_code_point() {
        assert_eq!(
            cmp_folded(&fold("Banco"), &fold("árbol")),
            Ordering::Greater
        );
        assert_eq!(cmp_folded("a", "b"), Ordering::Less);
    }
}
