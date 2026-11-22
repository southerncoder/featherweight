use crate::line_measure::{infer_line_measure, VISIBLE_LINE_SHORTFALL_MULTIPLIER};
use crate::model::PhysicalTextLine;
use crate::screenplay::ScreenplayElement;
use crate::screenplay_layout::ScreenplayLayout;
use crate::screenplay_text::is_uppercase_text;
use crate::sentence_spacing::SentenceSpacingConvention;
use crate::sequence_geometry::is_vertically_adjacent;
use crate::styled_text::{create_styled_text, is_continuation_dash};

pub struct ActionLineGroup {
    pub lines: Vec<PhysicalTextLine>,
    pub line_joins: Vec<&'static str>,
    pub next_index: usize,
}

pub struct CollectActionLinesOptions<'a> {
    pub lines: &'a [PhysicalTextLine],
    pub start_index: usize,
    pub blocked_lines: &'a [PhysicalTextLine],
    pub layout: &'a ScreenplayLayout,
    pub coordinate_tolerance: f64,
    pub action_line_measure: Option<f64>,
    pub centered_action_lines: &'a [PhysicalTextLine],
}

fn is_reserved(slice: &[PhysicalTextLine], l: &PhysicalTextLine) -> bool {
    slice.iter().any(|r| std::ptr::eq(r, l))
}

fn collect_centered(options: &CollectActionLinesOptions) -> ActionLineGroup {
    let mut action_lines = vec![options.lines[options.start_index].clone()];
    let mut line_joins: Vec<&'static str> = Vec::new();
    let mut next_index = options.start_index + 1;
    while next_index < options.lines.len() {
        let prev = action_lines.last().unwrap();
        let next_line = &options.lines[next_index];
        if is_reserved(options.blocked_lines, next_line)
            || !is_reserved(options.centered_action_lines, next_line)
            || !is_vertically_adjacent(prev, next_line, 1.5)
        { break; }
        line_joins.push("newline");
        action_lines.push(next_line.clone());
        next_index += 1;
    }
    ActionLineGroup { lines: action_lines, line_joins, next_index }
}

pub fn collect_action_lines(options: CollectActionLinesOptions) -> ActionLineGroup {
    let first_line = &options.lines[options.start_index];
    let centered = is_reserved(options.centered_action_lines, first_line);
    if centered { return collect_centered(&options); }

    let mut action_lines = vec![first_line.clone()];
    let mut line_joins: Vec<&'static str> = Vec::new();
    let mut next_index = options.start_index + 1;

    while next_index < options.lines.len() {
        let prev = action_lines.last().unwrap().clone();
        let next_line = &options.lines[next_index];
        if is_reserved(options.blocked_lines, next_line)
            || is_reserved(options.centered_action_lines, next_line)
            || !have_matching_action_origins(&prev, next_line, options.layout, options.coordinate_tolerance)
            || !continues_action_paragraph(&prev, next_line, options.action_line_measure)
        { break; }
        let preserve = should_preserve_action_line_break(&prev, next_line, options.action_line_measure);
        line_joins.push(if preserve { "newline" } else { "wrap" });
        action_lines.push(next_line.clone());
        next_index += 1;
    }
    ActionLineGroup { lines: action_lines, line_joins, next_index }
}

pub fn create_action(lines: Vec<PhysicalTextLine>, line_joins: Vec<&'static str>, centered: bool, sentence_spacing: SentenceSpacingConvention) -> ScreenplayElement {
    ScreenplayElement::Action {
        text: crate::styled_text::create_styled_text(&lines, &line_joins.iter().map(|s| *s).collect::<Vec<_>>(), sentence_spacing),
        alignment: if centered { "center".into() } else { "standard".into() },
    }
}

pub fn infer_action_line_measure(lines: &[PhysicalTextLine], layout: &ScreenplayLayout, tol: f64) -> Option<f64> {
    match &layout.action { Some(a) => infer_line_measure(lines, a.x, tol), None => None }
}

fn should_preserve_action_line_break(prev: &PhysicalTextLine, line: &PhysicalTextLine, measure: Option<f64>) -> bool {
    if measure.is_none() { return false; }
    let m = measure.unwrap();
    let visible = prev.height.min(line.height) * VISIBLE_LINE_SHORTFALL_MULTIPLIER as f64;
    (prev.width + visible < m) && (line.width + visible < m)
}

fn have_matching_action_origins(left: &PhysicalTextLine, right: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    if let Some(a) = &layout.action {
        let left_matches = (left.x - a.x).abs() <= tol;
        let right_matches = (right.x - a.x).abs() <= tol;
        if left_matches || right_matches { return left_matches && right_matches; }
        return left.x > a.x + tol && (left.x - right.x).abs() <= tol;
    }
    (left.x - right.x).abs() <= tol
}

fn continues_action_paragraph(prev: &PhysicalTextLine, line: &PhysicalTextLine, measure: Option<f64>) -> bool {
    if prev.page_index == line.page_index {
        return is_vertically_adjacent(prev, line, 1.5);
    }
    if line.page_index != prev.page_index + 1 { return false; }
    if measure.is_none() { return false; }
    let m = measure.unwrap();
    let visible = prev.height * VISIBLE_LINE_SHORTFALL_MULTIPLIER as f64;
    let reaches = prev.width + visible >= m;
    let prev_text = prev.text.trim_end();
    let line_text = line.text.trim_start();
    let paired_dash = is_continuation_dash(prev_text.chars().last().unwrap_or(' ').to_string().as_str())
        && line_text.chars().next().map(|c| is_continuation_dash(&c.to_string())).unwrap_or(false);
    let continues_ending = !prev_text.ends_with('.') && !prev_text.ends_with('!') && !prev_text.ends_with('?') && !prev_text.ends_with('\u{2026}');
    paired_dash || (reaches && continues_ending)
}