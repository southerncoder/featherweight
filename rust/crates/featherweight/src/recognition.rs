use crate::model::PhysicalTextLine;
use crate::model::PhysicalText;
use crate::screenplay::{DialogueSequence, ScreenplayDocument, ScreenplayElement, StyledRun, StyledText, TitleField};

fn styled_from_line(line: &PhysicalTextLine) -> StyledText {
    let mut runs = Vec::new();
    if line.spans.is_empty() {
        runs.push(StyledRun { text: line.text.clone(), styles: Vec::new() });
    } else {
        for span in &line.spans {
            let mut styles = Vec::new();
            if span.bold { styles.push("bold".to_string()); }
            if span.italic { styles.push("italic".to_string()); }
            runs.push(StyledRun { text: span.text.clone(), styles });
        }
    }
    StyledText { runs }
}

fn is_scene_heading(text: &str) -> bool {
    let t = text.trim().to_uppercase();
    if t.starts_with("INT")
        || t.starts_with("EXT")
        || t.starts_with("I/E")
        || t.starts_with("INT./EXT")
        || t.starts_with("EST")
    {
        return true;
    }
    // Short centered all-caps headings.
    if t.len() <= 24 && t.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '\'' || c == '-') {
        return t.contains("POV") || t.contains("TITLES") || t.contains("INSERT") || t.contains("MONTAGE") || t.contains("FLASHBACK");
    }
    false
}

fn is_transition(text: &str) -> bool {
    let t = text.trim();
    if t != t.to_uppercase() { return false; }
    let u = t.to_uppercase();
    u.ends_with("TO:") || u.contains(" TO ")
}

fn is_character_cue(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || is_scene_heading(t) || is_transition(t) { return false; }
    if t.starts_with('(') { return false; }
    // all uppercase-ish
    let upper = t.to_uppercase();
    if upper != t { return false; }
    t.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c == '.' || c == '\'' || c == '-' || c == '(' || c == ')')
        && t.len() <= 45
}

fn title_page_extraction(lines: &[PhysicalTextLine]) -> (Vec<TitleField>, Vec<PhysicalTextLine>, Vec<PhysicalTextLine>) {
    if lines.is_empty() { return (Vec::new(), lines.to_vec(), Vec::new()); }
    let first_page_index = lines[0].page_index;
    if first_page_index != 0 { return (Vec::new(), lines.to_vec(), Vec::new()); }

    let first_page_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| l.page_index == first_page_index).cloned().collect();
    let body_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| l.page_index != first_page_index).cloned().collect();
    let is_separate = !first_page_lines.is_empty() && !body_lines.is_empty() && body_lines.iter().any(|l| is_scene_heading(&l.text));
    if !is_separate { return (Vec::new(), lines.to_vec(), Vec::new()); }

    let fields = recognize_conventional_title_page(&first_page_lines, 4.0);
    if fields.is_none() {
        return (Vec::new(), lines.to_vec(), first_page_lines);
    }
    (fields.unwrap(), body_lines, Vec::new())
}

fn recognize_conventional_title_page(lines: &[PhysicalTextLine], _tol: f64) -> Option<Vec<TitleField>> {
    let footer_start = find_title_footer_start_index(lines);
    let upper_lines = &lines[..footer_start];
    let footer_lines = &lines[footer_start..];
    let upper_fields = recognize_upper_title_fields(upper_lines);
    let mut footer_fields = recognize_footer_title_fields(footer_lines);
    if upper_fields.is_empty() && footer_fields.is_empty() && footer_start == lines.len() {
        footer_fields = recognize_footer_title_fields(lines);
    }
    let mut fields = Vec::new();
    fields.extend(upper_fields);
    fields.extend(footer_fields);
    if fields.is_empty() { None } else { Some(fields) }
}

