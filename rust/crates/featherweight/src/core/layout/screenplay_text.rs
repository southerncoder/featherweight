//! Port of `src/core/layout/screenplay-text.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::text::{js_trim, js_trim_end};

static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}").expect("valid letter regex"));

/// `isUppercaseText`: at least two letters (`/\p{L}/gu` matches) and every
/// letter equals its upper-case form.
pub fn is_uppercase_text(text: &str) -> bool {
    let letters: Vec<&str> = LETTER.find_iter(text).map(|letter| letter.as_str()).collect();

    letters.len() >= 2 && letters.iter().all(|letter| *letter == letter.to_uppercase())
}

/// `isCueUppercaseText`.
pub fn is_cue_uppercase_text(text: &str) -> bool {
    is_uppercase_text(remove_trailing_parenthesized_groups(text))
}

/// `removeTrailingParenthesizedGroups`. Indexes are UTF-8 byte offsets; the
/// only characters inspected are ASCII parentheses, which never occur inside a
/// multi-byte UTF-8 sequence, so this is equivalent to the UTF-16 original.
fn remove_trailing_parenthesized_groups(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut prefix_end = js_trim_end(text).len();

    while prefix_end > 0 && bytes[prefix_end - 1] == b')' {
        let mut depth = 0_i64;
        let mut opening_index: Option<usize> = None;

        for index in (0..prefix_end).rev() {
            if bytes[index] == b')' {
                depth += 1;
            } else if bytes[index] == b'(' {
                depth -= 1;

                if depth == 0 {
                    opening_index = Some(index);
                    break;
                }
            }
        }

        let Some(opening_index) = opening_index else {
            break;
        };

        prefix_end = js_trim_end(&text[..opening_index]).len();
    }

    &text[..prefix_end]
}

/// `isParenthetical`.
pub fn is_parenthetical(text: &str) -> bool {
    let trimmed_text = js_trim(text);

    trimmed_text.starts_with('(') && trimmed_text.ends_with(')')
}

/// `isParentheticalFragment`.
pub fn is_parenthetical_fragment(text: &str) -> bool {
    let trimmed_text = js_trim(text);

    trimmed_text.starts_with('(') || trimmed_text.ends_with(')')
}

/// `isCueParentheticalFragment`.
pub fn is_cue_parenthetical_fragment(text: &str) -> bool {
    let trimmed_text = js_trim(text);

    trimmed_text.starts_with('(') || (trimmed_text.ends_with(')') && !trimmed_text.contains('('))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_requires_two_letters_all_upper() {
        assert!(is_uppercase_text("CUT TO:"));
        assert!(!is_uppercase_text("A"));
        assert!(!is_uppercase_text("A1"));
        assert!(!is_uppercase_text("Noah"));
        assert!(is_uppercase_text("\u{c9}LODIE"));
        assert!(!is_uppercase_text("STRA\u{df}E"));
    }

    #[test]
    fn cue_uppercase_ignores_trailing_parenthesized_groups() {
        assert!(is_cue_uppercase_text("NOAH (V.O.) (cont'd)"));
        assert!(is_cue_uppercase_text("NOAH (cont'd)  "));
        assert!(!is_cue_uppercase_text("Noah (V.O.)"));
        // Unbalanced: the closing paren has no opener, so nothing is removed.
        assert!(!is_cue_uppercase_text("NOAH cont'd)"));
        assert_eq!(remove_trailing_parenthesized_groups("NOAH ((x) y)"), "NOAH");
        assert_eq!(remove_trailing_parenthesized_groups("x)"), "x)");
    }

    #[test]
    fn parenthetical_predicates() {
        assert!(is_parenthetical(" (quietly) "));
        assert!(!is_parenthetical("(quietly"));
        assert!(is_parenthetical_fragment("quietly)"));
        assert!(is_cue_parenthetical_fragment("quietly)"));
        assert!(!is_cue_parenthetical_fragment("NOAH (V.O.)"));
        assert!(is_cue_parenthetical_fragment("(V.O.)"));
    }
}
