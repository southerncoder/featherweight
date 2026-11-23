//! Port of `src/core/page-extents.ts`.
use std::collections::HashMap;

use crate::core::physical_lines::PhysicalTextLine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentExtents {
    pub maximum_right: f64,
    pub maximum_y: f64,
    pub minimum_x: f64,
    pub minimum_y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageExtentMeasurements {
    pub document_extents: Option<ContentExtents>,
    pub page_extents: HashMap<usize, ContentExtents>,
    pub page_line_counts: HashMap<usize, usize>,
    pub typical_font_size: f64,
}

const MINIMUM_RELIABLE_PAGE_LINE_COUNT: usize = 6;
const MARGIN_BAND_FONT_DEPTH: f64 = 4.0;

/// Accepts any iterator of lines (TypeScript takes `readonly PhysicalTextLine[]`;
/// callers here often hold `Vec<&PhysicalTextLine>`).
pub fn measure_page_extents<'a>(lines: impl IntoIterator<Item = &'a PhysicalTextLine>) -> PageExtentMeasurements {
    let lines: Vec<&PhysicalTextLine> = lines.into_iter().collect();
    let mut font_sizes: Vec<f64> = lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.font.size))
        .collect();
    font_sizes.sort_by(f64::total_cmp);
    let typical_font_size = font_sizes.get(font_sizes.len() / 2).copied().unwrap_or(12.0);
    let document_extents = calculate_content_extents(&lines);
    let mut page_extents = HashMap::new();
    let mut page_line_counts = HashMap::new();
    let mut lines_by_page: Vec<(usize, Vec<&PhysicalTextLine>)> = Vec::new();

    for line in &lines {
        match lines_by_page
            .iter_mut()
            .find(|(page_index, _)| *page_index == line.page_index)
        {
            Some((_, page_lines)) => page_lines.push(line),
            None => lines_by_page.push((line.page_index, vec![line])),
        }
    }

    for (page_index, page_lines) in &lines_by_page {
        page_line_counts.insert(*page_index, page_lines.len());

        if let Some(extents) = calculate_content_extents(page_lines) {
            page_extents.insert(*page_index, extents);
        }
    }

    PageExtentMeasurements {
        document_extents,
        page_extents,
        page_line_counts,
        typical_font_size,
    }
}

pub fn get_effective_page_extents(measurements: &PageExtentMeasurements, page_index: usize) -> Option<ContentExtents> {
    let page_line_count = measurements.page_line_counts.get(&page_index).copied().unwrap_or(0);

    if page_line_count < MINIMUM_RELIABLE_PAGE_LINE_COUNT {
        return measurements.document_extents;
    }

    measurements
        .page_extents
        .get(&page_index)
        .copied()
        .or(measurements.document_extents)
}

pub fn get_page_horizontal_midpoint(measurements: &PageExtentMeasurements, page_index: usize) -> Option<f64> {
    let extents = get_effective_page_extents(measurements, page_index)?;
    Some((extents.minimum_x + extents.maximum_right) / 2.0)
}

pub fn is_line_in_top_page_band(line: &PhysicalTextLine, measurements: &PageExtentMeasurements) -> bool {
    let Some(extents) = get_effective_page_extents(measurements, line.page_index) else {
        return false;
    };
    let band_depth = measurements.typical_font_size * MARGIN_BAND_FONT_DEPTH;
    line.bounds.y >= extents.maximum_y - band_depth
}

pub fn is_line_in_bottom_page_band(line: &PhysicalTextLine, measurements: &PageExtentMeasurements) -> bool {
    let Some(extents) = get_effective_page_extents(measurements, line.page_index) else {
        return false;
    };
    let band_depth = measurements.typical_font_size * MARGIN_BAND_FONT_DEPTH;
    line.bounds.y <= extents.minimum_y + band_depth
}

pub fn page_margin_band_depth(measurements: &PageExtentMeasurements) -> f64 {
    measurements.typical_font_size * MARGIN_BAND_FONT_DEPTH
}

pub fn is_line_in_observed_bottom_page_band(line: &PhysicalTextLine, measurements: &PageExtentMeasurements) -> bool {
    let extents = measurements
        .page_extents
        .get(&line.page_index)
        .copied()
        .or(measurements.document_extents);
    let Some(extents) = extents else {
        return false;
    };
    line.bounds.y <= extents.minimum_y + page_margin_band_depth(measurements)
}

pub fn is_line_in_page_margin_band(line: &PhysicalTextLine, measurements: &PageExtentMeasurements) -> bool {
    is_line_in_top_page_band(line, measurements) || is_line_in_bottom_page_band(line, measurements)
}

fn calculate_content_extents(lines: &[&PhysicalTextLine]) -> Option<ContentExtents> {
    let first_line = lines.first()?;
    let mut extents = ContentExtents {
        maximum_right: first_line.bounds.x + first_line.bounds.width,
        maximum_y: first_line.bounds.y,
        minimum_x: first_line.bounds.x,
        minimum_y: first_line.bounds.y,
    };

    for line in &lines[1..] {
        extents.maximum_right = extents.maximum_right.max(line.bounds.x + line.bounds.width);
        extents.maximum_y = extents.maximum_y.max(line.bounds.y);
        extents.minimum_x = extents.minimum_x.min(line.bounds.x);
        extents.minimum_y = extents.minimum_y.min(line.bounds.y);
    }

    Some(extents)
}
