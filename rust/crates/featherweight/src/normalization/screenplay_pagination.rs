//! Port of `src/pdf/normalization/screenplay-pagination.ts`.
use crate::core::normalized_text::SuppressionReason;
use crate::core::page_extents::{
    PageExtentMeasurements, is_line_in_bottom_page_band, is_line_in_observed_bottom_page_band,
    is_line_in_top_page_band, page_margin_band_depth,
};
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::text::js_trim;
use crate::normalization::page_artifact_decisions::{
    ArtifactDiagnostics, SuppressionReasons, add_pagination_diagnostic, add_suppression_reason,
};
use crate::normalization::page_artifact_text::{
    CONTINUED_HEADER_PATTERN, CONTINUED_MARKER, CharacterCue, MORE_MARKER, create_text_fragments, parse_character_cue,
};

/// Port of `detectExplicitPaginationMarkers`.
pub fn detect_explicit_pagination_markers(
    lines: &[PhysicalTextLine],
    geometry: &PageExtentMeasurements,
    suppression_reasons: &mut SuppressionReasons,
    diagnostics: &mut ArtifactDiagnostics,
) {
    for (line_index, line) in lines.iter().enumerate() {
        let fragments = create_text_fragments(line_index, line);
        let line_text = js_trim(&line.text);
        let has_bottom_marker = line_text == MORE_MARKER || line_text == CONTINUED_MARKER;
        let has_top_marker = fragments
            .iter()
            .any(|fragment| CONTINUED_HEADER_PATTERN.is_match(&fragment.text));

        if has_bottom_marker {
            if is_line_in_bottom_page_band(line, geometry) {
                add_suppression_reason(suppression_reasons, line_index, SuppressionReason::ScreenplayPagination);
            } else {
                add_pagination_diagnostic(diagnostics, line_index, line);
            }
        }

        if !has_top_marker {
            continue;
        }

        if is_line_in_top_page_band(line, geometry) {
            add_suppression_reason(suppression_reasons, line_index, SuppressionReason::ScreenplayPagination);
        } else {
            add_pagination_diagnostic(diagnostics, line_index, line);
        }
    }
}

/// Port of `detectContinuedCharacterCues`.
pub fn detect_continued_character_cues(
    lines: &[PhysicalTextLine],
    geometry: &PageExtentMeasurements,
    suppression_reasons: &mut SuppressionReasons,
    diagnostics: &mut ArtifactDiagnostics,
) {
    // `Map.groupBy(lines, (line) => line.pageIndex)`, holding line indexes.
    let mut lines_by_page: Vec<(usize, Vec<usize>)> = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        match lines_by_page
            .iter_mut()
            .find(|(page_index, _)| *page_index == line.page_index)
        {
            Some((_, page_lines)) => page_lines.push(line_index),
            None => lines_by_page.push((line.page_index, vec![line_index])),
        }
    }

    lines_by_page.sort_by_key(|(page_index, _)| *page_index);

    for page_offset in 1..lines_by_page.len() {
        let (page_index, page_lines) = &lines_by_page[page_offset];
        let (previous_page_index, previous_page_lines) = &lines_by_page[page_offset - 1];

        if *page_index != previous_page_index + 1 {
            continue;
        }

        let is_suppressed = |line_index: usize| suppression_reasons[line_index].is_some();
        let Some(first_content_offset) = page_lines.iter().position(|&line_index| !is_suppressed(line_index)) else {
            continue;
        };

        let first_content_index = page_lines[first_content_offset];
        let first_content_line = &lines[first_content_index];
        let current_cue = parse_character_cue(first_content_index, first_content_line);

        let Some(current_cue) = current_cue.filter(|_| is_line_in_top_page_band(first_content_line, geometry)) else {
            continue;
        };

        let trailing_more_offset = find_trailing_more_line(lines, previous_page_lines, suppression_reasons);
        let previous_content_line =
            find_previous_content_line(previous_page_lines, trailing_more_offset, suppression_reasons)
                .map(|line_index| &lines[line_index]);
        let following_content_line = page_lines[first_content_offset + 1..]
            .iter()
            .find(|&&line_index| !is_suppressed(line_index))
            .map(|&line_index| &lines[line_index]);
        let previous_cue =
            find_previous_matching_column_cue(lines, previous_page_lines, &current_cue, suppression_reasons);
        let trailing_more_line = trailing_more_offset.map(|offset| &lines[previous_page_lines[offset]]);
        let ends_previous_page = previous_content_line.is_some_and(|previous_content_line| match trailing_more_line {
            None => is_line_in_observed_bottom_page_band(previous_content_line, geometry),
            Some(trailing_more_line) => {
                previous_content_line.bounds.y <= trailing_more_line.bounds.y + page_margin_band_depth(geometry)
            }
        });
        let has_continuation_context = match (previous_content_line, following_content_line) {
            (Some(previous_content_line), Some(following_content_line)) => {
                previous_cue.is_some_and(|previous_cue| previous_cue.name == current_cue.name)
                    && ends_previous_page
                    && has_dialogue_indent(first_content_line, previous_content_line)
                    && has_dialogue_indent(first_content_line, following_content_line)
            }
            _ => false,
        };

        if has_continuation_context {
            add_suppression_reason(
                suppression_reasons,
                first_content_index,
                SuppressionReason::ScreenplayPagination,
            );

            if let Some(offset) = trailing_more_offset {
                let trailing_more_index = previous_page_lines[offset];
                add_suppression_reason(
                    suppression_reasons,
                    trailing_more_index,
                    SuppressionReason::ScreenplayPagination,
                );
                diagnostics[trailing_more_index] = None;
            }

            continue;
        }

        if current_cue.continued {
            add_pagination_diagnostic(diagnostics, first_content_index, first_content_line);
        }
    }
}

