//! Port of `src/core/recognition/scene-headings.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_layout::ScreenplayLayout;
use crate::core::layout::screenplay_text::is_uppercase_text;
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::recognition::line_measure::{VISIBLE_LINE_SHORTFALL_MULTIPLIER, compare_numbers};
use crate::core::recognition::recognition_geometry::are_vertically_adjacent;
use crate::core::recognition::styled_text::{LineJoin, LineJoins, create_styled_text, trim_styled_text_end};
use crate::core::screenplay_document::ScreenplayElement;
use crate::core::text::constant_regex;
use crate::core::text::js_trim;

/// Port of `SceneHeadingEvidence`.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneHeadingEvidence<'a> {
    pub heading: &'a PhysicalTextLine,
    pub heading_lines: Vec<&'a PhysicalTextLine>,
    pub number_fragments: Vec<&'a PhysicalTextLine>,
    pub scene_number: Option<String>,
}

/// Port of `SceneNumberEvidence`.
struct SceneNumberEvidence<'a> {
    fragments: Vec<&'a PhysicalTextLine>,
    number: String,
}

/// `/^(?:INT\.|EXT\.|INT\/EXT\.|I\/E\.|EST\.)/u`.
static CONVENTIONAL_SCENE_HEADING: LazyLock<Regex> =
    LazyLock::new(|| constant_regex(r"^(?:INT\.|EXT\.|INT/EXT\.|I/E\.|EST\.)"));
/// `/^(?=.*\d)[A-Z0-9]+(?:[.-][A-Z0-9]+)*$/iu` without its lookahead, which
/// [`is_plausible_scene_number`] checks by hand.
static PLAUSIBLE_SCENE_NUMBER_SHAPE: LazyLock<Regex> =
    LazyLock::new(|| constant_regex(r"(?i)^[A-Z0-9]+(?:[.-][A-Z0-9]+)*$"));

/// `plausibleSceneNumber.test(value)`. The lookahead `(?=.*\d)` needs an ASCII
/// digit before any line terminator; the shape admits no line terminators, so
/// it reduces to "contains an ASCII digit".
fn is_plausible_scene_number(value: &str) -> bool {
    value.bytes().any(|byte| byte.is_ascii_digit()) && PLAUSIBLE_SCENE_NUMBER_SHAPE.is_match(value)
}

/// Port of `findSceneHeadings`.
pub fn find_scene_headings<'a>(
    lines: &[&'a PhysicalTextLine],
    reserved_lines: &LineSet<'a>,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
    action_line_measure: Option<f64>,
) -> Vec<SceneHeadingEvidence<'a>> {
    let mut headings = Vec::new();
    let mut consumed_continuation_lines: LineSet<'a> = LineSet::new();

    for (heading_index, &heading) in lines.iter().enumerate() {
        if reserved_lines.contains(heading) || consumed_continuation_lines.contains(heading) {
            continue;
        }

        let is_established_scene_heading = is_scene_heading_line(heading, layout, coordinate_tolerance);
        let is_possible_numbered_scene_heading = is_numbered_scene_heading_line(heading, layout, coordinate_tolerance);

        if !is_established_scene_heading && !is_possible_numbered_scene_heading {
            continue;
        }

        let scene_number = find_scene_number_evidence(lines, heading, coordinate_tolerance);

        if !is_established_scene_heading
            && scene_number
                .as_ref()
                .is_none_or(|evidence| !is_plausible_scene_number(&evidence.number))
        {
            continue;
        }

        let heading_lines = match lines.get(heading_index + 1) {
            Some(&continuation)
                if is_wrapped_scene_heading_continuation(
                    heading,
                    continuation,
                    reserved_lines,
                    layout,
                    coordinate_tolerance,
                    action_line_measure,
                ) =>
            {
                vec![heading, continuation]
            }
            _ => vec![heading],
        };

        if heading_lines.len() > 1 {
            consumed_continuation_lines.insert(heading_lines[1]);
        }

        let (number_fragments, scene_number) = match scene_number {
            Some(evidence) => (evidence.fragments, Some(evidence.number)),
            None => (Vec::new(), None),
        };

        headings.push(SceneHeadingEvidence {
            heading,
            heading_lines,
            number_fragments,
            scene_number,
        });
    }

    headings
}

/// Port of `createSceneHeading`.
pub fn create_scene_heading(evidence: &SceneHeadingEvidence<'_>) -> ScreenplayElement {
    ScreenplayElement::SceneHeading {
        page: None,
        text: trim_styled_text_end(&create_styled_text(
            &evidence.heading_lines,
            LineJoins::Uniform(LineJoin::Wrap),
            None,
        )),
        scene_number: evidence.scene_number.clone(),
    }
}

