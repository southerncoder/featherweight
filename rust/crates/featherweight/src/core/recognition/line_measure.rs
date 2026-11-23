//! Port of `src/core/recognition/line-measure.ts`.
use crate::core::physical_lines::PhysicalTextLine;

/// Port of `visibleLineShortfallMultiplier`.
pub const VISIBLE_LINE_SHORTFALL_MULTIPLIER: f64 = 5.0;

/// Port of `inferLineMeasure`: the 90th-percentile width of the lines that
/// start at `anchor_x`.
pub fn infer_line_measure(lines: &[&PhysicalTextLine], anchor_x: f64, coordinate_tolerance: f64) -> Option<f64> {
    let mut widths: Vec<f64> = lines
        .iter()
        .filter(|line| (line.bounds.x - anchor_x).abs() <= coordinate_tolerance)
        .map(|line| line.bounds.width)
        .collect();
    widths.sort_by(|left, right| compare_numbers(left - right));

    if widths.is_empty() {
        return None;
    }

    Some(widths[((widths.len() - 1) as f64 * 0.9).floor() as usize])
}

/// The ordering a JavaScript numeric sort comparator (`(a, b) => a - b`)
/// expresses through the sign of its result; `NaN` counts as `+0`.
pub(crate) fn compare_numbers(difference: f64) -> std::cmp::Ordering {
    if difference < 0.0 {
        std::cmp::Ordering::Less
    } else if difference > 0.0 {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}
