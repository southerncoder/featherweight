use crate::model::PhysicalTextLine;
use crate::normalized_text::{NormalizedText, NormalizedTextPage};
use crate::page_extents::{
    get_effective_page_extents, is_line_in_bottom_page_band, is_line_in_observed_bottom_page_band, is_line_in_top_page_band, measure_page_extents, page_margin_band_depth,
};

pub fn normalize_physical_text(lines: &[PhysicalTextLine], represented: &[usize]) -> NormalizedText {
    let mut suppressed = vec![false; lines.len()];
    let geometry = measure_page_extents(lines);

    // Source page numbers (top band, sequential numeric suffix).
    detect_source_page_numbers(lines, &geometry, &mut suppressed);
    // Explicit pagination markers (MORE/CONTINUED bottom markers and continued headers).
    detect_explicit_pagination_markers(lines, &geometry, &mut suppressed);
    // Recurring margin artifacts (headers/footers repeated across pages).
    detect_recurring_artifacts(lines, &geometry, &mut suppressed);
    // Continued character cues spanning page breaks.
    detect_continued_character_cues(lines, &geometry, &mut suppressed);

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
#[derive(Clone)]
struct Frag { text: String, font_size: f64, x: f64, y: f64, line_index: usize }

fn detect_recurring_artifacts(lines: &[PhysicalTextLine], geom: &crate::page_extents::PageExtentMeasurements, suppressed: &mut [bool]) {
    if lines.is_empty() { return; }
    let doc_pages = lines.iter().map(|l| l.page_index).collect::<std::collections::BTreeSet<_>>().len();
    // group fragments by text
    let mut by_text: std::collections::BTreeMap<String, Vec<Frag>> = std::collections::BTreeMap::new();
    for (i, l) in lines.iter().enumerate() {
        let text = l.text.trim().to_string();
        if text.len() < 5 { continue; }
        if is_explicit_pagination_text(&text) { continue; }
        let f = Frag { text: text.clone(), font_size: l.height, x: l.x, y: l.y, line_index: i };
        by_text.entry(text).or_default().push(f);
    }
    for fragments in by_text.values() {
        // cluster by stable presentation
        let mut clusters: Vec<Vec<Frag>> = Vec::new();
        for f in fragments {
            if let Some(c) = clusters.iter_mut().find(|c| have_stable_presentation(&c[0], f)) {
                c.push(f.clone());
            } else {
                clusters.push(vec![f.clone()]);
            }
        }
        for cluster in &clusters {
            // group by page, unambiguous (one per page)
            let mut by_page: std::collections::BTreeMap<usize, Vec<&Frag>> = std::collections::BTreeMap::new();
            for f in cluster {
                by_page.entry(lines[f.line_index].page_index).or_default().push(f);
            }
            let unambiguous: Vec<&Frag> = by_page.values().filter(|v| v.len() == 1).map(|v| v[0]).collect();
            if unambiguous.len() < 3 { continue; }
            if unambiguous.len() as f64 / doc_pages as f64 <= 0.5 { continue; }
            for f in unambiguous {
                let line = &lines[f.line_index];
                if is_line_in_page_margin_band(line, geom) {
                    suppressed[f.line_index] = true;
                }
            }
        }
    }
}

fn have_stable_presentation(a: &Frag, b: &Frag) -> bool {
    let tol = a.font_size.max(b.font_size) / 2.0;
    (a.font_size - b.font_size).abs() <= 0.25
        && (a.x - b.x).abs() <= tol
        && (a.y - b.y).abs() <= tol
}

fn is_explicit_pagination_text(text: &str) -> bool {
    text == "(MORE)" || text == "(CONTINUED):" || text.contains("CONTINUED")
}

fn is_line_in_page_margin_band(line: &PhysicalTextLine, geom: &crate::page_extents::PageExtentMeasurements) -> bool {
    crate::page_extents::is_line_in_top_page_band(line, geom) || crate::page_extents::is_line_in_bottom_page_band(line, geom)
}


struct CharCue { name: String, continued: bool, line_index: usize }

fn parse_char_cue(line: &PhysicalTextLine) -> Option<CharCue> {
    let t = line.text.trim();
    let (name, continued) = if t.ends_with("(CONT'D)") || t.ends_with("(cont'd)") {
        let name = t[..t.len()-9].trim().to_string(); // remove " (CONT'D)" ~9? adjust
        (name, true)
    } else {
        // if all uppercase words
        if !t.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c=='.' || c=='-' || c=='(' || c==')' || c=='\'') { return None; }
        (t.to_string(), false)
    };
    // require at least 2 letters
    if name.chars().filter(|c| c.is_alphabetic()).count() < 2 { return None; }
    Some(CharCue { name, continued, line_index: 0 })
}

