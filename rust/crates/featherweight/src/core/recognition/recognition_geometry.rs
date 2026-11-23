//! Port of `src/core/recognition/recognition-geometry.ts`.
use crate::core::layout::screenplay_layout::{LayoutAnchor, ScreenplayLayout};
use crate::core::physical_lines::PhysicalTextLine;

/// Port of `matchesEstablishedRoleGeometry`.
pub fn matches_established_role_geometry(
    line: &PhysicalTextLine,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    let cue_tolerance = coordinate_tolerance.max(line.bounds.height);

    matches_established_cue_geometry(line, layout, cue_tolerance)
        || matches_established_content_geometry(line, layout, coordinate_tolerance)
}

/// Port of `matchesEstablishedCueGeometry`.
pub fn matches_established_cue_geometry(line: &PhysicalTextLine, layout: &ScreenplayLayout, tolerance: f64) -> bool {
    let cue_anchors: [Option<LayoutAnchor>; 3] = [
        layout.character_cue,
        layout.dual_dialogue.map(|dual| dual.left.character_cue),
        layout.dual_dialogue.map(|dual| dual.right.character_cue),
    ];

    cue_anchors
        .iter()
        .flatten()
        .any(|anchor| (line.bounds.x - anchor.x).abs() <= tolerance)
}

/// Port of `matchesEstablishedContentGeometry`.
pub fn matches_established_content_geometry(
    line: &PhysicalTextLine,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    let role_tolerance = coordinate_tolerance.max(line.bounds.height);
    let content_anchors: [Option<LayoutAnchor>; 4] = [
        layout.dialogue,
        layout.parenthetical,
        layout.dual_dialogue.map(|dual| dual.left.dialogue),
        layout.dual_dialogue.map(|dual| dual.right.dialogue),
    ];

    if content_anchors
        .iter()
        .flatten()
        .any(|anchor| (line.bounds.x - anchor.x).abs() <= role_tolerance)
    {
        return true;
    }

    layout
        .transition
        .is_some_and(|transition| (line.bounds.x + line.bounds.width - transition.x).abs() <= role_tolerance)
}

/// Port of `hasBaselineSibling`.
pub fn has_baseline_sibling(line: &PhysicalTextLine, lines: &[&PhysicalTextLine], coordinate_tolerance: f64) -> bool {
    lines.iter().any(|candidate| {
        !std::ptr::eq(*candidate, line)
            && candidate.page_index == line.page_index
            && (candidate.bounds.y - line.bounds.y).abs() <= coordinate_tolerance
    })
}

/// Port of `isIsolatedFromRoleGeometry`.
pub fn is_isolated_from_role_geometry(
    line: &PhysicalTextLine,
    lines: &[&PhysicalTextLine],
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    !lines.iter().any(|candidate| {
        !std::ptr::eq(*candidate, line)
            && (are_vertically_adjacent(candidate, line, 1.5) || are_vertically_adjacent(line, candidate, 1.5))
            && matches_established_role_geometry(candidate, layout, coordinate_tolerance)
    })
}

/// Port of `lineCenter`.
pub fn line_center(line: &PhysicalTextLine) -> f64 {
    line.bounds.x + line.bounds.width / 2.0
}

/// Port of `verticalGapInLineHeights`.
pub fn vertical_gap_in_line_heights(upper: &PhysicalTextLine, lower: &PhysicalTextLine) -> f64 {
    let height = upper.bounds.height.max(lower.bounds.height);

    (upper.bounds.y - lower.bounds.y) / height
}

/// Port of `areVerticallyAdjacent`.
pub fn are_vertically_adjacent(upper: &PhysicalTextLine, lower: &PhysicalTextLine, height_multiplier: f64) -> bool {
    if upper.page_index != lower.page_index {
        return false;
    }

    let vertical_gap = upper.bounds.y - lower.bounds.y;
    let height = upper.bounds.height.max(lower.bounds.height);

    vertical_gap > 0.0 && vertical_gap <= height * height_multiplier
}
