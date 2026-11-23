//! Port of `src/core/layout/transition-evidence.ts`.
use std::collections::HashMap;

use crate::core::layout::coordinate_evidence::{cluster_coordinates, median};
use crate::core::layout::dialogue_occurrences::DialogueOccurrence;
use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_text::is_uppercase_text;
use crate::core::layout::sequence_geometry::{is_vertically_adjacent, js_math_max, js_math_min};
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::text::js_trim;

/// `new Map(lines.map((line, index) => [line, index]))`: identity-keyed, the
/// last index wins for a line that occurs twice.
fn index_lines(lines: &[&PhysicalTextLine]) -> HashMap<*const PhysicalTextLine, usize> {
    lines
        .iter()
        .enumerate()
        .map(|(index, &line)| (std::ptr::from_ref(line), index))
        .collect()
}

fn right_edge(line: &PhysicalTextLine) -> f64 {
    line.bounds.x + line.bounds.width
}

fn line_heights(lines: &[&PhysicalTextLine]) -> Vec<f64> {
    lines.iter().map(|line| line.bounds.height).collect()
}

/// `findContextualTransitionCandidates`.
pub fn find_contextual_transition_candidates<'a>(
    lines: &[&'a PhysicalTextLine],
    reserved_lines: &LineSet<'_>,
    dialogue_candidates: &[DialogueOccurrence<'_>],
    repeated_transition_lines: &[&PhysicalTextLine],
) -> Vec<&'a PhysicalTextLine> {
    if dialogue_candidates.len() < 2 || !repeated_transition_lines.is_empty() {
        return Vec::new();
    }

    let character_cue_x = median(
        &dialogue_candidates
            .iter()
            .map(|occurrence| occurrence.character_cue.bounds.x)
            .collect::<Vec<_>>(),
    );
    let dialogue_x = median(
        &dialogue_candidates
            .iter()
            .flat_map(|occurrence| occurrence.dialogue_lines.iter().map(|line| line.bounds.x))
            .collect::<Vec<_>>(),
    );
    let typical_line_height = median(&line_heights(lines));
    let cue_to_dialogue_indent = character_cue_x - dialogue_x;
    let right_side_threshold = character_cue_x + js_math_max(typical_line_height * 2.0, cue_to_dialogue_indent);
    let line_indexes = index_lines(lines);

    lines
        .iter()
        .copied()
        .filter(|&line| {
            if reserved_lines.contains(line) || !is_uppercase_text(&line.text) || line.bounds.x <= right_side_threshold
            {
                return false;
            }

            let next_line = lines.get(line_indexes[&std::ptr::from_ref(line)] + 1);

            next_line.is_some_and(|next_line| {
                next_line.page_index == line.page_index
                    && is_vertically_adjacent(line, next_line, 2.0)
                    && next_line.bounds.x < line.bounds.x - line.bounds.height
            })
        })
        .collect()
}

/// `findTransitionLines`.
pub fn find_transition_lines<'a>(
    lines: &[&'a PhysicalTextLine],
    reserved_lines: &LineSet<'_>,
    repeated_transition_lines: &[&'a PhysicalTextLine],
    dialogue_occurrences: &[DialogueOccurrence<'_>],
    coordinate_tolerance: f64,
) -> Vec<&'a PhysicalTextLine> {
    if lines.is_empty() {
        return Vec::new();
    }

    let standalone_opening_transitions: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .enumerate()
        .filter(|&(index, &line)| {
            !reserved_lines.contains(line)
                && js_trim(&line.text) == "FADE IN"
                && (index == 0 || lines[index - 1].page_index != line.page_index)
        })
        .map(|(_, &line)| line)
        .collect();

    // `Map.groupBy(lines, (line) => line.pageIndex)`: first-seen key order.
    let mut pages: Vec<(usize, Vec<&PhysicalTextLine>)> = Vec::new();
    for &line in lines {
        match pages.iter_mut().find(|(page_index, _)| *page_index == line.page_index) {
            Some((_, page_lines)) => page_lines.push(line),
            None => pages.push((line.page_index, vec![line])),
        }
    }
    let page_left_edges: Vec<f64> = pages
        .iter()
        .map(|(_, page_lines)| {
            page_lines
                .iter()
                .fold(f64::INFINITY, |minimum, line| js_math_min(minimum, line.bounds.x))
        })
        .collect();
    let page_right_edges: Vec<f64> = pages
        .iter()
        .map(|(_, page_lines)| {
            page_lines.iter().fold(f64::NEG_INFINITY, |maximum, line| {
                js_math_max(maximum, right_edge(line))
            })
        })
        .collect();
    let document_left_edge = median(&page_left_edges);
    let document_right_edge = median(&page_right_edges);
    let document_midpoint = (document_left_edge + document_right_edge) / 2.0;
    let right_side_threshold = infer_right_side_threshold(lines, dialogue_occurrences, document_midpoint);
    let geometric_candidates: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|&line| {
            !reserved_lines.contains(line)
                && is_uppercase_text(&line.text)
                && line.bounds.x > right_side_threshold
                && (right_edge(line) - document_right_edge).abs() <= line.bounds.height
        })
        .collect();
    let right_edge_clusters = cluster_coordinates(
        &geometric_candidates
            .iter()
            .map(|line| right_edge(line))
            .collect::<Vec<_>>(),
        coordinate_tolerance,
    );
    let line_indexes = index_lines(lines);

    let contextual_transition_lines = geometric_candidates.iter().copied().filter(|&line| {
        let line_index = line_indexes[&std::ptr::from_ref(line)];
        let next_line = lines.get(line_index + 1);
        let has_leftward_following_line = next_line.is_some_and(|next_line| {
            next_line.page_index == line.page_index
                && is_vertically_adjacent(line, next_line, 3.0)
                && next_line.bounds.x < line.bounds.x - line.bounds.height
        });
        let line_right_edge = right_edge(line);
        let has_repeated_right_edge = right_edge_clusters.iter().any(|cluster| {
            cluster.values.len() > 1
                && cluster
                    .values
                    .iter()
                    .any(|coordinate| (coordinate - line_right_edge).abs() <= coordinate_tolerance)
        });

        has_leftward_following_line || has_repeated_right_edge
    });

    repeated_transition_lines
        .iter()
        .copied()
        .chain(contextual_transition_lines)
        .chain(standalone_opening_transitions)
        .collect::<LineSet<'a>>()
        .into_vec()
}

