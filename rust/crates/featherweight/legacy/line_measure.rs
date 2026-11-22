use crate::model::PhysicalTextLine;

pub const VISIBLE_LINE_SHORTFALL_MULTIPLIER: f64 = 5.0;

pub fn infer_line_measure(lines: &[PhysicalTextLine], anchor_x: f64, tol: f64) -> Option<f64> {
    let mut widths: Vec<f64> = lines.iter()
        .filter(|l| (l.x - anchor_x).abs() <= tol)
        .map(|l| l.width)
        .collect();
    widths.sort_by(|a,b| a.partial_cmp(b).unwrap());
    if widths.is_empty() { return None; }
    Some(widths[((widths.len() - 1) * 9 / 10)])
}