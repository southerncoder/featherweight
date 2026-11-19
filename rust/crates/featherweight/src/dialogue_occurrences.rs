use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct DialogueOccurrence {
    pub character_cue: PhysicalTextLine,
    pub dialogue_lines: Vec<PhysicalTextLine>,
    pub parentheticals: Vec<PhysicalTextLine>,
}
#[derive(Debug, Clone)]
pub struct DualDialogueOccurrence {
    pub left_character_cue: PhysicalTextLine,
    pub left_dialogue: PhysicalTextLine,
    pub right_character_cue: PhysicalTextLine,
    pub right_dialogue: PhysicalTextLine,
    pub lines: Vec<PhysicalTextLine>,
    pub parentheticals: Vec<PhysicalTextLine>,
}

pub fn find_dialogue_occurrences(
    _lines: &[PhysicalTextLine],
    _reserved: &[PhysicalTextLine],
    _tol: f64,
) -> Vec<DialogueOccurrence> {
    Vec::new()
}

pub fn find_dual_dialogue_occurrences(
    _lines: &[PhysicalTextLine],
    _tol: f64,
) -> Vec<DualDialogueOccurrence> {
    Vec::new()
}