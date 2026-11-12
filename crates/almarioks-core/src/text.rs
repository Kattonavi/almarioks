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
#[must_use]
pub fn fold(text: &str) -> String {
    let lower: String = text.chars().flat_map(char::to_lowercase).collect();
    lower.nfkd().filter(|&c| !is_combining_mark(c)).collect()
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
