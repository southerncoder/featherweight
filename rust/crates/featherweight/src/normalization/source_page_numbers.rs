//! Port of `src/pdf/normalization/source-page-numbers.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::normalized_text::SuppressionReason;
use crate::core::page_extents::{PageExtentMeasurements, get_page_horizontal_midpoint, is_line_in_top_page_band};
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::text::js_trim;
use crate::normalization::page_artifact_decisions::{SuppressionReasons, add_suppression_reason};

struct NumberedPageCandidate<'a> {
    line_index: usize,
    line: &'a PhysicalTextLine,
    value: f64,
}

/// Port of `pageNumberPattern`: `/^(\d+)\.$/` (JS `\d` is ASCII).
static PAGE_NUMBER_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+)\.$").expect("valid regex"));

/// Port of `detectSourcePageNumbers`.
pub fn detect_source_page_numbers(
    lines: &[PhysicalTextLine],
    geometry: &PageExtentMeasurements,
    suppression_reasons: &mut SuppressionReasons,
) {
    let candidates: Vec<NumberedPageCandidate<'_>> = lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, line)| {
            let page_number = PAGE_NUMBER_PATTERN.captures(js_trim(&line.text))?;

            if !is_line_in_top_page_band(line, geometry) {
                return None;
            }

            let horizontal_midpoint = get_page_horizontal_midpoint(geometry, line.page_index)?;

            if line.bounds.x < horizontal_midpoint {
                return None;
            }

            // `Number(digits)`: Rust's float parsing rounds the same way.
            let value = page_number[1].parse::<f64>().expect("digits parse as a number");
            Some(NumberedPageCandidate { line_index, line, value })
        })
        .collect();

    for index in 0..candidates.len() {
        let candidate = &candidates[index];
        let previous = index.checked_sub(1).map(|previous_index| &candidates[previous_index]);
        let next = candidates.get(index + 1);

        if !are_sequential_page_numbers(previous, Some(candidate))
            && !are_sequential_page_numbers(Some(candidate), next)
        {
            continue;
        }

        add_suppression_reason(suppression_reasons, candidate.line_index, SuppressionReason::SourcePageNumber);
    }
}

fn are_sequential_page_numbers(
    left: Option<&NumberedPageCandidate<'_>>,
    right: Option<&NumberedPageCandidate<'_>>,
) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };

    right.line.page_index == left.line.page_index + 1 && right.value == left.value + 1.0
}
