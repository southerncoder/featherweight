//! Port of `src/core/recognition/styled-text.ts`.
use crate::core::text::constant_regex;
use std::sync::LazyLock;

use regex::Regex;

use crate::core::physical_lines::{PhysicalTextLine, PhysicalTextSpan};
use crate::core::recognition::sentence_spacing::{SentenceSpacingConvention, is_sentence_boundary};
use crate::core::screenplay_document::{StyledText, TextRun, TextStyle};
use crate::core::text::{is_js_whitespace, js_len, js_trim_end, js_trim_start};

/// `LineJoin`: how two consecutive physical lines are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    /// `"wrap"`: a soft wrap, joined with a space when needed.
    Wrap,
    /// `"newline"`: a deliberate line break.
    Newline,
}

/// The `lineJoins` parameter of `createStyledText`: one join for every line
/// boundary, or one join per boundary (missing entries default to `"wrap"`).
#[derive(Debug, Clone, Copy)]
pub enum LineJoins<'a> {
    /// `typeof lineJoins === "string"`.
    Uniform(LineJoin),
    /// `readonly LineJoin[]`.
    PerBoundary(&'a [LineJoin]),
}

/// `/^[,.;:!?…)]/u`.
static LEADING_ATTACHED_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| constant_regex(r"^[,.;:!?…)]"));

/// Port of `isContinuationDash`: `value` is one JavaScript "character"
/// (`text.at(-1)`, `text[0]`), or empty.
pub fn is_continuation_dash(value: &str) -> bool {
    value == "-" || value == "–" || value == "—"
}

/// Port of `createStyledText`.
pub fn create_styled_text(
    lines: &[&PhysicalTextLine],
    line_joins: LineJoins<'_>,
    sentence_spacing_convention: SentenceSpacingConvention,
) -> StyledText {
    let mut runs: Vec<TextRun> = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            let previous_line = lines[line_index - 1];
            let line_join = match line_joins {
                LineJoins::Uniform(join) => join,
                LineJoins::PerBoundary(joins) => joins.get(line_index - 1).copied().unwrap_or(LineJoin::Wrap),
            };
            let separator = create_line_separator(previous_line, line, line_join, sentence_spacing_convention);

            if !separator.is_empty() {
                append_run(&mut runs, &separator, &separator_styles(previous_line, line));
            }
        }

        // Whitespace at a line's edges is layout (indentation, padding), not
        // text; line separators account for it. Byte offsets here.
        let visible_start = line.text.len() - js_trim_start(&line.text).len();
        let visible_end = js_trim_end(&line.text).len();
        let mut previous_span_end = visible_start;

        for (span_index, span) in line.spans.iter().enumerate() {
            let start = span.start.max(visible_start);
            let end = span.end.min(visible_end);

            if end <= start {
                continue;
            }

            if start > previous_span_end {
                let previous_span = span_index.checked_sub(1).map(|index| &line.spans[index]);
                append_run(
                    &mut runs,
                    js_slice(&line.text, previous_span_end, start),
                    &separator_styles_for_spans(previous_span, span),
                );
            }

            append_run(
                &mut runs,
                js_slice(&line.text, start, end),
                &styles_for_span(Some(span)),
            );
            previous_span_end = end;
        }
    }

    StyledText { runs }
}

/// `text.slice(start, end)` for byte offsets: empty when `end <= start`, and
/// clamped to the text like JavaScript's `slice`.
fn js_slice(text: &str, start: usize, end: usize) -> &str {
    let end = end.min(text.len());
    if end <= start {
        return "";
    }
    text.get(start..end).unwrap_or("")
}

fn separator_styles_for_spans(
    previous_span: Option<&PhysicalTextSpan>,
    next_span: &PhysicalTextSpan,
) -> Vec<TextStyle> {
    let previous_styles = styles_for_span(previous_span);
    let next_styles = styles_for_span(Some(next_span));

    if previous_styles == next_styles {
        previous_styles
    } else {
        Vec::new()
    }
}

/// Port of `styledTextValue`.
pub fn styled_text_value(text: &StyledText) -> String {
    text.runs.iter().map(|run| run.text.as_str()).collect()
}

/// Port of `trimStyledTextEnd`.
pub fn trim_styled_text_end(text: &StyledText) -> StyledText {
    slice_styled_text(text, js_len(js_trim_end(&styled_text_value(text))))
}

