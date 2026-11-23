//! JavaScript string semantics the TypeScript port relies on.
//!
//! Use these instead of the `std` equivalents wherever the TypeScript code calls
//! `.length`, `.trim()`, or tests `\s`, so edge cases (U+FEFF, U+0085, astral
//! characters) behave identically.

/// Compiles one of this crate's constant regex patterns.
///
/// invariant: callers pass only string literals (or `format!` of literals) that
/// are compiled on first use by the unit tests of every module using them, so a
/// typo fails the test suite rather than a user's import.
pub(crate) fn constant_regex(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).expect("constant regex patterns are valid")
}

/// `String.prototype.length`: UTF-16 code units.
pub fn js_len(value: &str) -> usize {
    value.encode_utf16().count()
}

/// Membership in JavaScript's `\s` class (WhiteSpace + LineTerminator), which is
/// also the set `String.prototype.trim` strips. Differs from
/// [`char::is_whitespace`] on U+FEFF (JS: yes) and U+0085 (JS: no).
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | ' '
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// `String.prototype.trim`.
pub fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

/// `String.prototype.trimStart`.
pub fn js_trim_start(value: &str) -> &str {
    value.trim_start_matches(is_js_whitespace)
}

/// `String.prototype.trimEnd`.
pub fn js_trim_end(value: &str) -> &str {
    value.trim_end_matches(is_js_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_length_counts_utf16_units() {
        assert_eq!(js_len("a\u{2014}b"), 3);
        assert_eq!(js_len("\u{1F3AC}"), 2);
    }

    #[test]
    fn js_trim_matches_javascript_whitespace() {
        assert_eq!(js_trim("\u{FEFF} x \u{00A0}"), "x");
        assert_eq!(js_trim("\u{0085}x"), "\u{0085}x");
    }
}
