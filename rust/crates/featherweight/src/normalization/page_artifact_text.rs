//! Port of `src/pdf/normalization/page-artifact-text.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::physical_lines::{PhysicalTextLine, PhysicalTextSpan};
use crate::core::text::{js_len, js_trim};

/// Port of `TextFragment`. The TS `line` object is identified by `line_index`
/// (its position in `PhysicalText::lines`); `line` refers to the same line.
#[derive(Debug, Clone)]
pub struct TextFragment<'a> {
    pub font_family: String,
    pub font_size: f64,
    pub line_index: usize,
    pub line: &'a PhysicalTextLine,
    pub style: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// Port of `CharacterCue`; `line_index` stands in for the TS `line`.
#[derive(Debug, Clone)]
pub struct CharacterCue {
    pub continued: bool,
    pub line_index: usize,
    pub name: String,
}

/// Port of `moreMarker`.
pub const MORE_MARKER: &str = "(MORE)";
/// Port of `continuedMarker`.
pub const CONTINUED_MARKER: &str = "(CONTINUED)";

/// JavaScript's `\s` class (see [`crate::core::text::is_js_whitespace`]).
const JS_WHITESPACE_CLASS: &str =
    r"[\t\n\x0B\x0C\r \x{00A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// Port of `continuedHeaderPattern`: `/^CONTINUED:\s*\(\d+\)$/` (JS `\d` is ASCII).
pub static CONTINUED_HEADER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^CONTINUED:{JS_WHITESPACE_CLASS}*\([0-9]+\)$")).expect("valid regex"));

/// Port of `continuedCuePattern`.
static CONTINUED_CUE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][A-Z0-9 .'-]*?)(?: \((?:V\.?O\.?|O\.?S\.?)\))? \((?:CONT['\x{2019}]D|cont['\x{2019}]d)\)$")
        .expect("valid regex")
});

/// Port of `extendedCuePattern`.
static EXTENDED_CUE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Z][A-Z0-9 .'-]*?) \((?:V\.?O\.?|O\.?S\.?)\)$").expect("valid regex"));

/// Port of `bareCuePattern`.
static BARE_CUE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z][A-Z0-9 .'-]*$").expect("valid regex"));

/// `/-\d+$/` from `createTextFragment` (JS `\d` is ASCII).
static FONT_NAME_SUFFIX_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-[0-9]+$").expect("valid regex"));

/// Port of `createTextFragments`: one fragment per distinct trimmed span text, plus
/// the whole trimmed line text, in Map insertion order (a repeated text keeps its
/// first position but takes the later fragment, as `Map.prototype.set` does).
pub fn create_text_fragments(line_index: usize, line: &PhysicalTextLine) -> Vec<TextFragment<'_>> {
    let mut fragments: Vec<TextFragment<'_>> = Vec::new();

    for span in &line.spans {
        let text = js_trim(js_slice(&line.text, span.start, span.end));

        if js_len(text) == 0 {
            continue;
        }

        let fragment = create_text_fragment(line_index, line, span, text);

        match fragments.iter_mut().find(|existing| existing.text == fragment.text) {
            Some(existing) => *existing = fragment,
            None => fragments.push(fragment),
        }
    }

    if let Some(first_span) = line.spans.first() {
        let text = js_trim(&line.text);

        if !fragments.iter().any(|existing| existing.text == text) {
            fragments.push(create_text_fragment(line_index, line, first_span, text));
        }
    }

    fragments
}

/// `String.prototype.slice(start, end)` on byte offsets: clamps both ends to the
/// text and yields `""` when `start >= end`.
fn js_slice(text: &str, start: usize, end: usize) -> &str {
    let end = end.min(text.len());
    let start = start.min(end);
    text.get(start..end).unwrap_or("")
}

fn create_text_fragment<'a>(
    line_index: usize,
    line: &'a PhysicalTextLine,
    span: &PhysicalTextSpan,
    text: &str,
) -> TextFragment<'a> {
    TextFragment {
        font_family: FONT_NAME_SUFFIX_PATTERN.replace(&span.font.name, "").to_lowercase(),
        font_size: span.font.size,
        line_index,
        line,
        // `JSON.stringify(span.style)`: serde emits the fields in the TS object's order.
        style: serde_json::to_string(&span.style).expect("style serializes"),
        text: text.to_owned(),
        x: span.bounds.x,
        y: span.bounds.y,
    }
}

/// Port of `parseCharacterCue`.
pub fn parse_character_cue(line_index: usize, line: &PhysicalTextLine) -> Option<CharacterCue> {
    let text = js_trim(&line.text);

    if let Some(continued_match) = CONTINUED_CUE_PATTERN.captures(text) {
        return Some(CharacterCue { continued: true, line_index, name: continued_match[1].to_owned() });
    }

    if let Some(extended_match) = EXTENDED_CUE_PATTERN.captures(text) {
        return Some(CharacterCue { continued: false, line_index, name: extended_match[1].to_owned() });
    }

    if !BARE_CUE_PATTERN.is_match(text) {
        return None;
    }

    Some(CharacterCue { continued: false, line_index, name: text.to_owned() })
}

/// Port of `isExplicitPaginationText`.
pub fn is_explicit_pagination_text(text: &str) -> bool {
    text == MORE_MARKER
        || text == CONTINUED_MARKER
        || CONTINUED_HEADER_PATTERN.is_match(text)
        || CONTINUED_CUE_PATTERN.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continued_cue_captures_the_shortest_name() {
        let cue = CONTINUED_CUE_PATTERN.captures("JOHN (V.O.) (CONT'D)").unwrap();
        assert_eq!(&cue[1], "JOHN");
        let cue = CONTINUED_CUE_PATTERN.captures("MR. SMITH (cont\u{2019}d)").unwrap();
        assert_eq!(&cue[1], "MR. SMITH");
    }

    #[test]
    fn continued_header_accepts_js_whitespace_only() {
        assert!(is_explicit_pagination_text("CONTINUED:\u{FEFF}(2)"));
        assert!(!is_explicit_pagination_text("CONTINUED:\u{0085}(2)"));
        assert!(!is_explicit_pagination_text("CONTINUED: (\u{0663})"));
    }

    #[test]
    fn font_suffix_strips_only_ascii_digits() {
        assert_eq!(FONT_NAME_SUFFIX_PATTERN.replace("Courier-12", ""), "Courier");
        assert_eq!(FONT_NAME_SUFFIX_PATTERN.replace("Courier-\u{0663}", ""), "Courier-\u{0663}");
    }
}
