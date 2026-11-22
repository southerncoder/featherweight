use crate::coordinate_evidence::{cluster_coordinates, median};
use crate::dialogue_occurrences::DialogueOccurrence;
use crate::model::PhysicalTextLine;
use crate::screenplay_text::is_uppercase_text;

fn is_reserved(reserved: &[PhysicalTextLine], line: &PhysicalTextLine) -> bool {
    reserved.iter().any(|r| std::ptr::eq(r, line))
}

fn is_vertically_adjacent(upper: &PhysicalTextLine, lower: &PhysicalTextLine, mult: f64) -> bool {
    if upper.page_index != lower.page_index { return false; }
    let gap = upper.y - lower.y;
    let height = upper.height.max(lower.height);
    gap > 0.0 && gap <= height * mult
}

pub fn find_contextual_transition_candidates(
    lines: &[PhysicalTextLine],
    reserved: &[PhysicalTextLine],
    dialogue_candidates: &[DialogueOccurrence],
    repeated_transition_lines: &[PhysicalTextLine],
) -> Vec<PhysicalTextLine> {
    if dialogue_candidates.len() < 2 || !repeated_transition_lines.is_empty() {
        return Vec::new();
    }
    let char_cue_x = median(&dialogue_candidates.iter().map(|o| o.character_cue.x).collect::<Vec<_>>());
    let dialogue_x = median(&dialogue_candidates.iter().flat_map(|o| o.dialogue_lines.iter().map(|l| l.x)).collect::<Vec<_>>());
    let typical_height = median(&lines.iter().map(|l| l.height).collect::<Vec<_>>());
    let cue_to_dialogue_indent = char_cue_x - dialogue_x;
    let right_side_threshold = char_cue_x + (typical_height * 2.0).max(cue_to_dialogue_indent);

    let mut line_indexes = std::collections::BTreeMap::new();
    for (i, l) in lines.iter().enumerate() { line_indexes.insert(i, l); }

    lines.iter().enumerate().filter(|(idx, line)| {
        if is_reserved(reserved, line) || !is_uppercase_text(&line.text) || line.x <= right_side_threshold {
            return false;
        }
        let next = lines.get(idx + 1);
        if let Some(next) = next {
            next.page_index == line.page_index
                && is_vertically_adjacent(line, next, 2.0)
                && next.x < line.x - line.height
        } else { false }
    }).map(|(_, l)| l.clone()).collect()
}

pub fn find_transition_lines(
    lines: &[PhysicalTextLine],
    reserved: &[PhysicalTextLine],
    repeated_transition_lines: &[PhysicalTextLine],
    dialogue_occurrences: &[DialogueOccurrence],
    coordinate_tolerance: f64,
) -> Vec<PhysicalTextLine> {
    if lines.is_empty() { return Vec::new(); }

    let standalone_opening: Vec<PhysicalTextLine> = lines.iter().enumerate().filter(|(i, line)| {
        !is_reserved(reserved, line) && line.text.trim() == "FADE IN"
            && (*i == 0 || lines[*i-1].page_index != line.page_index)
    }).map(|(_, l)| l.clone()).collect();

    let mut pages: std::collections::BTreeMap<usize, Vec<&PhysicalTextLine>> = std::collections::BTreeMap::new();
    for l in lines { pages.entry(l.page_index).or_default().push(l); }
    let page_left_edges: Vec<f64> = pages.values().map(|v| v.iter().map(|l| l.x).fold(f64::INFINITY, |a, b| a.min(b))).collect();
    let page_right_edges: Vec<f64> = pages.values().map(|v| v.iter().map(|l| l.x + l.width).fold(f64::NEG_INFINITY, |a, b| a.max(b))).collect();
    let document_left_edge = median(&page_left_edges);
    let document_right_edge = median(&page_right_edges);
    let document_midpoint = (document_left_edge + document_right_edge) / 2.0;
    let right_side_threshold = infer_right_side_threshold(lines, dialogue_occurrences, document_midpoint);

    let geometric_candidates: Vec<PhysicalTextLine> = lines.iter().filter(|line| {
        let right_edge = line.x + line.width;
        !is_reserved(reserved, line)
            && is_uppercase_text(&line.text)
            && line.x > right_side_threshold
            && (right_edge - document_right_edge).abs() <= line.height
    }).cloned().collect();

    let right_edge_clusters = cluster_coordinates(&geometric_candidates.iter().map(|l| l.x + l.width).collect::<Vec<_>>(), coordinate_tolerance);
    let mut line_indexes = std::collections::BTreeMap::new();
    for (i, l) in lines.iter().enumerate() { line_indexes.insert(i, l); }

    let contextual: Vec<PhysicalTextLine> = geometric_candidates.iter().filter(|line| {
        let idx = lines.iter().position(|l| std::ptr::eq(l, *line)).unwrap_or(0);
        let next = lines.get(idx + 1);
        let has_leftward = if let Some(n) = next {
            n.page_index == line.page_index && is_vertically_adjacent(line, n, 3.0) && n.x < line.x - line.height
        } else { false };
        let right_edge = line.x + line.width;
        let has_repeated = right_edge_clusters.iter().any(|cluster| {
            cluster.values.len() > 1 && cluster.values.iter().any(|c| (c - right_edge).abs() <= coordinate_tolerance)
        });
        has_leftward || has_repeated
    }).cloned().collect();

    let mut all = Vec::new();
    all.extend(repeated_transition_lines.iter().cloned());
    all.extend(contextual.iter().cloned());
    all.extend(standalone_opening.iter().cloned());
    // dedup by pointer? keep unique by x/y/text
    all.sort_by(|a,b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal).then(a.x.partial_cmp(&b.x).unwrap()));
    all.dedup_by(|a,b| (a.x-b.x).abs()<0.5 && (a.y-b.y).abs()<0.5 && a.text==b.text);
    all
}

