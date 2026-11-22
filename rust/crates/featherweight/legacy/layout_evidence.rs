use crate::coordinate_evidence::{
    infer_coordinate_tolerance, resolve_coordinate_evidence, resolve_page_supported_coordinate_evidence, PageCoordinateObservation,
};
use crate::model::PhysicalTextLine;
use crate::normalized_text::NormalizedText;
use crate::screenplay_sequences::find_screenplay_sequences;
use crate::screenplay_text::is_uppercase_text;

#[derive(Debug, Clone)]
pub struct DualDialogueEvidence {
    pub left_character_cue: crate::coordinate_evidence::CoordinateEvidence,
    pub left_dialogue: crate::coordinate_evidence::CoordinateEvidence,
    pub right_character_cue: crate::coordinate_evidence::CoordinateEvidence,
    pub right_dialogue: crate::coordinate_evidence::CoordinateEvidence,
}

#[derive(Debug, Clone)]
pub struct ScreenplayLayoutEvidence {
    pub action: crate::coordinate_evidence::CoordinateEvidence,
    pub character_cue: crate::coordinate_evidence::CoordinateEvidence,
    pub dialogue: crate::coordinate_evidence::CoordinateEvidence,
    pub parenthetical: crate::coordinate_evidence::CoordinateEvidence,
    pub transition: crate::coordinate_evidence::CoordinateEvidence,
    pub dual_dialogue: Option<DualDialogueEvidence>,
}

pub fn infer_layout_evidence(normalized: &NormalizedText) -> ScreenplayLayoutEvidence {
    let lines: Vec<PhysicalTextLine> = normalized.pages.iter().flat_map(|p| p.lines.clone()).collect();
    let tol = infer_coordinate_tolerance(&lines);
    let sequences = find_screenplay_sequences(&lines, tol);

    let mut reserved_ptrs: Vec<*const PhysicalTextLine> = sequences.reserved_lines.iter().map(|l| l as *const PhysicalTextLine).collect();
    let action_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| !reserved_ptrs.contains(&(*l as *const PhysicalTextLine))).cloned().collect();
    let body_action_lines = crate::recognition::retain_body_action_evidence(&action_lines, tol);

    let parenthetical_lines: Vec<PhysicalTextLine> = sequences.dialogue_occurrences.iter().flat_map(|o| o.parentheticals.clone())
        .chain(sequences.dual_dialogue_occurrences.iter().flat_map(|o| o.parentheticals.clone()))
        .collect();

    let action_obs: Vec<PageCoordinateObservation> = body_action_lines.iter().map(|l| PageCoordinateObservation { x: l.x, page_index: l.page_index }).collect();
    let action = resolve_page_supported_coordinate_evidence(&action_obs, tol);

    let char_xs: Vec<f64> = sequences.dialogue_occurrences.iter().map(|o| o.character_cue.x).collect();
    let character_cue = resolve_coordinate_evidence(&char_xs, tol);

    let dialogue_xs: Vec<f64> = sequences.dialogue_occurrences.iter().flat_map(|o| o.dialogue_lines.iter().map(|l| l.x)).collect();
    let dialogue = resolve_coordinate_evidence(&dialogue_xs, tol);

    // parenthetical
    let paren_xs: Vec<f64> = parenthetical_lines.iter().map(|l| l.x).collect();
    let paren_clusters = crate::coordinate_evidence::cluster_coordinates(&paren_xs, tol);
    let center_clusters = {
        let mut vals: Vec<f64> = parenthetical_lines.iter().map(|l| l.x + l.width / 2.0).collect();
        vals.sort_by(|a,b| a.partial_cmp(b).unwrap());
        crate::coordinate_evidence::cluster_coordinates(&vals, tol)
    };
    let dominant_center = center_clusters.iter().max_by_key(|c| c.values.len());
    let stable_center = dominant_center.map(|c| c.values.len() as f64 / parenthetical_lines.len().max(1) as f64 >= 0.8).unwrap_or(false);
    let parenthetical = if stable_center && paren_clusters.len() > 1 {
        crate::coordinate_evidence::CoordinateEvidence { x: None, conflicting: true }
    } else {
        resolve_coordinate_evidence(&paren_xs, tol)
    };

    let trans_xs: Vec<f64> = sequences.transition_lines.iter().map(|l| l.x + l.width).collect();
    let transition = resolve_coordinate_evidence(&trans_xs, tol);

    let dual_dialogue = if sequences.dual_dialogue_occurrences.is_empty() {
        None
    } else {
        Some(DualDialogueEvidence {
            left_character_cue: resolve_coordinate_evidence(&sequences.dual_dialogue_occurrences.iter().map(|o| o.left_character_cue.x).collect::<Vec<_>>(), tol),
            left_dialogue: resolve_coordinate_evidence(&sequences.dual_dialogue_occurrences.iter().map(|o| o.left_dialogue.x).collect::<Vec<_>>(), tol),
            right_character_cue: resolve_coordinate_evidence(&sequences.dual_dialogue_occurrences.iter().map(|o| o.right_character_cue.x).collect::<Vec<_>>(), tol),
            right_dialogue: resolve_coordinate_evidence(&sequences.dual_dialogue_occurrences.iter().map(|o| o.right_dialogue.x).collect::<Vec<_>>(), tol),
        })
    };

    ScreenplayLayoutEvidence { action, character_cue, dialogue, parenthetical, transition, dual_dialogue }
}