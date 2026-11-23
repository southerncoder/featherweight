//! Port of `src/pdf/normalization/physical-text.ts`.
use crate::core::normalized_text::{
    NormalizationDiagnostic, NormalizedText, NormalizedTextPage, SuppressedPhysicalTextLine,
};
use crate::core::physical_lines::{PhysicalText, PhysicalTextLine};
use crate::normalization::page_artifacts::detect_page_artifacts;

/// Port of `normalizePhysicalText`: splits lines into active page lines and
/// suppressed page artifacts, and keeps diagnostics for the active lines only.
pub fn normalize_physical_text(physical_text: &PhysicalText, represented_page_indexes: &[usize]) -> NormalizedText {
    let decisions = detect_page_artifacts(physical_text);
    // `Map<number, PhysicalTextLine[]>` in insertion order.
    let mut active_lines_by_page: Vec<(usize, Vec<PhysicalTextLine>)> = Vec::new();

    for &page_index in represented_page_indexes {
        if !active_lines_by_page.iter().any(|(existing, _)| *existing == page_index) {
            active_lines_by_page.push((page_index, Vec::new()));
        }
    }

    let mut suppressed: Vec<SuppressedPhysicalTextLine> = Vec::new();
    let mut diagnostics: Vec<NormalizationDiagnostic> = Vec::new();

    for (line_index, line) in physical_text.lines.iter().enumerate() {
        let page_position = match active_lines_by_page
            .iter()
            .position(|(page_index, _)| *page_index == line.page_index)
        {
            Some(position) => position,
            None => {
                active_lines_by_page.push((line.page_index, Vec::new()));
                active_lines_by_page.len() - 1
            }
        };

        if let Some(reasons) = &decisions.suppression_reasons[line_index] {
            suppressed.push(SuppressedPhysicalTextLine {
                line: line.clone(),
                reasons: reasons.clone(),
            });
            continue;
        }

        active_lines_by_page[page_position].1.push(line.clone());

        if let Some(diagnostic) = &decisions.diagnostics[line_index] {
            diagnostics.push(diagnostic.clone());
        }
    }

    // Stable sort by page index, as the TS comparator `left - right`.
    active_lines_by_page.sort_by_key(|(page_index, _)| *page_index);
    let pages = active_lines_by_page
        .into_iter()
        .map(|(page_index, lines)| NormalizedTextPage { page_index, lines })
        .collect();

    NormalizedText {
        pages,
        suppressed,
        diagnostics,
    }
}
