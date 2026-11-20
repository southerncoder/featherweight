use crate::model::PhysicalTextLine;
use crate::normalized_text::{NormalizedText, NormalizedTextPage};
use crate::page_extents::{
    get_effective_page_extents, is_line_in_bottom_page_band, is_line_in_top_page_band, measure_page_extents, page_margin_band_depth,
};

pub fn normalize_physical_text(lines: &[PhysicalTextLine], represented: &[usize]) -> NormalizedText {
    let mut suppressed = vec![false; lines.len()];
    let geometry = measure_page_extents(lines);

    // Source page numbers (top band, sequential numeric suffix).
    detect_source_page_numbers(lines, &geometry, &mut suppressed);
    // Explicit pagination markers (MORE/CONTINUED bottom markers and continued headers).
    detect_explicit_pagination_markers(lines, &geometry, &mut suppressed);

    let mut pages: std::collections::BTreeMap<usize, Vec<PhysicalTextLine>> = std::collections::BTreeMap::new();
    for &p in represented { pages.insert(p, Vec::new()); }
    for (i, l) in lines.iter().enumerate() {
        if suppressed[i] { continue; }
        pages.entry(l.page_index).or_default().push(l.clone());
    }
    let pages = pages.into_iter().map(|(pi, ls)| NormalizedTextPage { page_index: pi, lines: ls }).collect();
    NormalizedText { pages }
}

fn detect_source_page_numbers(lines: &[PhysicalTextLine], geom: &crate::page_extents::PageExtentMeasurements, suppressed: &mut [bool]) {
    let mut candidates: Vec<(usize, usize, usize)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.text.trim();
        if t.ends_with('.') && t[..t.len()-1].chars().all(|c| c.is_ascii_digit()) && t.len() >= 2 {
            let val: usize = t[..t.len()-1].parse().unwrap_or(0);
            // top band
            if !is_line_in_top_page_band(l, geom) { continue; }
            let mid = match page_horizontal_midpoint(geom, l.page_index) { Some(m) => m, None => continue };
            if l.x < mid { continue; }
            candidates.push((i, l.page_index, val));
        }
    }
    for (i, &(idx, _page, _val)) in candidates.iter().enumerate() {
        let prev = if i > 0 { Some(candidates[i-1]) } else { None };
        let next = if i + 1 < candidates.len() { Some(candidates[i+1]) } else { None };
        let seq_prev = prev.map(|(pidx, ppage, pval)| (ppage + 1 == candidates[i].1 && pval + 1 == candidates[i].2)).unwrap_or(false);
        let seq_next = next.map(|(nidx, npage, nval)| (candidates[i].1 + 1 == npage && candidates[i].2 + 1 == nval)).unwrap_or(false);
        if seq_prev || seq_next {
            suppressed[idx] = true;
        }
    }
}

fn detect_explicit_pagination_markers(lines: &[PhysicalTextLine], geom: &crate::page_extents::PageExtentMeasurements, suppressed: &mut [bool]) {
    let more = "MORE";
    let continued = "CONTINUED:";
    for (i, l) in lines.iter().enumerate() {
        let text = l.text.trim();
        let has_bottom = text == more || text == continued;
        // top marker: contains "CONTINUED:"? TS pattern continuedHeaderPattern not exact; use contains "CONTINUED"
        let has_top = text.contains("CONTINUED");
        if has_bottom {
            if is_line_in_bottom_page_band(l, geom) { suppressed[i] = true; }
        }
        if has_top {
            if is_line_in_top_page_band(l, geom) { suppressed[i] = true; }
        }
    }
}

// need page_horizontal_midpoint accessible
use crate::page_extents::page_horizontal_midpoint;