use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct DialOcc {
    pub lines: Vec<PhysicalTextLine>,
}
#[derive(Debug, Clone)]
pub struct ScreenplaySequences {
    pub transition_lines: Vec<PhysicalTextLine>,
    pub reserved_lines: Vec<PhysicalTextLine>,
    pub dialogue_occurrences: Vec<DialOcc>,
    pub dual_dialogue_occurrences: Vec<DialOcc>,
}

pub fn find_screenplay_sequences(lines: &[PhysicalTextLine], _tol: f64) -> ScreenplaySequences {
    ScreenplaySequences {
        transition_lines: Vec::new(),
        reserved_lines: Vec::new(),
        dialogue_occurrences: Vec::new(),
        dual_dialogue_occurrences: Vec::new(),
    }
}