/// Port of `sliceStyledText`. `end` is a JavaScript string length (UTF-16
/// code units), as the TypeScript callers derive it from `.length`.
pub fn slice_styled_text(text: &StyledText, end: usize) -> StyledText {
    let mut runs: Vec<TextRun> = Vec::new();
    let mut remaining = end;

    for run in &text.runs {
        if remaining == 0 {
            break;
        }

        let run_text = utf16_prefix(&run.text, remaining);
        append_run(&mut runs, run_text, &run.styles);
        remaining = remaining.saturating_sub(js_len(&run.text));
    }

    StyledText { runs }
}

/// `value.slice(0, length)` with `length` in UTF-16 code units. A cut inside
/// a surrogate pair (which the callers never produce: they cut at the end of
/// a trimmed prefix) drops the whole character.
fn utf16_prefix(value: &str, length: usize) -> &str {
    let mut units = 0;

    for (byte_index, character) in value.char_indices() {
        units += character.len_utf16();
        if units > length {
            return &value[..byte_index];
        }
    }

    value
}

fn create_line_separator(
    previous_line: &PhysicalTextLine,
    line: &PhysicalTextLine,
    line_join: LineJoin,
    sentence_spacing_convention: SentenceSpacingConvention,
) -> String {
    if line_join == LineJoin::Newline {
        return "\n".to_owned();
    }

    // `/\s+$/u` and `/^\s+/u`: every JavaScript whitespace character is one
    // UTF-16 code unit, so character counts equal `.length`.
    let trailing_whitespace = previous_line
        .text
        .chars()
        .rev()
        .take_while(|c| is_js_whitespace(*c))
        .count();
    let leading_whitespace = line.text.chars().take_while(|c| is_js_whitespace(*c)).count();

    let retained_spacing = trailing_whitespace + leading_whitespace;

    // Whitespace at the join: at a sentence boundary it shows the writer's
    // sentence spacing; anywhere else it is a single word space (text-formatted
    // PDFs pad line ends with spaces).
    if retained_spacing > 0 {
        if is_sentence_boundary(&previous_line.text, &line.text) {
            return " ".repeat(sentence_spacing_convention.unwrap_or(retained_spacing));
        }

        return " ".to_owned();
    }

    if LEADING_ATTACHED_PUNCTUATION.is_match(&line.text) || ends_with_continuation_dash(&previous_line.text) {
        return String::new();
    }

    " ".to_owned()
}

/// `isContinuationDash(text.at(-1) ?? "")`. A trailing astral character makes
/// `at(-1)` a lone surrogate, which is no dash either, so the last `char`
/// decides the same way.
pub(crate) fn ends_with_continuation_dash(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|c| is_continuation_dash(c.encode_utf8(&mut [0; 4])))
}

/// `isContinuationDash(text[0] ?? "")`.
pub(crate) fn starts_with_continuation_dash(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| is_continuation_dash(c.encode_utf8(&mut [0; 4])))
}

fn separator_styles(previous_line: &PhysicalTextLine, line: &PhysicalTextLine) -> Vec<TextStyle> {
    let previous_styles = styles_for_span(previous_line.spans.last());
    let next_styles = styles_for_span(line.spans.first());

    if previous_styles == next_styles {
        previous_styles
    } else {
        Vec::new()
    }
}

fn styles_for_span(span: Option<&PhysicalTextSpan>) -> Vec<TextStyle> {
    let Some(span) = span else {
        return Vec::new();
    };

    TextStyle::ORDER
        .into_iter()
        .filter(|style| match style {
            TextStyle::Bold => span.style.bold,
            TextStyle::Italic => span.style.italic,
            TextStyle::Underline => span.style.underline,
            TextStyle::Strikeout => span.style.strikeout,
        })
        .collect()
}

