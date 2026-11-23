//! Port of `src/core/layout/dialogue-occurrences.ts`.
//!
//! Lines are borrowed (`&'a PhysicalTextLine`) and compared by identity, as the
//! TypeScript compares line objects.
use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_text::{
    is_cue_parenthetical_fragment, is_cue_uppercase_text, is_parenthetical, is_parenthetical_fragment,
};
use crate::core::layout::sequence_geometry::{is_vertically_adjacent, share_baseline};
use crate::core::page_extents::{is_line_in_bottom_page_band, is_line_in_top_page_band, measure_page_extents};
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::text::{js_trim_end, js_trim_start};

/// `DualDialogueOccurrence`.
#[derive(Debug, Clone, PartialEq)]
pub struct DualDialogueOccurrence<'a> {
    pub left_character_cue: &'a PhysicalTextLine,
    pub left_dialogue: &'a PhysicalTextLine,
    pub right_character_cue: &'a PhysicalTextLine,
    pub right_dialogue: &'a PhysicalTextLine,
    pub left_content: Vec<DialogueContentOccurrence<'a>>,
    pub right_content: Vec<DialogueContentOccurrence<'a>>,
    pub parentheticals: Vec<&'a PhysicalTextLine>,
    pub lines: Vec<&'a PhysicalTextLine>,
}

/// `DialogueContentOccurrence["type"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueContentType {
    Parenthetical,
    Dialogue,
}

/// `DialogueContentOccurrence`.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueContentOccurrence<'a> {
    pub content_type: DialogueContentType,
    pub lines: Vec<&'a PhysicalTextLine>,
}

/// `DialogueOccurrence`.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueOccurrence<'a> {
    pub character_cue: &'a PhysicalTextLine,
    pub content: Vec<DialogueContentOccurrence<'a>>,
    pub dialogue_lines: Vec<&'a PhysicalTextLine>,
    pub parentheticals: Vec<&'a PhysicalTextLine>,
}

/// `findDualDialogueOccurrences`.
pub fn find_dual_dialogue_occurrences<'a>(
    lines: &[&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Vec<DualDialogueOccurrence<'a>> {
    let mut occurrences = Vec::new();
    let mut index = 0;

    while index + 1 < lines.len() {
        match parse_dual_dialogue_occurrence(lines, index, coordinate_tolerance) {
            Some((occurrence, next_index)) => {
                occurrences.push(occurrence);
                // TS: `index = nextIndex - 1` followed by the loop's `index += 1`.
                index = next_index;
            }
            None => index += 1,
        }
    }

    occurrences
}

