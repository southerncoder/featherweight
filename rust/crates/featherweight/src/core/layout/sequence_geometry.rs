//! Port of `src/core/layout/sequence-geometry.ts`.
use crate::core::physical_lines::PhysicalTextLine;

/// `shareBaseline`.
pub fn share_baseline(left: &PhysicalTextLine, right: &PhysicalTextLine, coordinate_tolerance: f64) -> bool {
    (left.bounds.y - right.bounds.y).abs() <= coordinate_tolerance
}

/// `isVerticallyAdjacent`.
pub fn is_vertically_adjacent(upper: &PhysicalTextLine, lower: &PhysicalTextLine, height_multiplier: f64) -> bool {
    if upper.page_index != lower.page_index {
        return false;
    }

    let vertical_gap = upper.bounds.y - lower.bounds.y;
    let local_height = js_math_max(upper.bounds.height, lower.bounds.height);

    vertical_gap > 0.0 && vertical_gap <= local_height * height_multiplier
}

/// Two-argument `Math.max`: NaN-propagating, and `+0` beats `-0`.
pub(crate) fn js_math_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left > right {
        left
    } else if right > left {
        right
    } else if left.is_sign_positive() {
        left
    } else {
        right
    }
}

/// Two-argument `Math.min`: NaN-propagating, and `-0` beats `+0`.
pub(crate) fn js_math_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left < right {
        left
    } else if right < left {
        right
    } else if left.is_sign_negative() {
        left
    } else {
        right
    }
}
