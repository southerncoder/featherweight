use crate::dialogue_occurrences::{find_dialogue_occurrences, find_dual_dialogue_occurrences, DialogueOccurrence, DualDialogueOccurrence};
use crate::model::PhysicalTextLine;
use crate::transition_evidence::{find_contextual_transition_candidates, find_repeated_transition_lines, find_transition_lines, resolve_repeated_transition_lines};

#[derive(Debug, Clone)]
pub struct ScreenplaySequences {
    pub transition_lines: Vec<PhysicalTextLine>,
    pub dual_dialogue_occurrences: Vec<DualDialogueOccurrence>,
    pub dialogue_occurrences: Vec<DialogueOccurrence>,
    pub reserved_lines: Vec<PhysicalTextLine>,
}

pub fn find_screenplay_sequences(lines: &[PhysicalTextLine], tol: f64) -> ScreenplaySequences {
    let mut reserved: Vec<PhysicalTextLine> = Vec::new();
    let dual = find_dual_dialogue_occurrences(lines, tol);
    for occ in &dual { for l in &occ.lines { if !reserved.iter().any(|r| std::ptr::eq(r, l)) { reserved.push(l.clone()); } } }

    let dialogue_candidates = find_dialogue_occurrences(lines, &reserved, tol);
    let repeated_candidates = find_repeated_transition_lines(lines, &reserved, tol);
    let repeated_transition_lines = resolve_repeated_transition_lines(lines, &repeated_candidates, &dialogue_candidates, tol);
    let contextual_transition_lines = find_contextual_transition_candidates(lines, &reserved, &dialogue_candidates, &repeated_transition_lines);
    let mut arbitrated: Vec<PhysicalTextLine> = repeated_transition_lines.clone();
    for l in &contextual_transition_lines { if !arbitrated.iter().any(|r| std::ptr::eq(r, l)) { arbitrated.push(l.clone()); } }

    let arbitrated_set_ptrs: Vec<*const PhysicalTextLine> = arbitrated.iter().map(|l| l as *const PhysicalTextLine).collect();
    let dialogue_occurrences: Vec<DialogueOccurrence> = dialogue_candidates.into_iter().filter(|occ| {
        !arbitrated_set_ptrs.contains(&(&occ.character_cue as *const PhysicalTextLine))
    }).collect();

    for l in &arbitrated { if !reserved.iter().any(|r| std::ptr::eq(r, l)) { reserved.push(l.clone()); } }
    for occ in &dialogue_occurrences {
        if !reserved.iter().any(|r| std::ptr::eq(r, &occ.character_cue)) { reserved.push(occ.character_cue.clone()); }
        for l in &occ.dialogue_lines { if !reserved.iter().any(|r| std::ptr::eq(r, l)) { reserved.push(l.clone()); } }
        for l in &occ.parentheticals { if !reserved.iter().any(|r| std::ptr::eq(r, l)) { reserved.push(l.clone()); } }
    }

    let transition_lines = find_transition_lines(lines, &reserved, &arbitrated, &dialogue_occurrences, tol);
    for l in &transition_lines { if !reserved.iter().any(|r| std::ptr::eq(r, l)) { reserved.push(l.clone()); } }

    ScreenplaySequences {
        transition_lines,
        dual_dialogue_occurrences: dual,
        dialogue_occurrences,
        reserved_lines: reserved,
    }
}