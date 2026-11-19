use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct TransitionEvidence {
    pub lines: Vec<PhysicalTextLine>,
}

pub fn find_repeated_transition_lines(
    _lines: &[PhysicalTextLine],
    _reserved: &[PhysicalTextLine],
    _tol: f64,
) -> Vec<PhysicalTextLine> { Vec::new() }

pub fn find_contextual_transition_candidates(
    _lines: &[PhysicalTextLine],
    _reserved: &[PhysicalTextLine],
    _dialogue: &[PhysicalTextLine],
    _repeated: &[PhysicalTextLine],
) -> Vec<PhysicalTextLine> { Vec::new() }

pub fn resolve_repeated_transition_lines(
    _lines: &[PhysicalTextLine],
    repeated: &[PhysicalTextLine],
    _dialogue: &[PhysicalTextLine],
    _tol: f64,
) -> Vec<PhysicalTextLine> { repeated.to_vec() }

pub fn find_transition_lines(
    _lines: &[PhysicalTextLine],
    _reserved: &[PhysicalTextLine],
    _arbitrated: &[PhysicalTextLine],
    _dialogue: &[PhysicalTextLine],
    _tol: f64,
) -> Vec<PhysicalTextLine> { Vec::new() }