fn is_wrapped_scene_heading_continuation(
    heading: &PhysicalTextLine,
    continuation: &PhysicalTextLine,
    reserved_lines: &LineSet<'_>,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
    action_line_measure: Option<f64>,
) -> bool {
    let (Some(action_line_measure), Some(action)) = (action_line_measure, layout.action) else {
        return false;
    };

    if reserved_lines.contains(continuation)
        || continuation.page_index != heading.page_index
        || !are_vertically_adjacent(heading, continuation, 1.5)
        || (continuation.bounds.x - action.x).abs() > coordinate_tolerance
        || heading.bounds.width + heading.bounds.height * VISIBLE_LINE_SHORTFALL_MULTIPLIER < action_line_measure
    {
        return false;
    }

    let continuation_text = js_trim(&continuation.text);

    !CONVENTIONAL_SCENE_HEADING.is_match(continuation_text)
        && is_uppercase_text(continuation_text)
        && !continuation.spans.is_empty()
        && continuation.spans.iter().all(|span| span.style.bold)
}

/// Port of `isSceneHeadingLine`.
pub fn is_scene_heading_line(line: &PhysicalTextLine, layout: &ScreenplayLayout, coordinate_tolerance: f64) -> bool {
    let Some(action) = layout.action else {
        return false;
    };

    if (line.bounds.x - action.x).abs() > coordinate_tolerance {
        return false;
    }

    let trimmed_text = js_trim(&line.text);

    CONVENTIONAL_SCENE_HEADING.is_match(trimmed_text)
        || (is_uppercase_text(trimmed_text) && !line.spans.is_empty() && line.spans.iter().all(|span| span.style.bold))
}

fn is_numbered_scene_heading_line(
    line: &PhysicalTextLine,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    layout
        .action
        .is_some_and(|action| (line.bounds.x - action.x).abs() <= coordinate_tolerance)
        && is_uppercase_text(js_trim(&line.text))
}

fn find_scene_number_evidence<'a>(
    lines: &[&'a PhysicalTextLine],
    heading: &PhysicalTextLine,
    coordinate_tolerance: f64,
) -> Option<SceneNumberEvidence<'a>> {
    let baseline_fragments: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| {
            !std::ptr::eq(*line, heading)
                && line.page_index == heading.page_index
                && (line.bounds.y - heading.bounds.y).abs() <= coordinate_tolerance
        })
        .collect();
    let left_fragments: Vec<&'a PhysicalTextLine> = baseline_fragments
        .iter()
        .copied()
        .filter(|line| line.bounds.x < heading.bounds.x)
        .collect();
    let right_fragments: Vec<&'a PhysicalTextLine> = baseline_fragments
        .iter()
        .copied()
        .filter(|line| line.bounds.x > heading.bounds.x + heading.bounds.width)
        .collect();

    let paired_scene_number = left_fragments.iter().find_map(|&left| {
        let left_text = js_trim(&left.text);
        right_fragments
            .iter()
            .find(|right| js_trim(&right.text) == left_text && is_plausible_scene_number(left_text))
            .map(|&right| SceneNumberEvidence {
                fragments: vec![left, right],
                number: left_text.to_owned(),
            })
    });

    if paired_scene_number.is_some() {
        return paired_scene_number;
    }

    let mut left_candidates: Vec<&'a PhysicalTextLine> = left_fragments
        .into_iter()
        .filter(|line| is_plausible_scene_number(js_trim(&line.text)))
        .collect();
    left_candidates.sort_by(|left, right| {
        compare_numbers(
            heading.bounds.x
                - (left.bounds.x + left.bounds.width)
                - (heading.bounds.x - (right.bounds.x + right.bounds.width)),
        )
    });

    if let Some(&left_scene_number) = left_candidates.first() {
        return Some(SceneNumberEvidence {
            fragments: vec![left_scene_number],
            number: js_trim(&left_scene_number.text).to_owned(),
        });
    }

    let mut right_candidates: Vec<&'a PhysicalTextLine> = right_fragments
        .into_iter()
        .filter(|line| is_plausible_scene_number(js_trim(&line.text)))
        .collect();
    right_candidates.sort_by(|left, right| compare_numbers(left.bounds.x - right.bounds.x));

    right_candidates.first().map(|&right_scene_number| SceneNumberEvidence {
        fragments: vec![right_scene_number],
        number: js_trim(&right_scene_number.text).to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausible_scene_numbers_need_a_digit() {
        assert!(is_plausible_scene_number("12"));
        assert!(is_plausible_scene_number("12a"));
        assert!(is_plausible_scene_number("A-1.2"));
        assert!(!is_plausible_scene_number("AB"));
        assert!(!is_plausible_scene_number("1..2"));
        assert!(!is_plausible_scene_number("1 2"));
        assert!(!is_plausible_scene_number(""));
    }

    #[test]
    fn conventional_headings_are_prefix_matched() {
        assert!(CONVENTIONAL_SCENE_HEADING.is_match("INT/EXT. HOUSE"));
        assert!(CONVENTIONAL_SCENE_HEADING.is_match("EST. CITY"));
        assert!(!CONVENTIONAL_SCENE_HEADING.is_match("int. house"));
    }
}