pub fn find_repeated_transition_lines(
    lines: &[PhysicalTextLine],
    reserved: &[PhysicalTextLine],
    coordinate_tolerance: f64,
) -> Vec<PhysicalTextLine> {
    if lines.is_empty() { return Vec::new(); }
    let typical_left = median(&lines.iter().map(|l| l.x).collect::<Vec<_>>());
    let typical_right = median(&lines.iter().map(|l| l.x + l.width).collect::<Vec<_>>());
    let content_midpoint = (typical_left + typical_right) / 2.0;
    let candidates: Vec<PhysicalTextLine> = lines.iter().filter(|line| {
        !is_reserved(reserved, line) && is_uppercase_text(&line.text) && line.x > content_midpoint
    }).cloned().collect();
    filter_repeated_right_edge_lines(&candidates, coordinate_tolerance)
}

fn filter_repeated_right_edge_lines(candidates: &[PhysicalTextLine], coordinate_tolerance: f64) -> Vec<PhysicalTextLine> {
    if candidates.is_empty() { return Vec::new(); }
    let right_edges: Vec<f64> = candidates.iter().map(|l| l.x + l.width).collect();
    let clusters = cluster_coordinates(&right_edges, coordinate_tolerance);
    candidates.iter().filter(|line| {
        let right_edge = line.x + line.width;
        let matching = clusters.iter().find(|c| c.values.iter().any(|v| (v - right_edge).abs() <= coordinate_tolerance));
        if matching.is_none() || matching.unwrap().values.len() < 2 { return false; }
        let matching_candidates: Vec<&PhysicalTextLine> = candidates.iter().filter(|cand| {
            matching.unwrap().values.iter().any(|v| (v - (cand.x + cand.width)).abs() <= coordinate_tolerance)
        }).collect();
        let x_clusters = cluster_coordinates(&matching_candidates.iter().map(|c| c.x).collect::<Vec<_>>(), coordinate_tolerance);
        x_clusters.len() > 1
    }).cloned().collect()
}

pub fn resolve_repeated_transition_lines(
    lines: &[PhysicalTextLine],
    transition_candidates: &[PhysicalTextLine],
    dialogue_candidates: &[DialogueOccurrence],
    coordinate_tolerance: f64,
) -> Vec<PhysicalTextLine> {
    let candidate_set: Vec<&PhysicalTextLine> = transition_candidates.iter().collect();
    let unambiguous: Vec<&DialogueOccurrence> = dialogue_candidates.iter().filter(|occ| {
        !candidate_set.iter().any(|c| std::ptr::eq(*c, &occ.character_cue))
    }).collect();
    if unambiguous.is_empty() {
        return transition_candidates.to_vec();
    }
    let char_cue_x = median(&unambiguous.iter().map(|o| o.character_cue.x).collect::<Vec<_>>());
    let typical_height = median(&lines.iter().map(|l| l.height).collect::<Vec<_>>());
    let threshold = char_cue_x + typical_height;
    let filtered: Vec<PhysicalTextLine> = transition_candidates.iter().filter(|l| l.x > threshold).cloned().collect();
    filter_repeated_right_edge_lines(&filtered, coordinate_tolerance)
}

fn infer_right_side_threshold(lines: &[PhysicalTextLine], dialogue_occurrences: &[DialogueOccurrence], content_midpoint: f64) -> f64 {
    if dialogue_occurrences.is_empty() { return content_midpoint; }
    let char_cue_x = median(&dialogue_occurrences.iter().map(|o| o.character_cue.x).collect::<Vec<_>>());
    let typical_height = median(&lines.iter().map(|l| l.height).collect::<Vec<_>>());
    content_midpoint.max(char_cue_x + typical_height)
}