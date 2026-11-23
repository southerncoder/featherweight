//! Port of `src/fdx/styled-text-to-fdx.ts`.
use crate::core::screenplay_document::{StyledText, TextRun, TextStyle};
use crate::error::ScreenplayConversionError;

/// `fdxStyleNames`.
fn fdx_style_name(style: TextStyle) -> &'static str {
    match style {
        TextStyle::Bold => "Bold",
        TextStyle::Italic => "Italic",
        TextStyle::Underline => "Underline",
        TextStyle::Strikeout => "Strikeout",
    }
}

/// Whether `character` is in `xmlTextPattern`'s class
/// (`[\u0009\u000A\u000D -퟿-�\u{10000}-\u{10FFFF}]`).
/// Surrogate code points cannot occur in a Rust `char`.
fn is_xml_text_character(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
}

/// Port of `assertValidFdxText`: rejects text containing code points XML 1.0
/// cannot represent.
pub fn assert_valid_fdx_text(value: &str) -> Result<(), ScreenplayConversionError> {
    if value.chars().all(is_xml_text_character) {
        Ok(())
    } else {
        Err(ScreenplayConversionError::InvalidFdxText)
    }
}

/// Port of `assertValidStyledText`.
pub fn assert_valid_styled_text(text: &StyledText) -> Result<(), ScreenplayConversionError> {
    text.runs.iter().try_for_each(|run| assert_valid_fdx_text(&run.text))
}

/// Port of `styledTextToFDX`: one `<Text>` element per run, with `addedStyles`
/// merged into every run's styles.
pub fn styled_text_to_fdx(text: &StyledText, added_styles: &[TextStyle]) -> String {
    text.runs.iter().map(|run| serialize_run(run, added_styles)).collect()
}

/// Port of `splitStyledTextLines`: splits styled text at newlines, dropping
/// empty run fragments but keeping empty lines.
pub fn split_styled_text_lines(text: &StyledText) -> Vec<StyledText> {
    let mut lines: Vec<Vec<TextRun>> = vec![Vec::new()];

    for run in &text.runs {
        let normalized = normalize_line_endings(&run.text);
        let parts: Vec<&str> = normalized.split('\n').collect();

        for (part_index, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                // invariant: `lines` starts with one entry and only grows.
                lines.last_mut().expect("lines is never empty").push(TextRun {
                    text: (*part).to_owned(),
                    styles: run.styles.clone(),
                });
            }

            if part_index < parts.len() - 1 {
                lines.push(Vec::new());
            }
        }
    }

    lines.into_iter().map(|runs| StyledText { runs }).collect()
}

/// Port of `plainStyledText`.
pub fn plain_styled_text(value: &str) -> StyledText {
    StyledText {
        runs: vec![TextRun {
            text: value.to_owned(),
            styles: Vec::new(),
        }],
    }
}

/// Port of `escapeXml`: escapes `&`, `<`, `>`, `"` and `'`.
pub fn escape_xml(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        push_xml_escaped(&mut output, character);
    }
    output
}

/// Port of `escapeXmlAttribute`: [`escape_xml`] plus character references for
/// tab, line feed and carriage return.
pub fn escape_xml_attribute(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\t' => output.push_str("&#x9;"),
            '\n' => output.push_str("&#xA;"),
            '\r' => output.push_str("&#xD;"),
            _ => push_xml_escaped(&mut output, character),
        }
    }
    output
}

fn push_xml_escaped(output: &mut String, character: char) {
    match character {
        '&' => output.push_str("&amp;"),
        '<' => output.push_str("&lt;"),
        '>' => output.push_str("&gt;"),
        '"' => output.push_str("&quot;"),
        '\'' => output.push_str("&apos;"),
        _ => output.push(character),
    }
}

fn serialize_run(run: &TextRun, added_styles: &[TextStyle]) -> String {
    let styles: Vec<&str> = TextStyle::ORDER
        .into_iter()
        .filter(|style| run.styles.contains(style) || added_styles.contains(style))
        .map(fdx_style_name)
        .collect();
    let style_attribute = if styles.is_empty() {
        String::new()
    } else {
        format!(" Style=\"{}\"", styles.join("+"))
    };
    let text = escape_xml(&normalize_line_endings(&run.text));

    format!("<Text{style_attribute}>{text}</Text>")
}

/// `value.replace(/\r\n|\r/g, "\n")`.
fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}