/// `parseDualDialogueOccurrence`: returns the occurrence and `nextIndex`.
fn parse_dual_dialogue_occurrence<'a>(
    lines: &[&'a PhysicalTextLine],
    cue_index: usize,
    coordinate_tolerance: f64,
) -> Option<(DualDialogueOccurrence<'a>, usize)> {
    let left_character_cue = lines[cue_index];
    let right_character_cue = lines[cue_index + 1];

    if !is_cue_uppercase_text(&left_character_cue.text)
        || !is_cue_uppercase_text(&right_character_cue.text)
        || is_cue_parenthetical_fragment(&left_character_cue.text)
        || is_cue_parenthetical_fragment(&right_character_cue.text)
        || left_character_cue.page_index != right_character_cue.page_index
        || !share_baseline(left_character_cue, right_character_cue, coordinate_tolerance)
        || left_character_cue.bounds.x >= right_character_cue.bounds.x
    {
        return None;
    }

    let lane_boundary = (left_character_cue.bounds.x + right_character_cue.bounds.x) / 2.0;
    let mut occurrence_lines: Vec<&'a PhysicalTextLine> = vec![left_character_cue, right_character_cue];
    let mut parentheticals: Vec<&'a PhysicalTextLine> = Vec::new();
    let mut left_dialogue: Option<&'a PhysicalTextLine> = None;
    let mut right_dialogue: Option<&'a PhysicalTextLine> = None;
    let mut previous_baseline = left_character_cue;
    let mut next_index = cue_index + 2;

    while next_index < lines.len() {
        let (row_lines, row_next_index) = read_baseline_row(lines, next_index, coordinate_tolerance);
        let row_first = row_lines[0];

        if row_first.page_index != left_character_cue.page_index
            || !is_vertically_adjacent(previous_baseline, row_first, 1.5)
        {
            break;
        }

        occurrence_lines.extend_from_slice(row_lines);

        for &line in row_lines {
            if is_parenthetical_fragment(&line.text) {
                if is_parenthetical(&line.text) {
                    parentheticals.push(line);
                }

                continue;
            }

            if line.bounds.x < lane_boundary && left_dialogue.is_none() {
                left_dialogue = Some(line);
            }

            if line.bounds.x > lane_boundary && right_dialogue.is_none() {
                right_dialogue = Some(line);
            }
        }

        previous_baseline = row_first;
        next_index = row_next_index;

        if left_dialogue.is_some() && right_dialogue.is_some() {
            break;
        }
    }

    let (Some(left_dialogue), Some(right_dialogue)) = (left_dialogue, right_dialogue) else {
        return None;
    };

    if left_dialogue.bounds.x >= left_character_cue.bounds.x
        || right_dialogue.bounds.x <= left_character_cue.bounds.x
        || right_dialogue.bounds.x >= right_character_cue.bounds.x
    {
        return None;
    }

    while next_index < lines.len() {
        let (row_lines, row_next_index) = read_baseline_row(lines, next_index, coordinate_tolerance);
        let row_first = row_lines[0];

        if row_first.page_index != left_character_cue.page_index
            || !is_vertically_adjacent(previous_baseline, row_first, 1.5)
            || !row_lines.iter().all(|line| {
                belongs_to_dual_dialogue_lane(line, left_dialogue, right_dialogue, lane_boundary, coordinate_tolerance)
            })
        {
            break;
        }

        occurrence_lines.extend_from_slice(row_lines);
        parentheticals.extend(row_lines.iter().copied().filter(|line| is_parenthetical(&line.text)));
        previous_baseline = row_first;
        next_index = row_next_index;
    }

    let left_content = group_dialogue_content(
        &occurrence_lines[2..]
            .iter()
            .copied()
            .filter(|line| line.bounds.x < lane_boundary)
            .collect::<Vec<_>>(),
    );
    let right_content = group_dialogue_content(
        &occurrence_lines[2..]
            .iter()
            .copied()
            .filter(|line| line.bounds.x > lane_boundary)
            .collect::<Vec<_>>(),
    );

    Some((
        DualDialogueOccurrence {
            left_character_cue,
            left_dialogue,
            right_character_cue,
            right_dialogue,
            left_content,
            right_content,
            parentheticals,
            lines: occurrence_lines,
        },
        next_index,
    ))
}

/// `readBaselineRow`: the run of same-page lines sharing the first line's
/// baseline, and the index after it.
fn read_baseline_row<'l, 'a>(
    lines: &'l [&'a PhysicalTextLine],
    start_index: usize,
    coordinate_tolerance: f64,
) -> (&'l [&'a PhysicalTextLine], usize) {
    let first_line = lines[start_index];
    let mut next_index = start_index + 1;

    while next_index < lines.len()
        && lines[next_index].page_index == first_line.page_index
        && share_baseline(first_line, lines[next_index], coordinate_tolerance)
    {
        next_index += 1;
    }

    (&lines[start_index..next_index], next_index)
}

/// `belongsToDualDialogueLane`.
fn belongs_to_dual_dialogue_lane(
    line: &PhysicalTextLine,
    left_dialogue: &PhysicalTextLine,
    right_dialogue: &PhysicalTextLine,
    lane_boundary: f64,
    coordinate_tolerance: f64,
) -> bool {
    if is_parenthetical_fragment(&line.text) {
        return true;
    }

    let expected_x = if line.bounds.x < lane_boundary {
        left_dialogue.bounds.x
    } else {
        right_dialogue.bounds.x
    };

    (line.bounds.x - expected_x).abs() <= coordinate_tolerance
}

