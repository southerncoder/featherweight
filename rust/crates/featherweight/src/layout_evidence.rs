use crate::coordinate_evidence::{CoordinateEvidence, infer_coordinate_tolerance, resolve_coordinate_evidence};
use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct DualDialogueEvidence {
    pub left_character_cue: CoordinateEvidence,
    pub left_dialogue: CoordinateEvidence,
    pub right_character_cue: CoordinateEvidence,
    pub right_dialogue: CoordinateEvidence,
}

#[derive(Debug, Clone)]
pub struct ScreenplayLayoutEvidence {
    pub action: CoordinateEvidence,
    pub character_cue: CoordinateEvidence,
    pub dialogue: CoordinateEvidence,
    pub parenthetical: CoordinateEvidence,
    pub transition: CoordinateEvidence,
    pub dual_dialogue: Option<DualDialogueEvidence>,
}

pub fn infer_layout_evidence(lines: &[PhysicalTextLine]) -> ScreenplayLayoutEvidence {
    let tol = infer_coordinate_tolerance(lines);
    let action_xs: Vec<f64> = lines.iter().map(|l| l.x).collect();
    let char_xs: Vec<f64> = lines.iter().filter(|l| l.text.trim().chars().all(|c| c.is_uppercase() || c.is_whitespace())).map(|l| l.x).collect();
    let trans_xs: Vec<f64> = lines.iter().map(|l| l.x + l.width).collect();
    let paren_xs: Vec<f64> = lines.iter().filter(|l| l.text.trim().starts_with('(')).map(|l| l.x).collect();
    ScreenplayLayoutEvidence {
        action: resolve_coordinate_evidence(&action_xs, tol),
        character_cue: resolve_coordinate_evidence(&char_xs, tol),
        dialogue: resolve_coordinate_evidence(&lines.iter().map(|l| l.x).collect::<Vec<_>>(), tol),
        parenthetical: resolve_coordinate_evidence(&paren_xs, tol),
        transition: resolve_coordinate_evidence(&trans_xs, tol),
        dual_dialogue: None,
    }
}