fn detect_continued_character_cues(lines: &[PhysicalTextLine], geom: &crate::page_extents::PageExtentMeasurements, suppressed: &mut [bool]) {
    // group indices by page in order
    let mut pages: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, l) in lines.iter().enumerate() { pages.entry(l.page_index).or_default().push(i); }
    let ordered: Vec<usize> = pages.keys().copied().collect();
    for offset in 1..ordered.len() {
        let page_idx = ordered[offset];
        let prev_page_idx = ordered[offset-1];
        if page_idx != prev_page_idx + 1 { continue; }
        let page_lines = &pages[&page_idx];
        let prev_page_lines = &pages[&prev_page_idx];
        // first content index on current page
        let mut first_content = None;
        for (pos, &li) in page_lines.iter().enumerate() {
            if !suppressed[li] { first_content = Some((pos, li)); break; }
        }
        let (first_pos, first_li) = match first_content { Some(v) => v, None => continue };
        let current_cue = match parse_char_cue(&lines[first_li]) { Some(mut c) => { c.line_index=first_li; c }, None => continue };
        if !is_line_in_top_page_band(&lines[first_li], geom) { continue; }
        // trailing more line in previous page
        let mut trailing_more = None;
        for li in prev_page_lines.iter().rev() {
            if lines[*li].text.trim() == "(MORE)" { trailing_more = Some(*li); break; }
            if !suppressed[*li] { break; }
        }
        // previous content line
        let search_end = trailing_more.unwrap_or(prev_page_lines.len());
        let mut previous_content_line = None;
        for li in prev_page_lines.iter().take(search_end).rev() {
            if !suppressed[*li] { previous_content_line = Some(*li); break; }
        }
        // following content on current page
        let following_content_line = page_lines[first_pos+1..].iter().copied().find(|li| !suppressed[*li]);
        // previous matching column cue
        let font_size = lines[first_li].height;
        let mut previous_cue: Option<CharCue> = None;
        for li in prev_page_lines.iter().rev() {
            if suppressed[*li] { continue; }
            if let Some(c) = parse_char_cue(&lines[*li]) {
                if (lines[*li].x - lines[first_li].x).abs() <= font_size * 2.0 {
                    previous_cue = Some(CharCue { line_index: *li, ..c });
                    break;
                }
            }
        }
        let ends_previous_page = if let Some(pc) = previous_content_line {
            if trailing_more.is_none() {
                is_line_in_observed_bottom_page_band(&lines[pc], geom)
            } else {
                lines[pc].y <= lines[trailing_more.unwrap()].y + page_margin_band_depth(geom)
            }
        } else { false };
        let has_dialogue_indent = |cue_li: usize, content_li: usize| -> bool {
            let indent = lines[cue_li].x - lines[content_li].x;
            indent >= font_size * 2.0 && indent <= font_size * 8.0
        };
        let has_context = previous_content_line.is_some() && following_content_line.is_some()
            && previous_cue.as_ref().map(|c| c.name == current_cue.name).unwrap_or(false)
            && ends_previous_page
            && has_dialogue_indent(first_li, previous_content_line.unwrap())
            && has_dialogue_indent(first_li, following_content_line.unwrap());
        if has_context {
            suppressed[first_li] = true;
            if let Some(tm) = trailing_more { suppressed[tm] = true; }
        } else if current_cue.continued {
            // diagnostic skipped
        }
    }
}
