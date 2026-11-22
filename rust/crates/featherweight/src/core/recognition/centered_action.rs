//! Port of `src/core/recognition/centered-action.ts`.
use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_layout::ScreenplayLayout;
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::recognition::recognition_geometry::{
    are_vertically_adjacent, has_baseline_sibling, is_isolated_from_role_geometry, line_center,
    matches_established_content_geometry, matches_established_cue_geometry, matches_established_role_geometry,
};

/// A centered pair that supports nearby centered lines (`supportedGroups`).
#[derive(Debug, Clone, Copy)]
struct SupportedGroup<'a> {
    center: f64,
    upper: &'a PhysicalTextLine,
    lower: &'a PhysicalTextLine,
}

/// The inclusive line-index range of a centered group.
#[derive(Debug, Clone, Copy)]
struct GroupRange {
    first_index: usize,
    last_index: usize,
}

/// Port of `findCenteredActionLines`.
pub fn find_centered_action_lines<'a>(
    lines: &[&'a PhysicalTextLine],
    blocked_lines: &LineSet<'a>,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> LineSet<'a> {
    let Some(action) = layout.action else {
        return LineSet::new();
    };

    let candidate_lines: Vec<&'a PhysicalTextLine> = lines
        .iter()
        .copied()
        .filter(|line| {
            !blocked_lines.contains(line)
                && line.bounds.x > action.x + line.bounds.height * 6.0
                && !has_baseline_sibling(line, lines, coordinate_tolerance)
        })
        .collect();
    let candidates: Vec<&'a PhysicalTextLine> = candidate_lines
        .iter()
        .copied()
        .filter(|line| !matches_established_role_geometry(line, layout, coordinate_tolerance))
        .collect();
    let candidate_set: LineSet<'a> = candidate_lines.iter().copied().collect();
    let mut centered_lines: LineSet<'a> = candidates
        .iter()
        .copied()
        .filter(|line| {
            is_at_established_screenplay_center(line, layout)
                && is_isolated_from_role_geometry(line, lines, layout, coordinate_tolerance)
        })
        .collect();
    let mut supported_groups: Vec<SupportedGroup<'a>> = Vec::new();

    for index in 0..lines.len().saturating_sub(1) {
        let upper = lines[index];
        let lower = lines[index + 1];

        if !is_centered_pair(upper, lower, &candidate_set, coordinate_tolerance)
            || matches_established_content_geometry(upper, layout, coordinate_tolerance)
            || matches_established_content_geometry(lower, layout, coordinate_tolerance)
            || (matches_established_cue_geometry(upper, layout, coordinate_tolerance.max(upper.bounds.height / 2.0))
                && matches_established_cue_geometry(
                    lower,
                    layout,
                    coordinate_tolerance.max(lower.bounds.height / 2.0),
                ))
        {
            continue;
        }

        let group_center = (line_center(upper) + line_center(lower)) / 2.0;

        if !is_centered_group_isolated_from_content_geometry(
            lines,
            index,
            group_center,
            &candidate_set,
            layout,
            coordinate_tolerance,
        ) {
            continue;
        }

        supported_groups.push(SupportedGroup { center: group_center, upper, lower });
        add_centered_group(lines, index, group_center, &candidate_set, &mut centered_lines);
    }

    for &line in &candidates {
        if is_isolated_from_role_geometry(line, lines, layout, coordinate_tolerance)
            && supported_groups
                .iter()
                .any(|group| (line_center(line) - group.center).abs() <= line.bounds.height / 4.0)
        {
            centered_lines.insert(line);
        }
    }

    for &line in &candidates {
        let has_stable_center_peer = candidates.iter().any(|candidate| {
            !std::ptr::eq(*candidate, line)
                && (line_center(candidate) - line_center(line)).abs() <= coordinate_tolerance
        });

        if has_stable_center_peer
            && is_at_established_screenplay_center(line, layout)
            && is_isolated_from_role_geometry(line, lines, layout, coordinate_tolerance)
        {
            centered_lines.insert(line);
        }
    }

    for index in 0..lines.len().saturating_sub(1) {
        let upper = lines[index];
        let lower = lines[index + 1];

        if !is_centered_pair(upper, lower, &candidate_set, coordinate_tolerance) {
            continue;
        }

        let group_center = (line_center(upper) + line_center(lower)) / 2.0;

        if supported_groups
            .iter()
            .any(|group| is_locally_supported_centered_pair(upper, lower, group, coordinate_tolerance))
            && is_centered_group_isolated_from_content_geometry(
                lines,
                index,
                group_center,
                &candidate_set,
                layout,
                coordinate_tolerance,
            )
        {
            add_centered_group(lines, index, group_center, &candidate_set, &mut centered_lines);
        }
    }

    centered_lines
}

fn is_centered_group_isolated_from_content_geometry(
    lines: &[&PhysicalTextLine],
    seed_index: usize,
    group_center: f64,
    candidates: &LineSet<'_>,
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> bool {
    let range = find_centered_group_range(lines, seed_index, group_center, candidates);
    let first_line = lines[range.first_index];
    let last_line = lines[range.last_index];
    let preceding_line = range.first_index.checked_sub(1).map(|index| lines[index]);
    let following_line = lines.get(range.last_index + 1).copied();

    ![preceding_line, following_line].into_iter().flatten().any(|line| {
        (are_vertically_adjacent(line, first_line, 1.5) || are_vertically_adjacent(last_line, line, 1.5))
            && matches_established_content_geometry(line, layout, coordinate_tolerance)
    })
}

fn is_locally_supported_centered_pair(
    upper: &PhysicalTextLine,
    lower: &PhysicalTextLine,
    support: &SupportedGroup<'_>,
    coordinate_tolerance: f64,
) -> bool {
    if upper.page_index != support.upper.page_index
        || ((line_center(upper) + line_center(lower)) / 2.0 - support.center).abs() > coordinate_tolerance
    {
        return false;
    }

    let vertical_distance = (upper.bounds.y - support.upper.bounds.y)
        .abs()
        .min((upper.bounds.y - support.lower.bounds.y).abs())
        .min((lower.bounds.y - support.upper.bounds.y).abs())
        .min((lower.bounds.y - support.lower.bounds.y).abs());
    let line_height = upper
        .bounds
        .height
        .max(lower.bounds.height)
        .max(support.upper.bounds.height)
        .max(support.lower.bounds.height);

    vertical_distance <= line_height * 2.5
}

fn is_centered_pair(
    upper: &PhysicalTextLine,
    lower: &PhysicalTextLine,
    candidates: &LineSet<'_>,
    coordinate_tolerance: f64,
) -> bool {
    candidates.contains(upper)
        && candidates.contains(lower)
        && are_vertically_adjacent(upper, lower, 1.5)
        && (line_center(upper) - line_center(lower)).abs() <= coordinate_tolerance
        && (upper.bounds.x - lower.bounds.x).abs() > coordinate_tolerance
}

fn add_centered_group<'a>(
    lines: &[&'a PhysicalTextLine],
    seed_index: usize,
    group_center: f64,
    candidates: &LineSet<'a>,
    centered_lines: &mut LineSet<'a>,
) {
    let range = find_centered_group_range(lines, seed_index, group_center, candidates);

    for &line in &lines[range.first_index..=range.last_index] {
        centered_lines.insert(line);
    }
}

fn find_centered_group_range(
    lines: &[&PhysicalTextLine],
    seed_index: usize,
    group_center: f64,
    candidates: &LineSet<'_>,
) -> GroupRange {
    let mut first_index = seed_index;
    let mut last_index = seed_index + 1;

    while first_index > 0
        && belongs_to_centered_group(lines[first_index - 1], lines[first_index], group_center, candidates)
    {
        first_index -= 1;
    }

    while last_index + 1 < lines.len()
        && belongs_to_centered_group(lines[last_index], lines[last_index + 1], group_center, candidates)
    {
        last_index += 1;
    }

    GroupRange { first_index, last_index }
}

fn belongs_to_centered_group(
    upper: &PhysicalTextLine,
    lower: &PhysicalTextLine,
    group_center: f64,
    candidates: &LineSet<'_>,
) -> bool {
    candidates.contains(upper)
        && candidates.contains(lower)
        && are_vertically_adjacent(upper, lower, 1.5)
        && (line_center(lower) - group_center).abs() <= lower.bounds.height / 4.0
}

fn is_at_established_screenplay_center(line: &PhysicalTextLine, layout: &ScreenplayLayout) -> bool {
    let Some(action) = layout.action else {
        return false;
    };

    [16.5, 18.0]
        .map(|character_measure| action.x + line.bounds.height * character_measure)
        .iter()
        .any(|expected_center| (line_center(line) - expected_center).abs() <= line.bounds.height)
}