/// `findDialogueOccurrences`.
pub fn find_dialogue_occurrences<'a>(
    lines: &[&'a PhysicalTextLine],
    reserved_lines: &LineSet<'_>,
    coordinate_tolerance: f64,
) -> Vec<DialogueOccurrence<'a>> {
    let mut occurrences = Vec::new();
    let page_extents = measure_page_extents(lines.iter().copied());
    let mut index = 0;

    while index + 1 < lines.len() {
        let character_cue = lines[index];

        if reserved_lines.contains(character_cue) || is_cue_parenthetical_fragment(&character_cue.text) {
            index += 1;
            continue;
        }

        let mut dialogue_index = index + 1;
        let mut content_lines: Vec<&'a PhysicalTextLine> = Vec::new();
        let mut previous_content_line = character_cue;
        let mut parenthetical_open = false;

        while dialogue_index < lines.len() {
            let possible_parenthetical = lines[dialogue_index];
            let continues_parenthetical =
                parenthetical_open || js_trim_start(&possible_parenthetical.text).starts_with('(');

            if !continues_parenthetical
                || reserved_lines.contains(possible_parenthetical)
                || possible_parenthetical.page_index != character_cue.page_index
                || !is_vertically_adjacent(previous_content_line, possible_parenthetical, 1.5)
                || possible_parenthetical.bounds.x >= character_cue.bounds.x
            {
                break;
            }

            content_lines.push(possible_parenthetical);
            parenthetical_open = !js_trim_end(&possible_parenthetical.text).ends_with(')');
            previous_content_line = possible_parenthetical;
            dialogue_index += 1;
        }

        if parenthetical_open {
            index += 1;
            continue;
        }

        let preceding_line = lines[dialogue_index - 1];
        let first_dialogue_line = match lines.get(dialogue_index) {
            Some(&line)
                if !(reserved_lines.contains(line)
                    || line.page_index != character_cue.page_index
                    || !is_vertically_adjacent(preceding_line, line, 1.5)
                    || character_cue.bounds.x - line.bounds.x <= character_cue.bounds.height
                    || (!is_cue_uppercase_text(&character_cue.text)
                        && character_cue.bounds.width >= line.bounds.width)) =>
            {
                line
            }
            _ => {
                index += 1;
                continue;
            }
        };

        let mut dialogue_lines: Vec<&'a PhysicalTextLine> = vec![first_dialogue_line];
        content_lines.push(first_dialogue_line);
        let mut previous_line = first_dialogue_line;
        let mut continuation_index = dialogue_index + 1;
        parenthetical_open = false;

        while continuation_index < lines.len() {
            let continuation = lines[continuation_index];

            if reserved_lines.contains(continuation) {
                break;
            }

            let continues_parenthetical = parenthetical_open || js_trim_start(&continuation.text).starts_with('(');

            if continues_parenthetical
                && continuation.page_index == previous_line.page_index
                && is_vertically_adjacent(previous_line, continuation, 1.5)
                && continuation.bounds.x > first_dialogue_line.bounds.x
                && continuation.bounds.x < character_cue.bounds.x
            {
                content_lines.push(continuation);
                parenthetical_open = !js_trim_end(&continuation.text).ends_with(')');
                previous_line = continuation;
                continuation_index += 1;
                continue;
            }

            let continues_across_page = continuation.page_index == previous_line.page_index + 1
                && is_line_in_bottom_page_band(previous_line, &page_extents)
                && is_line_in_top_page_band(continuation, &page_extents);
            let continues_on_page = continuation.page_index == previous_line.page_index
                && is_vertically_adjacent(previous_line, continuation, 1.5);

            if (continuation.bounds.x - first_dialogue_line.bounds.x).abs() > coordinate_tolerance
                || (!continues_across_page && !continues_on_page)
            {
                break;
            }

            dialogue_lines.push(continuation);
            content_lines.push(continuation);
            previous_line = continuation;
            continuation_index += 1;
        }

        let content = group_dialogue_content(&content_lines);
        let parentheticals = content
            .iter()
            .filter(|item| item.content_type == DialogueContentType::Parenthetical)
            .flat_map(|item| item.lines.iter().copied())
            .collect();
        occurrences.push(DialogueOccurrence {
            character_cue,
            content,
            dialogue_lines,
            parentheticals,
        });
        // TS: `index = continuationIndex - 1` followed by the loop's `index += 1`.
        index = continuation_index;
    }

    occurrences
}

/// `groupDialogueContent`.
fn group_dialogue_content<'a>(lines: &[&'a PhysicalTextLine]) -> Vec<DialogueContentOccurrence<'a>> {
    let mut groups: Vec<DialogueContentOccurrence<'a>> = Vec::new();
    let mut parenthetical_open = false;

    for &line in lines {
        let content_type = if parenthetical_open || js_trim_start(&line.text).starts_with('(') {
            DialogueContentType::Parenthetical
        } else {
            DialogueContentType::Dialogue
        };

        match groups.last_mut() {
            Some(current_group) if current_group.content_type == content_type => current_group.lines.push(line),
            _ => groups.push(DialogueContentOccurrence {
                content_type,
                lines: vec![line],
            }),
        }

        parenthetical_open =
            content_type == DialogueContentType::Parenthetical && !js_trim_end(&line.text).ends_with(')');
    }

    groups
}
