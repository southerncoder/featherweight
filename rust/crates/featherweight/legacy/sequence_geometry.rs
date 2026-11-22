use crate::model::PhysicalTextLine;

pub fn share_baseline(left: &PhysicalTextLine, right: &PhysicalTextLine, tol: f64) -> bool {
    (left.y - right.y).abs() <= tol
}

pub fn is_vertically_adjacent(upper: &PhysicalTextLine, lower: &PhysicalTextLine, mult: f64) -> bool {
    if upper.page_index != lower.page_index { return false; }
    let gap = upper.y - lower.y;
    let height = upper.height.max(lower.height);
    gap > 0.0 && gap <= height * mult
}