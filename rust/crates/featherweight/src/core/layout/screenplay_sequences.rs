//! Port of `src/core/layout/screenplay-sequences.ts`.
use crate::core::layout::dialogue_occurrences::{
    DialogueOccurrence, DualDialogueOccurrence, find_dialogue_occurrences, find_dual_dialogue_occurrences,
};
use crate::core::layout::line_set::LineSet;
use crate::core::layout::transition_evidence::{
    find_contextual_transition_candidates, find_repeated_transition_lines, find_transition_lines,
    resolve_repeated_transition_lines,
};
use crate::core::physical_lines::PhysicalTextLine;

/// `ScreenplaySequences`.
#[derive(Debug, Clone)]
pub struct ScreenplaySequences<'a> {
    pub transition_lines: Vec<&'a PhysicalTextLine>,
    pub dual_dialogue_occurrences: Vec<DualDialogueOccurrence<'a>>,
    pub dialogue_occurrences: Vec<DialogueOccurrence<'a>>,
    /// Insertion-ordered like the TypeScript `Set` (recognition iterates it).
    pub reserved_lines: LineSet<'a>,
}

/// `findScreenplaySequences`.
pub fn find_screenplay_sequences<'a>(
    lines: &[&'a PhysicalTextLine],
    coordinate_tolerance: f64,
) -> ScreenplaySequences<'a> {
    let mut reserved_lines = LineSet::new();
    let dual_dialogue_occurrences = find_dual_dialogue_occurrences(lines, coordinate_tolerance);

    for occurrence in &dual_dialogue_occurrences {
        reserved_lines.extend(occurrence.lines.iter().copied());
    }

    let dialogue_candidates = find_dialogue_occurrences(lines, &reserved_lines, coordinate_tolerance);
    let repeated_transition_candidates = find_repeated_transition_lines(lines, &reserved_lines, coordinate_tolerance);
    let repeated_transition_lines = resolve_repeated_transition_lines(
        lines,
        &repeated_transition_candidates,
        &dialogue_candidates,
        coordinate_tolerance,
    );
    let contextual_transition_lines =
        find_contextual_transition_candidates(lines, &reserved_lines, &dialogue_candidates, &repeated_transition_lines);
    let arbitrated_transition_set: LineSet<'a> = repeated_transition_lines
        .iter()
        .chain(&contextual_transition_lines)
        .copied()
        .collect();
    let arbitrated_transition_lines = arbitrated_transition_set.to_vec();
    let dialogue_occurrences: Vec<DialogueOccurrence<'a>> = dialogue_candidates
        .into_iter()
        .filter(|occurrence| !arbitrated_transition_set.contains(occurrence.character_cue))
        .collect();

    reserved_lines.extend(arbitrated_transition_lines.iter().copied());

    for occurrence in &dialogue_occurrences {
        reserved_lines.insert(occurrence.character_cue);
        reserved_lines.extend(occurrence.dialogue_lines.iter().copied());
        reserved_lines.extend(occurrence.parentheticals.iter().copied());
    }

    let transition_lines = find_transition_lines(
        lines,
        &reserved_lines,
        &arbitrated_transition_lines,
        &dialogue_occurrences,
        coordinate_tolerance,
    );

    reserved_lines.extend(transition_lines.iter().copied());

    ScreenplaySequences {
        transition_lines,
        dual_dialogue_occurrences,
        dialogue_occurrences,
        reserved_lines,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::core::physical_lines::PhysicalTextSpan;
    use crate::core::positioned_text::{PositionedTextBounds, PositionedTextFont, PositionedTextStyle, SourceMethod};
    use crate::core::text::js_len;

    /// Mirrors `createPhysicalLine` in `test/unit/core/screenplay-sequences.test.ts`.
    pub(crate) fn line(source_index: usize, page_index: usize, text: &str, x: f64, y: f64) -> PhysicalTextLine {
        let bounds = PositionedTextBounds {
            x,
            y,
            width: js_len(text) as f64 * 7.2,
            height: 12.0,
        };

        PhysicalTextLine {
            page_index,
            text: text.to_owned(),
            bounds,
            indentation: None,
            spans: vec![PhysicalTextSpan {
                start: 0,
                end: text.len(),
                source_index,
                source_method: SourceMethod::EmbeddedText,
                bounds,
                font: PositionedTextFont {
                    name: "Courier Prime".to_owned(),
                    size: 12.0,
                },
                style: PositionedTextStyle::default(),
            }],
        }
    }

    #[test]
    fn joins_dialogue_split_across_a_page_break() {
        let owned = [
            line(0, 0, "The platform trembles.", 108.0, 720.0),
            line(1, 0, "NOAH", 252.0, 100.0),
            line(2, 0, "I heard it", 180.0, 88.0),
            line(3, 0, "under the tracks.", 180.0, 76.0),
            line(4, 1, "It is getting closer.", 180.0, 720.0),
            line(5, 1, "The signal turns red.", 108.0, 600.0),
        ];
        let lines: Vec<&PhysicalTextLine> = owned.iter().collect();

        let result = find_screenplay_sequences(&lines, 6.0);

        assert_eq!(result.dialogue_occurrences.len(), 1);
        let dialogue = &result.dialogue_occurrences[0].dialogue_lines;
        assert_eq!(dialogue.len(), 3);
        for (actual, expected) in dialogue.iter().zip([&owned[2], &owned[3], &owned[4]]) {
            assert!(std::ptr::eq(*actual, expected));
        }
    }

    #[test]
    fn does_not_continue_high_dialogue_into_mid_page_line() {
        let owned = [
            line(0, 0, "NOAH", 252.0, 700.0),
            line(1, 0, "Wait.", 180.0, 688.0),
            line(2, 1, "The station is empty.", 180.0, 360.0),
            line(3, 1, "Rain crosses the tracks.", 108.0, 300.0),
            line(4, 1, "A signal changes.", 108.0, 72.0),
        ];
        let lines: Vec<&PhysicalTextLine> = owned.iter().collect();

        let result = find_screenplay_sequences(&lines, 6.0);

        assert_eq!(result.dialogue_occurrences.len(), 1);
        let dialogue = &result.dialogue_occurrences[0].dialogue_lines;
        assert_eq!(dialogue.len(), 1);
        assert!(std::ptr::eq(dialogue[0], &owned[1]));
        assert!(!result.reserved_lines.contains(&owned[2]));
    }

    #[test]
    fn groups_parenthetical_content_and_dual_dialogue() {
        let owned = [
            line(0, 0, "ALICE", 150.0, 700.0),
            line(1, 0, "BOB", 400.0, 700.0),
            line(2, 0, "(softly)", 120.0, 688.0),
            line(3, 0, "Left words.", 100.0, 676.0),
            line(4, 0, "Right words.", 350.0, 676.0),
            line(5, 0, "More left.", 100.0, 664.0),
        ];
        let lines: Vec<&PhysicalTextLine> = owned.iter().collect();

        let result = find_screenplay_sequences(&lines, 3.0);

        assert_eq!(result.dual_dialogue_occurrences.len(), 1);
        let occurrence = &result.dual_dialogue_occurrences[0];
        assert!(std::ptr::eq(occurrence.left_dialogue, &owned[3]));
        assert!(std::ptr::eq(occurrence.right_dialogue, &owned[4]));
        assert_eq!(occurrence.lines.len(), 6);
        assert_eq!(occurrence.parentheticals.len(), 1);
        let left_types: Vec<_> = occurrence
            .left_content
            .iter()
            .map(|item| (item.content_type, item.lines.len()))
            .collect();
        use crate::core::layout::dialogue_occurrences::DialogueContentType::{Dialogue, Parenthetical};
        assert_eq!(left_types, vec![(Parenthetical, 1), (Dialogue, 2)]);
        assert_eq!(result.reserved_lines.len(), 6);
        assert!(result.dialogue_occurrences.is_empty());
    }
}
