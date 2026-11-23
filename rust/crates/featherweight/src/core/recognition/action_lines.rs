//! Port of `src/core/recognition/action-lines.ts`.
use crate::core::text::constant_regex;
use std::sync::LazyLock;

use regex::Regex;

use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_layout::ScreenplayLayout;
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::recognition::line_measure::{VISIBLE_LINE_SHORTFALL_MULTIPLIER, infer_line_measure};
use crate::core::recognition::recognition_geometry::are_vertically_adjacent;
use crate::core::recognition::sentence_spacing::SentenceSpacingConvention;
use crate::core::recognition::styled_text::{
    LineJoin, LineJoins, create_styled_text, ends_with_continuation_dash, starts_with_continuation_dash,
};
use crate::core::screenplay_document::{ActionAlignment, ScreenplayElement};
use crate::core::text::{js_trim_end, js_trim_start};

/// `/[.!?…]["'’”)\]]?$/u`.
static TERMINAL_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| constant_regex(r#"[.!?…]["'’”)\]]?$"#));

/// Port of `ActionLineGroup`.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionLineGroup<'a> {
    pub lines: Vec<&'a PhysicalTextLine>,
    pub line_joins: Vec<LineJoin>,
    pub next_index: usize,
}

/// The `options` object of `collectActionLines`.
#[derive(Debug, Clone, Copy)]
pub struct CollectActionLinesOptions<'o, 'a> {
    pub lines: &'o [&'a PhysicalTextLine],
    pub start_index: usize,
    pub blocked_lines: &'o LineSet<'a>,
    pub layout: &'o ScreenplayLayout,
    pub coordinate_tolerance: f64,
    pub action_line_measure: Option<f64>,
    pub centered_action_lines: &'o LineSet<'a>,
}

/// Port of `collectActionLines`.
pub fn collect_action_lines<'a>(options: &CollectActionLinesOptions<'_, 'a>) -> ActionLineGroup<'a> {
    let lines = options.lines;
    let start_index = options.start_index;
    let first_line = lines[start_index];
    let centered = options.centered_action_lines.contains(first_line);

    if centered {
        return collect_centered_action_lines(options);
    }

    let mut action_lines = vec![first_line];
    let mut line_joins = Vec::new();
    let mut next_index = start_index + 1;

    while next_index < lines.len() {
        let previous_line = action_lines[action_lines.len() - 1];
        let next_line = lines[next_index];

        if options.blocked_lines.contains(next_line)
            || options.centered_action_lines.contains(next_line)
            || !have_matching_action_origins(previous_line, next_line, options.layout, options.coordinate_tolerance)
            || !continues_action_paragraph(previous_line, next_line, options.action_line_measure)
        {
            break;
        }

        line_joins.push(
            if should_preserve_action_line_break(previous_line, next_line, options.action_line_measure) {
                LineJoin::Newline
            } else {
                LineJoin::Wrap
            },
        );
        action_lines.push(next_line);
        next_index += 1;
    }

    ActionLineGroup {
        lines: action_lines,
        line_joins,
        next_index,
    }
}

/// Port of `createAction`.
pub fn create_action(
    lines: &[&PhysicalTextLine],
    line_joins: &[LineJoin],
    centered_action_lines: &LineSet<'_>,
    sentence_spacing_convention: SentenceSpacingConvention,
) -> ScreenplayElement {
    ScreenplayElement::Action {
        page: None,
        text: create_styled_text(lines, LineJoins::PerBoundary(line_joins), sentence_spacing_convention),
        alignment: if centered_action_lines.contains(lines[0]) {
            ActionAlignment::Center
        } else {
            ActionAlignment::Standard
        },
    }
}

/// Port of `inferActionLineMeasure`.
pub fn infer_action_line_measure(
    lines: &[&PhysicalTextLine],
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> Option<f64> {
    let action = layout.action?;

    infer_line_measure(lines, action.x, coordinate_tolerance)
}

fn collect_centered_action_lines<'a>(options: &CollectActionLinesOptions<'_, 'a>) -> ActionLineGroup<'a> {
    let mut action_lines = vec![options.lines[options.start_index]];
    let mut line_joins = Vec::new();
    let mut next_index = options.start_index + 1;

    while next_index < options.lines.len() {
        let previous_line = action_lines[action_lines.len() - 1];
        let next_line = options.lines[next_index];

        if options.blocked_lines.contains(next_line)
            || !options.centered_action_lines.contains(next_line)
            || !are_vertically_adjacent(previous_line, next_line, 1.5)
        {
            break;
        }

        line_joins.push(LineJoin::Newline);
        action_lines.push(next_line);
        next_index += 1;
    }

    ActionLineGroup {
        lines: action_lines,
        line_joins,
        next_index,
    }
}

fn should_preserve_action_line_break(
    previous_line: &PhysicalTextLine,
    line: &PhysicalTextLine,
    action_line_measure: Option<f64>,
) -> bool {
    let Some(action_line_measure) = action_line_measure else {
        return false;
    };

    let visible_shortfall = previous_line.bounds.height.min(line.bounds.height) * VISIBLE_LINE_SHORTFALL_MULTIPLIER;

    previous_line.bounds.width + visible_shortfall < action_line_measure
        && line.bounds.width + visible_shortfall < action_line_measure
}

fn have_matching_action_origins(
    left: &PhysicalTextLine,
    right: &PhysicalTextLine,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    let Some(action) = layout.action else {
        return (left.bounds.x - right.bounds.x).abs() <= coordinate_tolerance;
    };

    let left_matches_action = (left.bounds.x - action.x).abs() <= coordinate_tolerance;
    let right_matches_action = (right.bounds.x - action.x).abs() <= coordinate_tolerance;

    if left_matches_action || right_matches_action {
        return left_matches_action && right_matches_action;
    }

    left.bounds.x > action.x + coordinate_tolerance && (left.bounds.x - right.bounds.x).abs() <= coordinate_tolerance
}

fn continues_action_paragraph(
    previous_line: &PhysicalTextLine,
    line: &PhysicalTextLine,
    action_line_measure: Option<f64>,
) -> bool {
    if previous_line.page_index == line.page_index {
        return are_vertically_adjacent(previous_line, line, 1.5);
    }

    if line.page_index != previous_line.page_index + 1 {
        return false;
    }

    let Some(action_line_measure) = action_line_measure else {
        return false;
    };

    let visible_shortfall = previous_line.bounds.height * VISIBLE_LINE_SHORTFALL_MULTIPLIER;
    let reaches_action_measure = previous_line.bounds.width + visible_shortfall >= action_line_measure;
    let previous_text = js_trim_end(&previous_line.text);
    let line_text = js_trim_start(&line.text);
    let has_paired_dash_continuation =
        ends_with_continuation_dash(previous_text) && starts_with_continuation_dash(line_text);
    let has_continuation_ending = !TERMINAL_PUNCTUATION.is_match(previous_text);

    has_paired_dash_continuation || (reaches_action_measure && has_continuation_ending)
}
