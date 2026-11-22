//! Port of `src/core/layout/layout-evidence.ts`.
use crate::core::layout::coordinate_evidence::{
    CoordinateEvidence, PageCoordinateObservation, cluster_coordinates, infer_coordinate_tolerance,
    resolve_coordinate_evidence, resolve_page_supported_coordinate_evidence,
};
use crate::core::layout::dialogue_occurrences::DualDialogueOccurrence;
use crate::core::layout::screenplay_sequences::find_screenplay_sequences;
use crate::core::layout::screenplay_text::is_uppercase_text;
use crate::core::normalized_text::NormalizedText;
use crate::core::physical_lines::PhysicalTextLine;

/// `DualDialogueEvidence`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DualDialogueEvidence {
    pub left_character_cue: CoordinateEvidence,
    pub left_dialogue: CoordinateEvidence,
    pub right_character_cue: CoordinateEvidence,
    pub right_dialogue: CoordinateEvidence,
}

/// `ScreenplayLayoutEvidence`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenplayLayoutEvidence {
    pub action: CoordinateEvidence,
    pub character_cue: CoordinateEvidence,
    pub dialogue: CoordinateEvidence,
    pub parenthetical: CoordinateEvidence,
    pub transition: CoordinateEvidence,
    pub dual_dialogue: Option<DualDialogueEvidence>,
}

/// `inferLayoutEvidence`.
pub fn infer_layout_evidence(normalized_text: &NormalizedText) -> ScreenplayLayoutEvidence {
    let lines: Vec<&PhysicalTextLine> = normalized_text
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .collect();
    let coordinate_tolerance = infer_coordinate_tolerance(&lines);
    let sequences = find_screenplay_sequences(&lines, coordinate_tolerance);
    let action_lines: Vec<&PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| !sequences.reserved_lines.contains(line))
        .collect();
    let body_action_lines = retain_body_action_evidence(action_lines, coordinate_tolerance);
    let parenthetical_lines: Vec<&PhysicalTextLine> = sequences
        .dialogue_occurrences
        .iter()
        .flat_map(|occurrence| occurrence.parentheticals.iter().copied())
        .chain(
            sequences
                .dual_dialogue_occurrences
                .iter()
                .flat_map(|occurrence| occurrence.parentheticals.iter().copied()),
        )
        .collect();

    ScreenplayLayoutEvidence {
        action: resolve_page_supported_coordinate_evidence(
            &body_action_lines
                .iter()
                .map(|line| PageCoordinateObservation {
                    x: line.bounds.x,
                    page_index: line.page_index,
                })
                .collect::<Vec<_>>(),
            coordinate_tolerance,
        ),
        character_cue: resolve_coordinate_evidence(
            &sequences
                .dialogue_occurrences
                .iter()
                .map(|occurrence| occurrence.character_cue.bounds.x)
                .collect::<Vec<_>>(),
            coordinate_tolerance,
        ),
        dialogue: resolve_coordinate_evidence(
            &sequences
                .dialogue_occurrences
                .iter()
                .flat_map(|occurrence| occurrence.dialogue_lines.iter().map(|line| line.bounds.x))
                .collect::<Vec<_>>(),
            coordinate_tolerance,
        ),
        parenthetical: resolve_parenthetical_evidence(&parenthetical_lines, coordinate_tolerance),
        transition: resolve_coordinate_evidence(
            &sequences
                .transition_lines
                .iter()
                .map(|line| line.bounds.x + line.bounds.width)
                .collect::<Vec<_>>(),
            coordinate_tolerance,
        ),
        dual_dialogue: infer_dual_dialogue_evidence(&sequences.dual_dialogue_occurrences, coordinate_tolerance),
    }
}

/// `retainBodyActionEvidence`: drops an all-uppercase, single-column first
/// page (a title page) whose origin never recurs on later pages.
fn retain_body_action_evidence(lines: Vec<&PhysicalTextLine>, coordinate_tolerance: f64) -> Vec<&PhysicalTextLine> {
    let Some(earliest_page_index) = lines.iter().map(|line| line.page_index).min() else {
        return lines;
    };
    let first_page_lines: Vec<&PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| line.page_index == earliest_page_index)
        .collect();
    let later_page_lines: Vec<&PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| line.page_index != earliest_page_index)
        .collect();

    if later_page_lines.is_empty() || !first_page_lines.iter().all(|line| is_uppercase_text(&line.text)) {
        return lines;
    }

    let first_page_clusters = cluster_coordinates(
        &first_page_lines.iter().map(|line| line.bounds.x).collect::<Vec<_>>(),
        coordinate_tolerance,
    );

    if first_page_clusters.len() != 1 {
        return lines;
    }

    let origin_values = &first_page_clusters[0].values;
    let first_page_origin = origin_values.iter().fold(0.0, |sum, x| sum + x) / origin_values.len() as f64;
    let origin_continues_later = later_page_lines
        .iter()
        .any(|line| (line.bounds.x - first_page_origin).abs() <= coordinate_tolerance);

    if origin_continues_later {
        lines
    } else {
        later_page_lines
    }
}

/// `resolveParentheticalEvidence`.
fn resolve_parenthetical_evidence(lines: &[&PhysicalTextLine], coordinate_tolerance: f64) -> CoordinateEvidence {
    let left_edges: Vec<f64> = lines.iter().map(|line| line.bounds.x).collect();
    let left_edge_evidence = resolve_coordinate_evidence(&left_edges, coordinate_tolerance);

    if lines.len() < 3 {
        return left_edge_evidence;
    }

    let left_edge_clusters = cluster_coordinates(&left_edges, coordinate_tolerance);
    let mut center_clusters = cluster_coordinates(
        &lines
            .iter()
            .map(|line| line.bounds.x + line.bounds.width / 2.0)
            .collect::<Vec<_>>(),
        coordinate_tolerance,
    );
    center_clusters.sort_by_key(|cluster| std::cmp::Reverse(cluster.values.len()));
    let dominant_center_cluster = &center_clusters[0];
    let stable_center = dominant_center_cluster.values.len() as f64 / lines.len() as f64 >= 0.8;

    if stable_center && left_edge_clusters.len() > 1 {
        return CoordinateEvidence {
            x: None,
            conflicting: true,
        };
    }

    left_edge_evidence
}

/// `inferDualDialogueEvidence`.
fn infer_dual_dialogue_evidence(
    occurrences: &[DualDialogueOccurrence<'_>],
    coordinate_tolerance: f64,
) -> Option<DualDialogueEvidence> {
    if occurrences.is_empty() {
        return None;
    }

    let resolve = |select: fn(&DualDialogueOccurrence<'_>) -> f64| {
        resolve_coordinate_evidence(
            &occurrences.iter().map(select).collect::<Vec<_>>(),
            coordinate_tolerance,
        )
    };

    Some(DualDialogueEvidence {
        left_character_cue: resolve(|occurrence| occurrence.left_character_cue.bounds.x),
        left_dialogue: resolve(|occurrence| occurrence.left_dialogue.bounds.x),
        right_character_cue: resolve(|occurrence| occurrence.right_character_cue.bounds.x),
        right_dialogue: resolve(|occurrence| occurrence.right_dialogue.bounds.x),
    })
}