/// Port of `findTrailingMoreLine`; returns the line's offset within `page_lines`.
fn find_trailing_more_line(
    lines: &[PhysicalTextLine],
    page_lines: &[usize],
    suppression_reasons: &SuppressionReasons,
) -> Option<usize> {
    for (offset, &line_index) in page_lines.iter().enumerate().rev() {
        if js_trim(&lines[line_index].text) == MORE_MARKER {
            return Some(offset);
        }

        suppression_reasons[line_index].as_ref()?;
    }

    None
}

/// Port of `findPreviousContentLine`. `trailing_more_offset` is the trailing
/// `(MORE)` line's offset within `page_lines` (TS: `lines.indexOf(trailingMoreLine)`).
fn find_previous_content_line(
    page_lines: &[usize],
    trailing_more_offset: Option<usize>,
    suppression_reasons: &SuppressionReasons,
) -> Option<usize> {
    let search_end = trailing_more_offset.unwrap_or(page_lines.len());

    page_lines[..search_end]
        .iter()
        .rev()
        .find(|&&line_index| suppression_reasons[line_index].is_none())
        .copied()
}

/// Port of `findPreviousMatchingColumnCue`.
fn find_previous_matching_column_cue(
    lines: &[PhysicalTextLine],
    page_lines: &[usize],
    current_cue: &CharacterCue,
    suppression_reasons: &SuppressionReasons,
) -> Option<CharacterCue> {
    let current_line = &lines[current_cue.line_index];
    let current_font_size = current_line.spans.first().map_or(12.0, |span| span.font.size);

    for &line_index in page_lines.iter().rev() {
        if suppression_reasons[line_index].is_some() {
            continue;
        }

        let line = &lines[line_index];
        let Some(cue) = parse_character_cue(line_index, line) else {
            continue;
        };

        if (line.bounds.x - current_line.bounds.x).abs() <= current_font_size * 2.0 {
            return Some(cue);
        }
    }

    None
}

fn has_dialogue_indent(cue_line: &PhysicalTextLine, content_line: &PhysicalTextLine) -> bool {
    let font_size = cue_line.spans.first().map_or(12.0, |span| span.font.size);
    let indent = cue_line.bounds.x - content_line.bounds.x;

    indent >= font_size * 2.0 && indent <= font_size * 8.0
}
