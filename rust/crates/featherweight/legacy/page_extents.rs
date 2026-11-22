use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct ContentExtents {
    pub maximum_right: f64,
    pub maximum_y: f64,
    pub minimum_x: f64,
    pub minimum_y: f64,
}

#[derive(Debug, Clone)]
pub struct PageExtentMeasurements {
    pub document_extents: Option<ContentExtents>,
    pub page_extents: std::collections::BTreeMap<usize, ContentExtents>,
    pub page_line_counts: std::collections::BTreeMap<usize, usize>,
    pub typical_font_size: f64,
}

const MIN_RELIABLE_PAGE_LINE_COUNT: usize = 6;
const MARGIN_BAND_FONT_DEPTH: f64 = 4.0;

fn calculate_extents(lines: &[PhysicalTextLine]) -> Option<ContentExtents> {
    if lines.is_empty() { return None; }
    Some(ContentExtents {
        maximum_right: lines.iter().map(|l| l.x + l.width).fold(f64::NEG_INFINITY, f64::max),
        maximum_y: lines.iter().map(|l| l.y).fold(f64::NEG_INFINITY, f64::max),
        minimum_x: lines.iter().map(|l| l.x).fold(f64::INFINITY, f64::min),
        minimum_y: lines.iter().map(|l| l.y).fold(f64::INFINITY, f64::min),
    })
}

pub fn measure_page_extents(lines: &[PhysicalTextLine]) -> PageExtentMeasurements {
    // fallback to height if font_size unavailable
    let font_sizes: Vec<f64> = lines.iter().map(|l| l.height).collect();
    let mut fs = font_sizes.clone(); fs.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let typical = if fs.is_empty() { 12.0 } else { fs[fs.len()/2] };
    let document_extents = calculate_extents(lines);
    let mut by_page: std::collections::BTreeMap<usize, Vec<&PhysicalTextLine>> = std::collections::BTreeMap::new();
    for l in lines { by_page.entry(l.page_index).or_default().push(l); }
    let mut page_extents = std::collections::BTreeMap::new();
    let mut page_line_counts = std::collections::BTreeMap::new();
    for (pi, pls) in &by_page {
        page_line_counts.insert(*pi, pls.len());
        if let Some(e) = calculate_extents(&pls.iter().copied().cloned().collect::<Vec<_>>()) {
            page_extents.insert(*pi, e);
        }
    }
    PageExtentMeasurements { document_extents, page_extents, page_line_counts, typical_font_size: typical }
}

pub fn get_effective_page_extents(m: &PageExtentMeasurements, page_index: usize) -> Option<ContentExtents> {
    let count = m.page_line_counts.get(&page_index).copied().unwrap_or(0);
    if count < MIN_RELIABLE_PAGE_LINE_COUNT { m.document_extents.clone() }
    else { m.page_extents.get(&page_index).cloned().or_else(|| m.document_extents.clone()) }
}

pub fn page_horizontal_midpoint(m: &PageExtentMeasurements, page_index: usize) -> Option<f64> {
    get_effective_page_extents(m, page_index).map(|e| (e.minimum_x + e.maximum_right) / 2.0)
}

pub fn is_line_in_top_page_band(line: &PhysicalTextLine, m: &PageExtentMeasurements) -> bool {
    let e = match get_effective_page_extents(m, line.page_index) { Some(e) => e, None => return false };
    let depth = m.typical_font_size * MARGIN_BAND_FONT_DEPTH;
    line.y >= e.maximum_y - depth
}

pub fn is_line_in_bottom_page_band(line: &PhysicalTextLine, m: &PageExtentMeasurements) -> bool {
    let e = match get_effective_page_extents(m, line.page_index) { Some(e) => e, None => return false };
    let depth = m.typical_font_size * MARGIN_BAND_FONT_DEPTH;
    line.y <= e.minimum_y + depth
}

pub fn page_margin_band_depth(m: &PageExtentMeasurements) -> f64 {
    m.typical_font_size * MARGIN_BAND_FONT_DEPTH
}

pub fn is_line_in_observed_bottom_page_band(line: &PhysicalTextLine, m: &PageExtentMeasurements) -> bool {
    let e = m.page_extents.get(&line.page_index).cloned().or(m.document_extents.clone());
    match e { Some(e) => line.y <= e.minimum_y + page_margin_band_depth(m), None => false }
}