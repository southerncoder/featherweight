//! Port of `src/core/normalized-text.ts`.
use serde::{Deserialize, Serialize};

use crate::core::physical_lines::PhysicalTextLine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SuppressionReason {
    RecurringPageArtifact,
    SourcePageNumber,
    ScreenplayPagination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizationDiagnosticCode {
    #[serde(rename = "AMBIGUOUS_PAGE_ARTIFACT")]
    AmbiguousPageArtifact,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedText {
    pub pages: Vec<NormalizedTextPage>,
    pub suppressed: Vec<SuppressedPhysicalTextLine>,
    pub diagnostics: Vec<NormalizationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedTextPage {
    pub page_index: usize,
    pub lines: Vec<PhysicalTextLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppressedPhysicalTextLine {
    pub line: PhysicalTextLine,
    pub reasons: Vec<SuppressionReason>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationDiagnostic {
    pub code: NormalizationDiagnosticCode,
    pub page_index: usize,
    pub source_indexes: Vec<usize>,
    pub candidate_reasons: Vec<SuppressionReason>,
}
