//! Port of `src/pdf/normalization/page-artifacts.ts`.
use crate::core::page_extents::measure_page_extents;
use crate::core::physical_lines::PhysicalText;
use crate::normalization::page_artifact_decisions::{ArtifactDiagnostics, SuppressionReasons};
use crate::normalization::recurring_artifacts::detect_recurring_artifacts;
use crate::normalization::screenplay_pagination::{
    detect_continued_character_cues, detect_explicit_pagination_markers,
};
use crate::normalization::source_page_numbers::detect_source_page_numbers;

/// Port of `PageArtifactDecisions`. Both maps are indexed by line index in
/// `PhysicalText::lines` and always have one slot per line.
#[derive(Debug, Clone, Default)]
pub struct PageArtifactDecisions {
    pub diagnostics: ArtifactDiagnostics,
    pub suppression_reasons: SuppressionReasons,
}

/// Port of `detectPageArtifacts`.
pub fn detect_page_artifacts(physical_text: &PhysicalText) -> PageArtifactDecisions {
    let lines = &physical_text.lines;
    let mut suppression_reasons: SuppressionReasons = vec![None; lines.len()];
    let mut diagnostics: ArtifactDiagnostics = vec![None; lines.len()];

    if lines.is_empty() {
        return PageArtifactDecisions {
            diagnostics,
            suppression_reasons,
        };
    }

    let geometry = measure_page_extents(lines);

    detect_recurring_artifacts(lines, &geometry, &mut suppression_reasons, &mut diagnostics);
    detect_source_page_numbers(lines, &geometry, &mut suppression_reasons);
    detect_explicit_pagination_markers(lines, &geometry, &mut suppression_reasons, &mut diagnostics);
    detect_continued_character_cues(lines, &geometry, &mut suppression_reasons, &mut diagnostics);

    PageArtifactDecisions {
        diagnostics,
        suppression_reasons,
    }
}
