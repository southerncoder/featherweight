use crate::layout_evidence::{infer_layout_evidence, ScreenplayLayoutEvidence};
use crate::normalized_text::NormalizedText;
use crate::screenplay_layout::{DualDialogueLayout, LayoutAnchor, ScreenplayLayout};

#[derive(Debug, Clone)]
pub struct LayoutDiagnostic { pub code: &'static str, pub role: &'static str }

pub struct InferredLayout { pub layout: ScreenplayLayout, pub diagnostics: Vec<LayoutDiagnostic> }

pub fn infer_screenplay_layout(normalized: &NormalizedText) -> ScreenplayLayout {
    let evidence = infer_layout_evidence(normalized);
    let dual = if let Some(dd) = &evidence.dual_dialogue {
        if dd.left_character_cue.x.is_some() && dd.left_dialogue.x.is_some() && dd.right_character_cue.x.is_some() && dd.right_dialogue.x.is_some() {
            Some(DualDialogueLayout {
                left_character_cue_x: dd.left_character_cue.x.unwrap(),
                left_dialogue_x: dd.left_dialogue.x.unwrap(),
                right_character_cue_x: dd.right_character_cue.x.unwrap(),
                right_dialogue_x: dd.right_dialogue.x.unwrap(),
            })
        } else { None }
    } else { None };
    ScreenplayLayout {
        action: evidence.action.x.map(|x| LayoutAnchor { alignment: "left", x }),
        character_cue: evidence.character_cue.x.map(|x| LayoutAnchor { alignment: "left", x }),
        dialogue: evidence.dialogue.x.map(|x| LayoutAnchor { alignment: "left", x }),
        parenthetical: evidence.parenthetical.x.map(|x| LayoutAnchor { alignment: "left", x }),
        transition: evidence.transition.x.map(|x| LayoutAnchor { alignment: "right", x }),
        dual_dialogue: dual,
    }
}