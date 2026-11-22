use crate::model::PhysicalTextLine;
use crate::screenplay_layout::ScreenplayLayout;
use crate::screenplay_text::{is_uppercase_text};
use crate::sequence_geometry::is_vertically_adjacent;

pub const VISIBLE_LINE_SHORTFALL_MULTIPLIER: f64 = 5.0;

#[derive(Debug, Clone)]
pub struct SceneHeadingEvidence {
    pub heading: PhysicalTextLine,
    pub heading_lines: Vec<PhysicalTextLine>,
    pub number_fragments: Vec<PhysicalTextLine>,
    pub scene_number: Option<String>,
}

fn is_reserved(reserved: &[PhysicalTextLine], l: &PhysicalTextLine) -> bool {
    reserved.iter().any(|r| std::ptr::eq(r, l))
}

pub fn is_scene_heading_line(line: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    let action_x = match &layout.action { Some(a) => a.x, None => return false };
    if (line.x - action_x).abs() > tol { return false; }
    let t = line.text.trim();
    let re = t.starts_with("INT.") || t.starts_with("EXT.") || t.starts_with("INT/EXT.") || t.starts_with("I/E.") || t.starts_with("EST.");
    let bold = line.spans.iter().all(|s| s.bold) && !line.spans.is_empty();
    re || (is_uppercase_text(t) && bold)
}

pub fn is_numbered_scene_heading_line(line: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    let action_x = match &layout.action { Some(a) => a.x, None => return false };
    if (line.x - action_x).abs() > tol { return false; }
    is_uppercase_text(line.text.trim())
}

pub fn plausible_scene_number(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || !t.chars().any(|c| c.is_ascii_digit()) { return false; }
    t.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

pub fn find_scene_headings(
    lines: &[PhysicalTextLine],
    reserved: &[PhysicalTextLine],
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
    action_line_measure: Option<f64>,
) -> Vec<SceneHeadingEvidence> {
    let mut result = Vec::new();
    let mut consumed = Vec::new();
    for heading_index in 0..lines.len() {
        let heading = &lines[heading_index];
        if is_reserved(reserved, heading) || consumed.iter().any(|c| std::ptr::eq(c, heading)) { continue; }
        let is_established = is_scene_heading_line(heading, layout, coordinate_tolerance);
        let is_numbered = is_numbered_scene_heading_line(heading, layout, coordinate_tolerance);
        if !is_established && !is_numbered { continue; }
        let scene_number = find_scene_number_evidence(lines, heading, coordinate_tolerance);

        if !is_established {
            let ok = scene_number.as_ref().map(|s| plausible_scene_number(&s.number)).unwrap_or(false);
            if !ok { continue; }
        }

        let continuation = lines.get(heading_index + 1);
        let heading_lines = if let Some(cont) = continuation {
            if can_wrap_scene_heading_continuation(heading, cont, reserved, layout, coordinate_tolerance, action_line_measure) {
                consumed.push(cont.clone());
                vec![heading.clone(), cont.clone()]
            } else { vec![heading.clone()] }
        } else { vec![heading.clone()] };

        result.push(SceneHeadingEvidence {
            heading: heading.clone(),
            heading_lines,
            number_fragments: scene_number.as_ref().map(|s| s.fragments.clone()).unwrap_or_default(),
            scene_number: scene_number.as_ref().map(|s| s.number.clone()),
        });
    }
    result
}

struct SceneNumberEvidence { fragments: Vec<PhysicalTextLine>, number: String }

fn find_scene_number_evidence(lines: &[PhysicalTextLine], heading: &PhysicalTextLine, tol: f64) -> Option<SceneNumberEvidence> {
    let baseline: Vec<&PhysicalTextLine> = lines.iter().filter(|l| !std::ptr::eq(*l, heading) && l.page_index == heading.page_index && (l.y - heading.y).abs() <= tol).collect();
    let left: Vec<&PhysicalTextLine> = baseline.iter().copied().filter(|l| l.x < heading.x).collect();
    let right: Vec<&PhysicalTextLine> = baseline.iter().copied().filter(|l| l.x > heading.x + heading.width).collect();

    // paired identical
    for l in &left {
        for r in &right {
            if r.text.trim() == l.text.trim() && plausible_scene_number(l.text.trim()) {
                return Some(SceneNumberEvidence { fragments: vec![(*l).clone(), (*r).clone()], number: l.text.trim().to_string() });
            }
        }
    }
    // left nearest
    let mut left_candidates: Vec<&PhysicalTextLine> = left.into_iter().filter(|l| plausible_scene_number(l.text.trim())).collect();
    left_candidates.sort_by(|a,b| {
        let da = heading.x - (a.x + a.width);
        let db = heading.x - (b.x + b.width);
        da.partial_cmp(&db).unwrap()
    });
    if let Some(l) = left_candidates.first() {
        return Some(SceneNumberEvidence { fragments: vec![(*l).clone()], number: l.text.trim().to_string() });
    }
    // right nearest
    let mut right_candidates: Vec<&PhysicalTextLine> = right.into_iter().filter(|l| plausible_scene_number(l.text.trim())).collect();
    right_candidates.sort_by(|a,b| a.x.partial_cmp(&b.x).unwrap());
    if let Some(r) = right_candidates.first() {
        return Some(SceneNumberEvidence { fragments: vec![(*r).clone()], number: r.text.trim().to_string() });
    }
    None
}

fn can_wrap_scene_heading_continuation(
    heading: &PhysicalTextLine,
    continuation: &PhysicalTextLine,
    reserved: &[PhysicalTextLine],
    layout: &ScreenplayLayout,
    tol: f64,
    action_line_measure: Option<f64>,
) -> bool {
    if action_line_measure.is_none() { return false; }
    let am = action_line_measure.unwrap();
    if is_reserved(reserved, continuation) { return false; }
    if continuation.page_index != heading.page_index { return false; }
    if !is_vertically_adjacent(heading, continuation, 1.5) { return false; }
    let action_x = match &layout.action { Some(a) => a.x, None => return false };
    if (continuation.x - action_x).abs() > tol { return false; }
    let shortfall = heading.height * VISIBLE_LINE_SHORTFALL_MULTIPLIER;
    if heading.width + shortfall < am { return false; }
    let continuation_text = continuation.text.trim();
    let conv = continuation_text.starts_with("INT.") || continuation_text.starts_with("EXT.");
    !conv && is_uppercase_text(continuation_text)
        && !continuation.spans.is_empty()
        && continuation.spans.iter().all(|s| s.bold)
}

pub fn create_scene_heading(evidence: &SceneHeadingEvidence) -> (String, Option<String>) {
    (evidence.heading.text.trim().to_string(), evidence.scene_number.clone())
}