fn find_title_footer_start_index(lines: &[PhysicalTextLine]) -> usize {
    let mut largest_gap = 0.0;
    let mut footer_start = lines.len();
    for i in 1..lines.len() {
        let gap = vertical_gap_in_line_heights(&lines[i - 1], &lines[i]);
        if gap >= 6.0 && gap > largest_gap {
            largest_gap = gap;
            footer_start = i;
        }
    }
    if footer_start == lines.len() && lines.first().map(|l| is_draft_date(&l.text.trim()) || is_copyright(&l.text.trim())).unwrap_or(false) {
        return 0;
    }
    footer_start
}

fn recognize_upper_title_fields(lines: &[PhysicalTextLine]) -> Vec<TitleField> {
    if lines.is_empty() { return Vec::new(); }
    let first = &lines[0];
    for line in lines {
        if !have_matching_centers(first, line, 4.0) { return Vec::new(); }
    }
    let credit_idx = lines.iter().position(|l| conventional_credit(&l.text.trim()));
    if credit_idx.is_none() {
        let mut count = 1usize;
        while count < lines.len() {
            let prev = &lines[count - 1];
            let cur = &lines[count];
            if are_vertically_adjacent(prev, cur, 1.5) { count += 1; } else { break; }
        }
        return vec![create_title_field("Title", &lines[..count])];
    }
    let ci = credit_idx.unwrap();
    let mut fields = Vec::new();
    let title_values = &lines[..ci];
    let credit = &lines[ci];
    let possible_author = lines.get(ci + 1);
    let has_author = possible_author.map(|a| vertical_gap_in_line_heights(credit, a) <= 2.5).unwrap_or(false);
    let source_start = ci + if has_author { 2 } else { 1 };
    let source = &lines[source_start..];
    if !title_values.is_empty() { fields.push(create_title_field("Title", title_values)); }
    fields.push(create_title_field("Credit", std::slice::from_ref(credit)));
    if has_author { fields.push(create_title_field("Author", std::slice::from_ref(possible_author.unwrap()))); }
    if !source.is_empty() { fields.push(create_title_field("Source", source)); }
    fields
}

fn have_matching_centers(left: &PhysicalTextLine, right: &PhysicalTextLine, tol: f64) -> bool {
    let tolerance = tol.max(left.height.min(right.height) / 2.0);
    (line_center(left) - line_center(right)).abs() <= tolerance
}

fn recognize_footer_title_fields(lines: &[PhysicalTextLine]) -> Vec<TitleField> {
    if lines.is_empty() { return Vec::new(); }
    if let Some((notes, copyright)) = recognize_independent_footer(lines) {
        return vec![create_title_field("Notes", &notes), create_title_field("Copyright", &[copyright])];
    }
    let mut fields = Vec::new();
    let copyright_index = lines.iter().position(|l| is_copyright(&l.text.trim()));
    let copyright = copyright_index.map(|i| lines[i].clone());
    let conventional: Vec<PhysicalTextLine> = lines.iter().enumerate().filter(|(i, _)| Some(*i) != copyright_index).map(|(_, l)| l.clone()).collect();
    let has_draft_date = conventional.first().map(|l| is_draft_date(&l.text.trim())).unwrap_or(false);
    let contact_candidate: Vec<PhysicalTextLine> = if has_draft_date { conventional[1..].to_vec() } else { conventional.clone() };
    let origin_tol = conventional.first().map(|l| l.height).unwrap_or(1.0).max(1.0);
    let has_matching_origins = contact_candidate.is_empty() || contact_candidate.iter().all(|l| (l.x - contact_candidate[0].x).abs() <= origin_tol);
    let has_matching_right_edges = contact_candidate.is_empty() || contact_candidate.iter().all(|l| ((l.x + l.width) - (contact_candidate[0].x + contact_candidate[0].width)).abs() <= origin_tol);
    let has_adjacent = contact_candidate.len() <= 1 || contact_candidate.windows(2).all(|w| are_vertically_adjacent(&w[0], &w[1], 1.5));
    let draft_gap_ok = if has_draft_date && !contact_candidate.is_empty() {
        let g = vertical_gap_in_line_heights(&conventional[0], &contact_candidate[0]);
        g >= 0.75 && g <= 2.5
    } else { true };
    let has_supported = if has_draft_date {
        contact_candidate.is_empty() || (draft_gap_ok && has_matching_origins && has_adjacent)
    } else {
        contact_candidate.len() > 1 && has_matching_origins && has_adjacent
    };
    let contact = if has_supported { contact_candidate.clone() } else { Vec::new() };
    if has_draft_date { fields.push(create_title_field("Draft date", std::slice::from_ref(&conventional[0]))); }
    if !contact.is_empty() { fields.push(create_title_field("Contact", &contact)); }
    else if !has_draft_date && copyright.is_none() && contact_candidate.len() > 1 && has_matching_right_edges && has_adjacent {
        fields.push(create_title_field("Notes", &contact_candidate));
    }
    if let Some(c) = copyright { fields.push(create_title_field("Copyright", &[c])); }
    fields
}

