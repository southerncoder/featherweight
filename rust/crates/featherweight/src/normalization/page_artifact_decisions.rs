//! Port of `src/pdf/normalization/page-artifact-decisions.ts`.
use crate::core::normalized_text::{NormalizationDiagnostic, NormalizationDiagnosticCode, SuppressionReason};
use crate::core::physical_lines::PhysicalTextLine;

/// `Map<PhysicalTextLine, Set<SuppressionReason>>`, indexed by line index.
///
/// Each present entry holds its reasons in Set insertion order, without duplicates.
pub type SuppressionReasons = Vec<Option<Vec<SuppressionReason>>>;

/// `Map<PhysicalTextLine, NormalizationDiagnostic>`, indexed by line index.
pub type ArtifactDiagnostics = Vec<Option<NormalizationDiagnostic>>;

/// Port of `addSuppressionReason`.
pub fn add_suppression_reason(
    suppression_reasons: &mut SuppressionReasons,
    line_index: usize,
    reason: SuppressionReason,
) {
    match &mut suppression_reasons[line_index] {
        Some(reasons) => {
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
        }
        entry @ None => *entry = Some(vec![reason]),
    }
}

/// Port of `addPaginationDiagnostic`.
pub fn add_pagination_diagnostic(diagnostics: &mut ArtifactDiagnostics, line_index: usize, line: &PhysicalTextLine) {
    add_artifact_diagnostic(diagnostics, line_index, line, SuppressionReason::ScreenplayPagination);
}

/// Port of `addArtifactDiagnostic`.
pub fn add_artifact_diagnostic(
    diagnostics: &mut ArtifactDiagnostics,
    line_index: usize,
    line: &PhysicalTextLine,
    candidate_reason: SuppressionReason,
) {
    diagnostics[line_index] = Some(NormalizationDiagnostic {
        code: NormalizationDiagnosticCode::AmbiguousPageArtifact,
        page_index: line.page_index,
        source_indexes: line.spans.iter().map(|span| span.source_index).collect(),
        candidate_reasons: vec![candidate_reason],
    });
}