fn append_run(runs: &mut Vec<TextRun>, text: &str, styles: &[TextStyle]) {
    if text.is_empty() {
        return;
    }

    if let Some(previous_run) = runs.last_mut()
        && previous_run.styles == styles
    {
        previous_run.text.push_str(text);
        return;
    }

    runs.push(TextRun {
        text: text.to_owned(),
        styles: styles.to_vec(),
    });
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::core::positioned_text::{PositionedTextBounds, PositionedTextFont, PositionedTextStyle, SourceMethod};

    pub(crate) fn run(text: &str, styles: &[TextStyle]) -> TextRun {
        TextRun {
            text: text.to_owned(),
            styles: styles.to_vec(),
        }
    }

    fn styled(runs: Vec<TextRun>) -> StyledText {
        StyledText { runs }
    }

    #[test]
    fn slices_within_a_run_while_preserving_its_styles() {
        let text = styled(vec![
            run("Bold", &[TextStyle::Bold]),
            run(" italic", &[TextStyle::Italic]),
        ]);

        assert_eq!(
            slice_styled_text(&text, 7),
            styled(vec![run("Bold", &[TextStyle::Bold]), run(" it", &[TextStyle::Italic])])
        );
    }

    #[test]
    fn returns_all_runs_when_the_slice_end_exceeds_the_text_length() {
        let text = styled(vec![
            run("Bold", &[TextStyle::Bold]),
            run(" italic", &[TextStyle::Italic]),
        ]);

        assert_eq!(slice_styled_text(&text, 100), text);
    }

    #[test]
    fn trims_trailing_whitespace_across_multiple_runs() {
        let text = styled(vec![
            run("Text", &[TextStyle::Bold]),
            run("  ", &[TextStyle::Italic]),
            run("\t", &[TextStyle::Underline]),
        ]);

        assert_eq!(
            trim_styled_text_end(&text),
            styled(vec![run("Text", &[TextStyle::Bold])])
        );
    }

    #[test]
    fn trims_all_whitespace_text_to_no_runs() {
        let text = styled(vec![run(" ", &[TextStyle::Bold]), run("\t", &[TextStyle::Italic])]);

        assert_eq!(trim_styled_text_end(&text), styled(vec![]));
    }

    #[test]
    fn slices_astral_characters_by_utf16_length() {
        let text = styled(vec![run("a\u{1F3AC}b", &[])]);

        assert_eq!(slice_styled_text(&text, 3), styled(vec![run("a\u{1F3AC}", &[])]));
        assert_eq!(
            trim_styled_text_end(&styled(vec![run("\u{1F3AC} ", &[])])),
            styled(vec![run("\u{1F3AC}", &[])])
        );
    }

    pub(crate) fn line_with(text: &str, x: f64, y: f64, width: f64, style: PositionedTextStyle) -> PhysicalTextLine {
        let bounds = PositionedTextBounds {
            x,
            y,
            width,
            height: 12.0,
        };
        PhysicalTextLine {
            page_index: 0,
            text: text.to_owned(),
            bounds,
            indentation: None,
            spans: vec![PhysicalTextSpan {
                start: 0,
                end: text.len(),
                source_index: 0,
                source_method: SourceMethod::EmbeddedText,
                bounds,
                font: PositionedTextFont {
                    name: "Courier Prime".to_owned(),
                    size: 12.0,
                },
                style,
            }],
        }
    }

    #[test]
    fn does_not_create_empty_text_runs() {
        let line = line_with("", 108.0, 700.0, 0.0, PositionedTextStyle::default());

        assert_eq!(
            create_styled_text(&[&line], LineJoins::Uniform(LineJoin::Wrap), None),
            styled(vec![])
        );
    }

    #[test]
    fn joins_lines_with_separators() {
        let plain = PositionedTextStyle::default();
        let first = line_with("He left.", 0.0, 0.0, 0.0, plain);
        let second = line_with("Then", 0.0, 0.0, 0.0, plain);
        let dash = line_with("half-", 0.0, 0.0, 0.0, plain);
        let comma = line_with(", and", 0.0, 0.0, 0.0, plain);
        let spaced = line_with("He left. ", 0.0, 0.0, 0.0, plain);
        let wrap = LineJoins::Uniform(LineJoin::Wrap);

        assert_eq!(
            styled_text_value(&create_styled_text(&[&first, &second], wrap, None)),
            "He left. Then"
        );
        assert_eq!(
            styled_text_value(&create_styled_text(&[&dash, &second], wrap, None)),
            "half-Then"
        );
        assert_eq!(
            styled_text_value(&create_styled_text(&[&first, &comma], wrap, None)),
            "He left., and"
        );
        assert_eq!(
            styled_text_value(&create_styled_text(&[&spaced, &second], wrap, Some(2))),
            "He left.  Then"
        );
        assert_eq!(
            styled_text_value(&create_styled_text(&[&first, &second], wrap, Some(2))),
            "He left. Then"
        );
        assert_eq!(
            styled_text_value(&create_styled_text(
                &[&first, &second],
                LineJoins::PerBoundary(&[LineJoin::Newline]),
                None
            )),
            "He left.\nThen"
        );
    }
}