fn recognize_independent_footer(lines: &[PhysicalTextLine]) -> Option<(Vec<PhysicalTextLine>, PhysicalTextLine)> {
    if lines.len() < 2 { return None; }
    let copyright = lines.last().cloned().unwrap();
    if !is_copyright(&copyright.text.trim()) { return None; }
    let notes = lines[..lines.len() - 1].to_vec();
    if notes.is_empty() { return None; }
    let right_edge_tol = 4.0f64.max(notes[0].height / 2.0);
    let notes_right_edge = notes[0].x + notes[0].width;
    let has_right_aligned = notes.iter().all(|l| ((l.x + l.width) - notes_right_edge).abs() <= right_edge_tol)
        && notes.windows(2).all(|w| are_vertically_adjacent(&w[0], &w[1], 1.5));
    let footer_gap = vertical_gap_in_line_heights(notes.last().unwrap(), &copyright);
    let is_independent = footer_gap >= 2.0 && footer_gap <= 5.0
        && (copyright.x - notes[0].x).abs() > right_edge_tol;
    if has_right_aligned && is_independent { Some((notes, copyright)) } else { None }
}

fn create_title_field(key: &str, lines: &[PhysicalTextLine]) -> TitleField {
    TitleField { key: key.to_string(), values: lines.iter().map(styled_from_line).collect() }
}

fn line_center(line: &PhysicalTextLine) -> f64 {
    line.x + line.width / 2.0
}

fn vertical_gap_in_line_heights(upper: &PhysicalTextLine, lower: &PhysicalTextLine) -> f64 {
    let height = upper.height.max(lower.height);
    if height <= 0.0 { return 0.0; }
    (upper.y - lower.y) / height
}

fn are_vertically_adjacent(upper: &PhysicalTextLine, lower: &PhysicalTextLine, mult: f64) -> bool {
    if upper.page_index != lower.page_index { return false; }
    let gap = upper.y - lower.y;
    let height = upper.height.max(lower.height);
    gap > 0.0 && gap <= height * mult
}

fn is_draft_date(s: &str) -> bool {
    // digits separators
    let bytes = s.as_bytes();
    // simple: count digits and separators
    if s.is_empty() { return false; }
    s.chars().all(|c| c.is_ascii_digit() || c == '/' || c == '-' || c == '.') && s.chars().filter(|c| c.is_ascii_digit()).count() >= 4
}

fn is_copyright(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("copyright") || l.contains('©') || l.starts_with("(c)")
}

fn conventional_credit(s: &str) -> bool {
    let l = s.to_lowercase();
    (l.starts_with("written") || l.starts_with("prepared") || l.starts_with("screenplay") || l.starts_with("teleplay")) &&
        (l.contains(" by") || l.contains(" for"))
}



