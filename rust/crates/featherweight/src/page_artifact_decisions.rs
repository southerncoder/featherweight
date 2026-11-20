use crate::model::PhysicalTextLine;
use crate::normalized_text::{NormalizationDiagnostic, SuppressionReason};

pub type SuppressionSet = std::collections::BTreeSet<SuppressionReason>;

pub fn add_suppression_reason(
    sets: &mut std::collections::BTreeMap<usize, std::collections::BTreeSet<SuppressionReason>>,
    line_index: usize,
    reason: SuppressionReason,
) {
    sets.entry(line_index).or_default().insert(reason);
}

pub fn add_artifact_diagnostic(
    diags: &mut std::collections::BTreeMap<usize, NormalizationDiagnostic>,
    line_index: usize,
    kind: &str,
) {
    diags.insert(line_index, NormalizationDiagnostic { kind: kind.to_string() });
}

pub fn add_pagination_diagnostic(
    diags: &mut std::collections::BTreeMap<usize, NormalizationDiagnostic>,
    line_index: usize,
) {
    diags.insert(line_index, NormalizationDiagnostic { kind: "screenplay-pagination".to_string() });
}