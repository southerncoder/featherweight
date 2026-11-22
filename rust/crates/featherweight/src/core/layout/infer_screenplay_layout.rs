//! Port of `src/core/layout/infer-screenplay-layout.ts`.
use crate::core::layout::coordinate_evidence::CoordinateEvidence;
use crate::core::layout::layout_evidence::{DualDialogueEvidence, infer_layout_evidence};
use crate::core::layout::screenplay_layout::{
    DualDialogueColumn, DualDialogueLayout, HorizontalAlignment, LayoutAnchor, LayoutDiagnostic, LayoutDiagnosticCode,
    LayoutRole, ScreenplayLayout,
};
use crate::core::normalized_text::NormalizedText;

/// `inferScreenplayLayout`.
pub fn infer_screenplay_layout(normalized_text: &NormalizedText) -> ScreenplayLayout {
    let evidence = infer_layout_evidence(normalized_text);
    let dual_dialogue = create_dual_dialogue_layout(evidence.dual_dialogue.as_ref());
    let mut diagnostics = Vec::new();

    add_coordinate_diagnostic(&mut diagnostics, LayoutRole::Action, &evidence.action);
    add_coordinate_diagnostic(&mut diagnostics, LayoutRole::CharacterCue, &evidence.character_cue);
    add_coordinate_diagnostic(&mut diagnostics, LayoutRole::Dialogue, &evidence.dialogue);
    add_coordinate_diagnostic(&mut diagnostics, LayoutRole::Parenthetical, &evidence.parenthetical);
    add_coordinate_diagnostic(&mut diagnostics, LayoutRole::Transition, &evidence.transition);

    match &evidence.dual_dialogue {
        None => diagnostics.push(LayoutDiagnostic {
            code: LayoutDiagnosticCode::InsufficientLayoutEvidence,
            role: LayoutRole::DualDialogue,
        }),
        Some(dual)
            if dual.left_character_cue.conflicting
                || dual.left_dialogue.conflicting
                || dual.right_character_cue.conflicting
                || dual.right_dialogue.conflicting =>
        {
            diagnostics.push(LayoutDiagnostic {
                code: LayoutDiagnosticCode::ConflictingLayoutEvidence,
                role: LayoutRole::DualDialogue,
            });
        }
        Some(_) => {}
    }

    ScreenplayLayout {
        action: create_anchor(HorizontalAlignment::Left, evidence.action.x),
        character_cue: create_anchor(HorizontalAlignment::Left, evidence.character_cue.x),
        dialogue: create_anchor(HorizontalAlignment::Left, evidence.dialogue.x),
        parenthetical: create_anchor(HorizontalAlignment::Left, evidence.parenthetical.x),
        transition: create_anchor(HorizontalAlignment::Right, evidence.transition.x),
        dual_dialogue,
        diagnostics,
    }
}

/// `createDualDialogueLayout`.
fn create_dual_dialogue_layout(evidence: Option<&DualDialogueEvidence>) -> Option<DualDialogueLayout> {
    let evidence = evidence?;
    let left = |x: f64| LayoutAnchor {
        alignment: HorizontalAlignment::Left,
        x,
    };

    Some(DualDialogueLayout {
        left: DualDialogueColumn {
            character_cue: left(evidence.left_character_cue.x?),
            dialogue: left(evidence.left_dialogue.x?),
        },
        right: DualDialogueColumn {
            character_cue: left(evidence.right_character_cue.x?),
            dialogue: left(evidence.right_dialogue.x?),
        },
    })
}

/// `addCoordinateDiagnostic`.
fn add_coordinate_diagnostic(diagnostics: &mut Vec<LayoutDiagnostic>, role: LayoutRole, evidence: &CoordinateEvidence) {
    if evidence.conflicting {
        diagnostics.push(LayoutDiagnostic {
            code: LayoutDiagnosticCode::ConflictingLayoutEvidence,
            role,
        });
        return;
    }

    if evidence.x.is_none() {
        diagnostics.push(LayoutDiagnostic {
            code: LayoutDiagnosticCode::InsufficientLayoutEvidence,
            role,
        });
    }
}

