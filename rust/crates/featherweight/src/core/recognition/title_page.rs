//! Port of `src/core/recognition/title-page.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::layout::screenplay_layout::ScreenplayLayout;
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::recognition::recognition_geometry::{
    are_vertically_adjacent, line_center, vertical_gap_in_line_heights,
};
use crate::core::recognition::scene_headings::is_scene_heading_line;
use crate::core::recognition::styled_text::{LineJoin, LineJoins, create_styled_text};
use crate::core::screenplay_document::TitlePageField;
use crate::core::text::constant_regex;
use crate::core::text::{js_len, js_trim, js_trim_end};

/// Port of `TitlePageExtraction`.
#[derive(Debug, Clone, PartialEq)]
pub struct TitlePageExtraction<'a> {
    pub fields: Vec<TitlePageField>,
    pub body_lines: Vec<&'a PhysicalTextLine>,
    pub unattributed_lines: Vec<&'a PhysicalTextLine>,
}

/// JavaScript's `\s` class, spelled out because the `regex` crate's `\s`
/// differs on U+0085 and U+FEFF.
const JS_WHITESPACE_CLASS: &str =
    r"[\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// `/^(?:(?:written|prepared|screenplay|teleplay)\s+(?:by|for)|by)$/iu`.
static CONVENTIONAL_CREDIT: LazyLock<Regex> = LazyLock::new(|| {
    constant_regex(&format!(
        r"(?i)^(?:(?:written|prepared|screenplay|teleplay){JS_WHITESPACE_CLASS}+(?:by|for)|by)$"
    ))
});
/// `/^\d{1,4}[./-]\d{1,2}[./-]\d{1,4}$/u` (`\d` is ASCII in JavaScript).
static DRAFT_DATE: LazyLock<Regex> = LazyLock::new(|| constant_regex(r"^[0-9]{1,4}[./-][0-9]{1,2}[./-][0-9]{1,4}$"));
/// The `copyright` alternative of `copyrightNotice` before its `\b`.
static COPYRIGHT_WORD: LazyLock<Regex> = LazyLock::new(|| constant_regex(r"(?i)^copyright"));
/// The `©|\(c\)` alternatives of `copyrightNotice`.
static COPYRIGHT_SYMBOL: LazyLock<Regex> = LazyLock::new(|| constant_regex(r"(?i)^(?:©|\(c\))"));

/// `copyrightNotice.test(text)` for `/^(?:copyright\b|©|\(c\))/iu`.
///
/// Under the `u` and `i` flags, JavaScript's `\b` treats as word characters
/// `[A-Za-z0-9_]` plus the characters that case-fold into them (U+017F LATIN
/// SMALL LETTER LONG S and U+212A KELVIN SIGN); the boundary after
/// `copyright` is checked by hand against exactly that set.
fn is_copyright_notice(text: &str) -> bool {
    if COPYRIGHT_SYMBOL.is_match(text) {
        return true;
    }

    COPYRIGHT_WORD.find(text).is_some_and(|word| {
        !text[word.end()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphanumeric() || next == '_' || next == '\u{017F}' || next == '\u{212A}')
    })
}

/// Port of `extractTitlePage`.
pub fn extract_title_page<'a>(
    lines: &[&'a PhysicalTextLine],
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> TitlePageExtraction<'a> {
    let untouched = || TitlePageExtraction {
        fields: Vec::new(),
        body_lines: lines.to_vec(),
        unattributed_lines: Vec::new(),
    };

    if layout.action.is_none() {
        return untouched();
    }

    // TypeScript reads `lines[0]!`; callers never pass an empty list.
    let Some(first_line) = lines.first() else {
        return untouched();
    };
    let first_page_index = first_line.page_index;

    if first_page_index != 0 {
        return untouched();
    }

    let first_page_lines: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| line.page_index == first_page_index)
        .collect();
    let first_page_heading_index = first_page_lines
        .iter()
        .position(|line| is_scene_heading_line(line, layout, coordinate_tolerance));

    // A title block may share the first page with the script's opening. It is
    // then the lines above the first scene heading, set well apart from it.
    let title_lines = match first_page_heading_index {
        Some(heading_index) => shared_page_title_lines(&first_page_lines, heading_index),
        None => first_page_lines.clone(),
    };

    // The script starts at the top of the first page: no title page.
    if title_lines.is_empty() {
        return untouched();
    }

    let body_lines: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| !title_lines.iter().any(|title_line| std::ptr::eq(*title_line, *line)))
        .collect();
    let is_separate_title_page = !title_lines.is_empty()
        && !body_lines.is_empty()
        && body_lines
            .iter()
            .any(|line| is_scene_heading_line(line, layout, coordinate_tolerance));

    if !is_separate_title_page {
        return untouched();
    }

    match recognize_conventional_title_page(&title_lines, coordinate_tolerance) {
        // Lines above the script's first heading that are not a title block are
        // part of the script (cold-open text, scene-number fragments).
        None if first_page_heading_index.is_some() => untouched(),
        None => TitlePageExtraction {
            fields: Vec::new(),
            body_lines: lines.to_vec(),
            unattributed_lines: title_lines,
        },
        Some(fields) => TitlePageExtraction {
            fields,
            body_lines,
            unattributed_lines: Vec::new(),
        },
    }
}