/// `findRepeatedTransitionLines`.
pub fn find_repeated_transition_lines<'a>(
    lines: &[&'a PhysicalTextLine],
    reserved_lines: &LineSet<'_>,
    coordinate_tolerance: f64,
) -> Vec<&'a PhysicalTextLine> {
    if lines.is_empty() {
        return Vec::new();
    }

    let typical_left_edge = median(&lines.iter().map(|line| line.bounds.x).collect::<Vec<_>>());
    let typical_right_edge = median(&lines.iter().map(|line| right_edge(line)).collect::<Vec<_>>());
    let content_midpoint = (typical_left_edge + typical_right_edge) / 2.0;
    let candidates: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|&line| {
            !reserved_lines.contains(line) && is_uppercase_text(&line.text) && line.bounds.x > content_midpoint
        })
        .collect();

    filter_repeated_right_edge_lines(&candidates, coordinate_tolerance)
}

/// `filterRepeatedRightEdgeLines`.
fn filter_repeated_right_edge_lines<'a>(
    candidates: &[&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Vec<&'a PhysicalTextLine> {
    let right_edge_clusters = cluster_coordinates(
        &candidates.iter().map(|line| right_edge(line)).collect::<Vec<_>>(),
        coordinate_tolerance,
    );

    candidates
        .iter()
        .copied()
        .filter(|&line| {
            let line_right_edge = right_edge(line);
            let matching_cluster = right_edge_clusters.iter().find(|cluster| {
                cluster
                    .values
                    .iter()
                    .any(|coordinate| (coordinate - line_right_edge).abs() <= coordinate_tolerance)
            });

            let Some(matching_cluster) = matching_cluster.filter(|cluster| cluster.values.len() >= 2) else {
                return false;
            };

            let matching_candidate_xs: Vec<f64> = candidates
                .iter()
                .filter(|candidate| {
                    matching_cluster
                        .values
                        .iter()
                        .any(|coordinate| (coordinate - right_edge(candidate)).abs() <= coordinate_tolerance)
                })
                .map(|candidate| candidate.bounds.x)
                .collect();

            cluster_coordinates(&matching_candidate_xs, coordinate_tolerance).len() > 1
        })
        .collect()
}

/// `resolveRepeatedTransitionLines`.
pub fn resolve_repeated_transition_lines<'a>(
    lines: &[&PhysicalTextLine],
    transition_candidates: &[&'a PhysicalTextLine],
    dialogue_candidates: &[DialogueOccurrence<'_>],
    coordinate_tolerance: f64,
) -> Vec<&'a PhysicalTextLine> {
    let transition_candidate_set: LineSet<'a> = transition_candidates.iter().copied().collect();
    let unambiguous_cue_xs: Vec<f64> = dialogue_candidates
        .iter()
        .filter(|occurrence| !transition_candidate_set.contains(occurrence.character_cue))
        .map(|occurrence| occurrence.character_cue.bounds.x)
        .collect();

    if unambiguous_cue_xs.is_empty() {
        return transition_candidates.to_vec();
    }

    let character_cue_x = median(&unambiguous_cue_xs);
    let typical_line_height = median(&line_heights(lines));
    let right_side_threshold = character_cue_x + typical_line_height;

    filter_repeated_right_edge_lines(
        &transition_candidates
            .iter()
            .copied()
            .filter(|line| line.bounds.x > right_side_threshold)
            .collect::<Vec<_>>(),
        coordinate_tolerance,
    )
}

/// `inferRightSideThreshold`.
fn infer_right_side_threshold(
    lines: &[&PhysicalTextLine],
    dialogue_occurrences: &[DialogueOccurrence<'_>],
    content_midpoint: f64,
) -> f64 {
    if dialogue_occurrences.is_empty() {
        return content_midpoint;
    }

    let character_cue_x = median(
        &dialogue_occurrences
            .iter()
            .map(|occurrence| occurrence.character_cue.bounds.x)
            .collect::<Vec<_>>(),
    );
    let typical_line_height = median(&line_heights(lines));

    js_math_max(content_midpoint, character_cue_x + typical_line_height)
}
