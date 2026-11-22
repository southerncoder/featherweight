//! Port of `src/core/recognition/sentence-spacing.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::physical_lines::PhysicalTextLine;
use crate::core::text::{js_trim_end, js_trim_start};

/// `SentenceSpacingConvention` (`1 | 2 | null`): `None` is TypeScript `null`,
/// otherwise the number of spaces between sentences.
pub type SentenceSpacingConvention = Option<usize>;

/// The consuming part of
/// `/[.!?…]["'’”)\]]?( +)(?=["'“‘(\[]?[\p{Lu}\p{N}])/gu`; the lookahead is
/// checked separately with [`SENTENCE_BEGINNING`].
static INTRA_LINE_SENTENCE_SPACING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[.!?…]["'’”)\]]?( +)"#).expect("valid regex"));
/// `/[.!?…]["'’”)\]]?$/u`.
static TERMINAL_SENTENCE_BOUNDARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[.!?…]["'’”)\]]?$"#).expect("valid regex"));
/// `/^["'“‘(\[]?[\p{Lu}\p{N}]/u`, also the lookahead of the intra-line pattern.
static SENTENCE_BEGINNING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^["'“‘(\[]?[\p{Lu}\p{N}]"#).expect("valid regex"));

/// Port of `inferSentenceSpacingConvention`.
pub fn infer_sentence_spacing_convention(lines: &[&PhysicalTextLine]) -> SentenceSpacingConvention {
    let mut one_space_boundaries = 0_usize;
    let mut two_space_boundaries = 0_usize;

    for line in lines {
        for spacing in intra_line_sentence_spacings(&line.text) {
            if spacing.len() == 1 {
                one_space_boundaries += 1;
            } else {
                two_space_boundaries += 1;
            }
        }
    }

    if one_space_boundaries == two_space_boundaries {
        return None;
    }

    Some(if one_space_boundaries > two_space_boundaries { 1 } else { 2 })
}

/// The `( +)` capture of every `matchAll` match of `intraLineSentenceSpacing`.
///
/// When the lookahead fails at a match start, JavaScript retries from the next
/// character; no shorter match at the same start can succeed (fewer spaces
/// leave a space before the lookahead, and dropping the closing mark leaves it
/// where a space is required), so advancing one character reproduces it.
fn intra_line_sentence_spacings(text: &str) -> Vec<&str> {
    let mut spacings = Vec::new();
    let mut position = 0;

    while let Some(captures) = INTRA_LINE_SENTENCE_SPACING.captures_at(text, position) {
        let whole = captures.get(0).expect("whole match");
        let spacing = captures.get(1).expect("spacing group");

        if SENTENCE_BEGINNING.is_match(&text[whole.end()..]) {
            spacings.push(spacing.as_str());
            position = whole.end();
        } else {
            let first_character = text[whole.start()..].chars().next().expect("non-empty match");
            position = whole.start() + first_character.len_utf8();
        }
    }

    spacings
}

/// Port of `isSentenceBoundary`.
pub fn is_sentence_boundary(previous_text: &str, next_text: &str) -> bool {
    TERMINAL_SENTENCE_BOUNDARY.is_match(js_trim_end(previous_text))
        && SENTENCE_BEGINNING.is_match(js_trim_start(next_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_spacing_only_before_sentence_beginnings() {
        assert_eq!(intra_line_sentence_spacings("One.  Two. three. \u{201C}Four"), vec!["  ", " "]);
        assert_eq!(intra_line_sentence_spacings("Wait!) 9 lives"), vec![" "]);
        assert!(intra_line_sentence_spacings("No.   ").is_empty());
    }

    #[test]
    fn detects_boundaries_across_lines() {
        assert!(is_sentence_boundary("He left.\u{2019}  ", "  (Then"));
        assert!(!is_sentence_boundary("He left", "Then"));
        assert!(!is_sentence_boundary("He left.", "then"));
    }
}