/// `createLeftAnchor` / `createRightAnchor`.
fn create_anchor(alignment: HorizontalAlignment, x: Option<f64>) -> Option<LayoutAnchor> {
    x.map(|x| LayoutAnchor { alignment, x })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::layout::screenplay_sequences::tests::line;
    use crate::core::normalized_text::NormalizedTextPage;

    /// Mirrors `createNormalizedText` in `test/unit/core/screenplay-layout.test.ts`.
    fn normalized(pages: &[&[(&str, f64, f64)]]) -> NormalizedText {
        let mut source_index = 0;
        NormalizedText {
            pages: pages
                .iter()
                .enumerate()
                .map(|(page_index, page_lines)| NormalizedTextPage {
                    page_index,
                    lines: page_lines
                        .iter()
                        .map(|&(text, x, y)| {
                            source_index += 1;
                            line(source_index - 1, page_index, text, x, y)
                        })
                        .collect(),
                })
                .collect(),
            suppressed: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn assert_anchor(actual: Option<LayoutAnchor>, alignment: HorizontalAlignment, expected_x: f64) {
        let anchor = actual.expect("anchor present");
        assert_eq!(anchor.alignment, alignment);
        assert!((anchor.x - expected_x).abs() <= 0.1, "{} vs {expected_x}", anchor.x);
    }

    fn has(result: &ScreenplayLayout, code: LayoutDiagnosticCode, role: LayoutRole) -> bool {
        result.diagnostics.contains(&LayoutDiagnostic { code, role })
    }

    #[test]
    fn empty_text_is_fully_insufficient() {
        let result = infer_screenplay_layout(&NormalizedText::default());
        assert_eq!(result.diagnostics.len(), 6);
        assert!(result.action.is_none() && result.transition.is_none() && result.dual_dialogue.is_none());
    }

    #[test]
    fn repeated_transition_right_edges_beyond_body_extents() {
        let result = infer_screenplay_layout(&normalized(&[
            &[
                (
                    "The unusually long body line reaches beyond the transition edge.",
                    126.0,
                    700.0,
                ),
                ("SHIFT:", 456.8, 676.0),
                ("A new image fills the frame.", 126.0, 664.0),
            ],
            &[
                (
                    "Another unusually long body line reaches beyond that same edge.",
                    126.0,
                    700.0,
                ),
                ("LONG SHIFT:", 420.8, 676.0),
                ("The image resolves into morning.", 126.0, 664.0),
            ],
        ]));

        assert_anchor(result.transition, HorizontalAlignment::Right, 500.0);
        assert_anchor(result.action, HorizontalAlignment::Left, 126.0);
        assert!(result.character_cue.is_none() && result.dialogue.is_none());
    }

    #[test]
    fn contextual_transition_separates_from_dialogue_geometry() {
        let result = infer_screenplay_layout(&normalized(&[
            &[
                ("ALICE", 252.0, 700.0),
                ("Wait here.", 180.0, 688.0),
                ("Rain reaches the windows.", 108.0, 652.0),
            ],
            &[
                ("BEN", 252.0, 700.0),
                ("I understand.", 180.0, 688.0),
                ("The room falls quiet.", 108.0, 652.0),
            ],
            &[("SHIFT:", 456.8, 700.0), ("Morning fills the orchard.", 108.0, 688.0)],
        ]));

        assert_anchor(result.action, HorizontalAlignment::Left, 108.0);
        assert_anchor(result.character_cue, HorizontalAlignment::Left, 252.0);
        assert_anchor(result.dialogue, HorizontalAlignment::Left, 180.0);
        assert_anchor(result.transition, HorizontalAlignment::Right, 500.0);
        for role in [
            LayoutRole::Action,
            LayoutRole::CharacterCue,
            LayoutRole::Dialogue,
            LayoutRole::Transition,
        ] {
            assert!(!has(&result, LayoutDiagnosticCode::ConflictingLayoutEvidence, role));
        }
    }

    #[test]
    fn cue_parenthetical_dialogue_block_is_not_a_transition() {
        let result = infer_screenplay_layout(&normalized(&[&[
            ("MARA", 252.0, 700.0),
            ("(quietly)", 216.0, 688.0),
            ("Wait.", 180.0, 676.0),
        ]]));

        assert_anchor(result.character_cue, HorizontalAlignment::Left, 252.0);
        assert_anchor(result.dialogue, HorizontalAlignment::Left, 180.0);
        assert_anchor(result.parenthetical, HorizontalAlignment::Left, 216.0);
        assert!(result.action.is_none() && result.transition.is_none());
    }

    #[test]
    fn stable_centered_parentheticals_conflict() {
        let pages: Vec<[(&str, f64, f64); 3]> = [
            ("ALICE", "(quietly)", 237.6, "Wait here."),
            ("BEN", "(quietly)", 237.6, "I understand."),
            ("CARA", "(quietly)", 237.6, "Then stay close."),
            ("DAVID", "(with concern)", 219.6, "The road is empty."),
            ("ELENA", "(after a measured pause)", 183.6, "We should leave."),
        ]
        .into_iter()
        .map(|(cue, parenthetical, x, dialogue)| {
            [(cue, 252.0, 700.0), (parenthetical, x, 688.0), (dialogue, 180.0, 676.0)]
        })
        .collect();
        let page_refs: Vec<&[(&str, f64, f64)]> = pages.iter().map(|page| page.as_slice()).collect();

        let result = infer_screenplay_layout(&normalized(&page_refs));

        assert!(result.parenthetical.is_none());
        assert!(has(
            &result,
            LayoutDiagnosticCode::ConflictingLayoutEvidence,
            LayoutRole::Parenthetical
        ));
    }
}