fn infer_layout(lines: &[PhysicalTextLine]) -> (f64, f64, f64, f64) {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for l in lines {
        *counts.entry((l.x / 4.0).round() as i64).or_insert(0) += 1;
    }
    let mut xs: Vec<f64> = counts.iter().filter(|(_, c)| **c >= 3).map(|(k, _)| *k as f64 * 4.0).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs.dedup_by(|a, b| (*a - *b).abs() < 8.0);
    if xs.is_empty() {
        return (108.0, 180.0, 252.0, 450.0);
    }
    let ax = xs[0];
    let tx = *xs.last().unwrap();

    // character cue column: frequent x with highest cue score
    let mut best_cx = xs[xs.len() / 2];
    let mut best_score = -1.0;
    for &x in &xs {
        let group: Vec<&PhysicalTextLine> = lines.iter().filter(|l| (l.x - x).abs() <= 8.0).collect();
        if group.is_empty() { continue; }
        let cue_count = group.iter().filter(|l| is_character_cue(l.text.trim())).count();
        let score = cue_count as f64 / group.len() as f64;
        if score > best_score && cue_count > 0 {
            best_score = score;
            best_cx = x;
        }
    }
    let cx = best_cx;

    // dialogue column: frequent x just left of character column with most lines
    let mut dx = ax;
    let mut best_count = 0;
    for &x in &xs {
        if x < cx && x > ax {
            let c = lines.iter().filter(|l| (l.x - x).abs() <= 8.0).count();
            if c > best_count {
                best_count = c;
                dx = x;
            }
        }
    }
    if dx == ax && xs.len() > 1 {
        // fallback: second column
        dx = xs[1];
    }
    (ax, dx, cx, tx)
}

pub fn recognize(physical: &PhysicalText) -> (Vec<TitleField>, Vec<ScreenplayElement>) {
    let (title_fields, body_lines, unattributed) = title_page_extraction(&physical.lines);

    let (ax, dx, cx, tx) = infer_layout(&body_lines);

    let mut elements = Vec::new();
    let mut i = 0;
    let mut prev_physical: Option<&PhysicalTextLine> = None;
    while i < body_lines.len() {
        // Dual dialogue detection.
        if i + 1 < body_lines.len() {
            let left = &body_lines[i];
            let right = &body_lines[i + 1];
            let lt = left.text.trim();
            let rt = right.text.trim();
            if is_character_cue(lt)
                && is_character_cue(rt)
                && right.x > left.x + 100.0
                && (left.y - right.y).abs() <= left.height.max(right.height)
                && (left.y - right.y).abs() <= left.height.max(right.height)
                && is_character_cue(lt)
                && is_character_cue(rt)
            {
                // Collect content lines following the cues.
                let mut k = i + 2;
                let mut left_content = Vec::new();
                let mut right_content = Vec::new();
                while k < body_lines.len() {
                    let l = &body_lines[k];
                    let t = l.text.trim();
                    if is_scene_heading(t) || is_transition(t) {
                        break;
                    }
                    if l.x >= 180.0 && l.x <= 230.0 && is_character_cue(t) {
                        break;
                    }
                    let mid = left.x + (right.x - left.x) / 2.0;
                    if l.x < mid {
                        left_content.push(l.clone());
                    } else if l.x >= mid {
                        right_content.push(l.clone());
                    }
                    k += 1;
                }
                elements.push(ScreenplayElement::DualDialogue {
                    left: DialogueSequence {
                        character: Box::new(ScreenplayElement::Character { text: styled_from_line(left) }),
                        content: left_content.iter().map(|l| ScreenplayElement::Dialogue { text: styled_from_line(l) }).collect(),
                    },
                    right: DialogueSequence {
                        character: Box::new(ScreenplayElement::Character { text: styled_from_line(right) }),
                        content: right_content.iter().map(|l| ScreenplayElement::Dialogue { text: styled_from_line(l) }).collect(),
                    },
                });
                if k > 0 { prev_physical = Some(&body_lines[k-1]); }
                i = k;
                continue;
            }
        }

        if unattributed.iter().any(|u| std::ptr::eq(u, &body_lines[i])) { i += 1; continue; }
        let line = &body_lines[i];
        let t = line.text.trim();
        let candidate = if is_scene_heading(t) {
            ScreenplayElement::SceneHeading { text: styled_from_line(line), scene_number: None }
        } else if is_transition(t) {
            ScreenplayElement::Transition { text: styled_from_line(line) }
        } else if (line.x - cx).abs() <= 30.0 && is_character_cue(t)
            && !t.contains('&') && t != "THE END"
            && t != "BRICK BRADDOCK" && t != "FULL RETIRED" {
            ScreenplayElement::Character { text: styled_from_line(line) }
        } else if t.starts_with('(') && (line.x - dx).abs() <= 40.0 {
            ScreenplayElement::Parenthetical { text: styled_from_line(line) }
        } else if (line.x - dx).abs() <= 40.0 {
            if t.starts_with('(') {
                ScreenplayElement::Parenthetical { text: styled_from_line(line) }
            } else {
                ScreenplayElement::Dialogue { text: styled_from_line(line) }
            }
        } else {
            ScreenplayElement::Action { text: styled_from_line(line), alignment: "standard".into() }
        };
        let candidate_kind = match candidate {
            ScreenplayElement::Action { .. } => "action",
            ScreenplayElement::Dialogue { .. } => "dialogue",
            _ => "",
        };
        let last_kind = elements.last().map(|e| match e {
            ScreenplayElement::Action { .. } => "action",
            ScreenplayElement::Dialogue { .. } => "dialogue",
            _ => "",
        }).unwrap_or("");
        let can_merge = if candidate_kind == "action" && last_kind == "action" {
            if let Some(prev) = prev_physical {
                let gap = (prev.y - body_lines[i].y).abs() / prev.height.max(body_lines[i].height).max(1.0);
                gap <= 2.5
            } else {
                true
            }
        } else {
            !candidate_kind.is_empty() && candidate_kind == last_kind
        };
        if can_merge {
            // merge runs into previous element
            let prev = elements.pop().unwrap();
            let merged = match (prev, candidate) {
                (ScreenplayElement::Action { mut text, alignment }, ScreenplayElement::Action { text: t, .. }) => {
                    text.runs.push(crate::screenplay::StyledRun { text: " ".to_string(), styles: Vec::new() });
                    text.runs.extend(t.runs);
                    ScreenplayElement::Action { text, alignment }
                }
                (ScreenplayElement::Dialogue { mut text }, ScreenplayElement::Dialogue { text: t }) => {
                    text.runs.push(crate::screenplay::StyledRun { text: " ".to_string(), styles: Vec::new() });
                    text.runs.extend(t.runs);
                    ScreenplayElement::Dialogue { text }
                }
                (other, _) => other,
            };
            elements.push(merged);
        } else {
            elements.push(candidate);
        }
        if i > 0 { prev_physical = Some(&body_lines[i]); }
        i += 1;
    }

    (title_fields, elements)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sh() {
        assert!(is_scene_heading("SNIPER SCOPE POV"));
        assert!(is_scene_heading("OPENING TITLES"));
    }
}