const MINIMUM_TITLE_BLOCK_GAP: f64 = 3.0;

/// Port of `sharedPageTitleLines`: the lines of a title block that shares the
/// first page with the script, i.e. all lines above the first scene heading when
/// the lowest of them sits at least [`MINIMUM_TITLE_BLOCK_GAP`] line heights
/// above it. Otherwise none.
fn shared_page_title_lines<'a>(
    first_page_lines: &[&'a PhysicalTextLine],
    heading_index: usize,
) -> Vec<&'a PhysicalTextLine> {
    let heading = first_page_lines[heading_index];
    let candidates: Vec<&'a PhysicalTextLine> = first_page_lines[..heading_index]
        .iter()
        .copied()
        .filter(|line| line.bounds.y > heading.bounds.y)
        .collect();
    // `reduce` keeps the first of equally low lines.
    let lowest = candidates
        .iter()
        .copied()
        .fold(None::<&PhysicalTextLine>, |current, line| match current {
            Some(current) if line.bounds.y >= current.bounds.y => Some(current),
            _ => Some(line),
        });

    match lowest {
        Some(lowest) if vertical_gap_in_line_heights(lowest, heading) >= MINIMUM_TITLE_BLOCK_GAP => candidates,
        _ => Vec::new(),
    }
}

fn recognize_conventional_title_page(
    lines: &[&PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Option<Vec<TitlePageField>> {
    let footer_start_index = find_title_footer_start_index(lines);
    let upper_lines = &lines[..footer_start_index];
    let footer_lines = &lines[footer_start_index..];
    let upper_fields = recognize_upper_title_fields(upper_lines, coordinate_tolerance);
    let mut footer_fields = recognize_footer_title_fields(footer_lines, coordinate_tolerance);

    if upper_fields.is_empty() && footer_fields.is_empty() && footer_start_index == lines.len() {
        footer_fields = recognize_footer_title_fields(lines, coordinate_tolerance);
    }

    let fields: Vec<TitleFieldLines<'_>> = upper_fields.into_iter().chain(footer_fields).collect();

    if fields.is_empty() {
        None
    } else {
        Some(to_title_page_fields(lines, fields))
    }
}

fn find_title_footer_start_index(lines: &[&PhysicalTextLine]) -> usize {
    let mut largest_gap = 0.0;
    let mut footer_start_index = lines.len();

    for index in 1..lines.len() {
        let gap = vertical_gap_in_line_heights(lines[index - 1], lines[index]);

        if gap >= 6.0 && gap > largest_gap {
            largest_gap = gap;
            footer_start_index = index;
        }
    }

    if footer_start_index == lines.len() {
        let first_text = js_trim(&lines[0].text);
        if DRAFT_DATE.is_match(first_text) || is_copyright_notice(first_text) {
            return 0;
        }
    }

    footer_start_index
}

fn recognize_upper_title_fields<'a>(
    lines: &[&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Vec<TitleFieldLines<'a>> {
    if lines.is_empty()
        || !lines
            .iter()
            .all(|line| have_matching_centers(lines[0], line, coordinate_tolerance))
    {
        return Vec::new();
    }

    let Some(credit_index) = lines
        .iter()
        .position(|line| CONVENTIONAL_CREDIT.is_match(js_trim(&line.text)))
    else {
        let mut title_value_count = 1;

        while title_value_count < lines.len()
            && are_vertically_adjacent(lines[title_value_count - 1], lines[title_value_count], 1.5)
        {
            title_value_count += 1;
        }

        return vec![create_title_field("Title", &lines[..title_value_count])];
    };

    let mut fields = Vec::new();
    let title_values = &lines[..credit_index];
    let credit = lines[credit_index];
    let possible_author = lines.get(credit_index + 1).copied();
    let author = possible_author.filter(|author| vertical_gap_in_line_heights(credit, author) <= 2.5);
    let source_start_index = credit_index + if author.is_some() { 2 } else { 1 };
    let source = &lines[source_start_index.min(lines.len())..];

    if !title_values.is_empty() {
        fields.push(create_title_field("Title", title_values));
    }

    fields.push(create_title_field("Credit", &[credit]));

    if let Some(author) = author {
        fields.push(create_title_field("Author", &[author]));
    }

    if !source.is_empty() {
        fields.push(create_title_field("Source", source));
    }

    fields
}

fn recognize_footer_title_fields<'a>(
    lines: &[&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Vec<TitleFieldLines<'a>> {
    if lines.is_empty() {
        return Vec::new();
    }

    if let Some((notes, copyright)) = recognize_independent_footer(lines, coordinate_tolerance) {
        return vec![
            create_title_field("Notes", notes),
            create_title_field("Copyright", &[copyright]),
        ];
    }

    let mut fields = Vec::new();
    let copyright_index = lines.iter().position(|line| is_copyright_notice(js_trim(&line.text)));
    let copyright = copyright_index.map(|index| lines[index]);
    let conventional_lines: Vec<&PhysicalTextLine> = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != copyright_index)
        .map(|(_, line)| *line)
        .collect();
    let has_draft_date = DRAFT_DATE.is_match(conventional_lines.first().map_or("", |line| js_trim(&line.text)));
    let contact_candidate = &conventional_lines[usize::from(has_draft_date)..];
    let origin_tolerance = coordinate_tolerance.max(conventional_lines.first().map_or(0.0, |line| line.bounds.height));
    let has_matching_origins = contact_candidate
        .iter()
        .all(|line| (line.bounds.x - contact_candidate[0].bounds.x).abs() <= origin_tolerance);
    let has_matching_right_edges = contact_candidate.iter().all(|line| {
        (line.bounds.x + line.bounds.width - (contact_candidate[0].bounds.x + contact_candidate[0].bounds.width)).abs()
            <= origin_tolerance
    });
    let has_adjacent_contact_lines = contact_candidate
        .windows(2)
        .all(|pair| are_vertically_adjacent(pair[0], pair[1], 1.5));
    let has_supported_contact_geometry = if has_draft_date {
        contact_candidate.is_empty() || {
            let gap = vertical_gap_in_line_heights(conventional_lines[0], contact_candidate[0]);
            (0.75..=2.5).contains(&gap) && has_matching_origins && has_adjacent_contact_lines
        }
    } else {
        contact_candidate.len() > 1 && has_matching_origins && has_adjacent_contact_lines
    };
    let contact: &[&PhysicalTextLine] = if has_supported_contact_geometry {
        contact_candidate
    } else {
        &[]
    };

    if has_draft_date {
        fields.push(create_title_field("Draft date", &[conventional_lines[0]]));
    }

    if !contact.is_empty() {
        fields.push(create_title_field("Contact", contact));
    } else if !has_draft_date
        && copyright.is_none()
        && contact_candidate.len() > 1
        && has_matching_right_edges
        && has_adjacent_contact_lines
    {
        fields.push(create_title_field("Notes", contact_candidate));
    }

    if let Some(copyright) = copyright {
        fields.push(create_title_field("Copyright", &[copyright]));
    }

    fields
}

/// Port of `recognizeIndependentFooter`: the notes and the copyright line.
fn recognize_independent_footer<'l, 'a>(
    lines: &'l [&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Option<(&'l [&'a PhysicalTextLine], &'a PhysicalTextLine)> {
    if lines.len() < 2 {
        return None;
    }

    let copyright = lines[lines.len() - 1];

    if !is_copyright_notice(js_trim(&copyright.text)) {
        return None;
    }

    let notes = &lines[..lines.len() - 1];
    let right_edge_tolerance = coordinate_tolerance.max(notes[0].bounds.height / 2.0);
    let notes_right_edge = notes[0].bounds.x + notes[0].bounds.width;
    let has_right_aligned_notes = notes
        .iter()
        .all(|line| (line.bounds.x + line.bounds.width - notes_right_edge).abs() <= right_edge_tolerance)
        && notes
            .windows(2)
            .all(|pair| are_vertically_adjacent(pair[0], pair[1], 1.5));
    let footer_gap = vertical_gap_in_line_heights(notes[notes.len() - 1], copyright);
    let is_independent_copyright =
        (2.0..=5.0).contains(&footer_gap) && (copyright.bounds.x - notes[0].bounds.x).abs() > right_edge_tolerance;

    (has_right_aligned_notes && is_independent_copyright).then_some((notes, copyright))
}

/// Port of `TitleFieldLines`: a title-page field before its lines become
/// styled values.
struct TitleFieldLines<'a> {
    key: &'static str,
    lines: Vec<&'a PhysicalTextLine>,
}

fn create_title_field<'a>(key: &'static str, lines: &[&'a PhysicalTextLine]) -> TitleFieldLines<'a> {
    TitleFieldLines {
        key,
        lines: lines.to_vec(),
    }
}

/// Port of `toTitlePageFields`: title lines no field claims are kept as Notes,
/// so recognizing a title page never drops text.
fn to_title_page_fields(lines: &[&PhysicalTextLine], fields: Vec<TitleFieldLines<'_>>) -> Vec<TitlePageField> {
    let is_claimed = |line: &PhysicalTextLine| {
        fields
            .iter()
            .any(|field| field.lines.iter().any(|claimed| std::ptr::eq(*claimed, line)))
    };
    let unclaimed: Vec<&PhysicalTextLine> = lines.iter().copied().filter(|line| !is_claimed(line)).collect();
    let mut fields = fields;

    if !unclaimed.is_empty() {
        match fields.iter_mut().find(|field| field.key == "Notes") {
            Some(notes) => notes.lines.extend(unclaimed),
            None => fields.push(TitleFieldLines {
                key: "Notes",
                lines: unclaimed,
            }),
        }
    }

    fields
        .into_iter()
        .map(|field| TitlePageField {
            key: field.key.to_owned(),
            values: field
                .lines
                .iter()
                .map(|line| create_styled_text(&[line], LineJoins::Uniform(LineJoin::Wrap), None))
                .collect(),
        })
        .collect()
}

fn have_matching_centers(left: &PhysicalTextLine, right: &PhysicalTextLine, coordinate_tolerance: f64) -> bool {
    let tolerance = coordinate_tolerance.max(left.bounds.height.min(right.bounds.height) / 2.0);

    (line_center(left) - line_center(right)).abs() <= tolerance || have_matching_character_centers(left, right)
}

/// Port of `haveMatchingCharacterCenters`: text-formatted PDFs center lines by
/// padding with spaces, which a proportional font no longer renders centered,
/// so two space-indented lines are compared in character columns instead.
fn have_matching_character_centers(left: &PhysicalTextLine, right: &PhysicalTextLine) -> bool {
    let (Some(left_indentation), Some(right_indentation)) = (left.indentation, right.indentation) else {
        return false;
    };
    let character_center =
        |indentation: usize, line: &PhysicalTextLine| indentation as f64 + js_len(js_trim_end(&line.text)) as f64 / 2.0;

    (character_center(left_indentation, left) - character_center(right_indentation, right)).abs() <= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_copyright_notices_with_a_javascript_word_boundary() {
        assert!(is_copyright_notice("Copyright 2024"));
        assert!(is_copyright_notice("COPYRIGHT"));
        assert!(is_copyright_notice("copyright-2024"));
        assert!(is_copyright_notice("\u{a9} 2024"));
        assert!(is_copyright_notice("(C) 2024"));
        assert!(!is_copyright_notice("Copyrighted"));
        assert!(!is_copyright_notice("Copyright_x"));
        assert!(is_copyright_notice("Copyright\u{e9}"));
    }

    #[test]
    fn recognizes_credits_and_draft_dates() {
        assert!(CONVENTIONAL_CREDIT.is_match("Written  by"));
        assert!(CONVENTIONAL_CREDIT.is_match("SCREENPLAY\u{a0}FOR"));
        assert!(!CONVENTIONAL_CREDIT.is_match("Written\u{85}by"));
        assert!(DRAFT_DATE.is_match("12/31/2024"));
        assert!(!DRAFT_DATE.is_match("12/31/20245"));
    }
}