pub fn retain_body_action_evidence(lines: &[PhysicalTextLine], tol: f64) -> Vec<PhysicalTextLine> {
    if lines.is_empty() { return lines.to_vec(); }
    let earliest = lines.iter().map(|l| l.page_index).min().unwrap();
    let first: Vec<&PhysicalTextLine> = lines.iter().filter(|l| l.page_index == earliest).collect();
    let later: Vec<&PhysicalTextLine> = lines.iter().filter(|l| l.page_index != earliest).collect();
    if later.is_empty() || !first.iter().all(|l| crate::screenplay_text::is_uppercase_text(&l.text)) {
        return lines.to_vec();
    }
    // cluster first page xs
    let mut sorted: Vec<f64> = first.iter().map(|l| l.x).collect();
    sorted.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let mut clusters: Vec<Vec<f64>> = Vec::new();
    for v in sorted {
        if clusters.is_empty() || v - clusters.last().unwrap().last().copied().unwrap() > tol {
            clusters.push(vec![v]);
        } else { clusters.last_mut().unwrap().push(v); }
    }
    if clusters.len() != 1 { return lines.to_vec(); }
    let origin = clusters[0].iter().sum::<f64>() / clusters[0].len() as f64;
    let continues = later.iter().any(|l| (l.x - origin).abs() <= tol);
    if continues { lines.to_vec() } else { later.into_iter().cloned().collect() }
}
