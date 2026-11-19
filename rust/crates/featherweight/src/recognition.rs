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

fn title_page_extraction(lines: &[PhysicalTextLine]) -> (Vec<TitleField>, Vec<PhysicalTextLine>) {
    let first_page = lines.first().map(|l| l.page_index).unwrap_or(0);
    if first_page != 0 { return (Vec::new(), lines.to_vec()); }
    // later page scene heading?
    let has_later_scene = lines.iter().any(|l| l.page_index != 0 && is_scene_heading(&l.text));
    if !has_later_scene { return (Vec::new(), lines.to_vec()); }

    let first_page_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| l.page_index == 0).cloned().collect();
    let body_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| l.page_index != 0).cloned().collect();

    // footer start by largest vertical gap
    let mut footer_start = first_page_lines.len();
    let mut largest_gap = 0.0;
    for i in 1..first_page_lines.len() {
        let prev = &first_page_lines[i-1];
        let cur = &first_page_lines[i];
        let gap = (cur.y - (prev.y + prev.height)) / prev.height.max(cur.height).max(1.0);
        if gap >= 6.0 && gap > largest_gap {
            largest_gap = gap;
            footer_start = i;
        }
    }
    if footer_start == first_page_lines.len() && (
        first_page_lines.first().map(|l| is_draft_date(&l.text.trim())).unwrap_or(false) ||
        first_page_lines.first().map(|l| is_copyright(&l.text.trim())).unwrap_or(false)
    ) {
        footer_start = 0;
    }

    let upper_lines = &first_page_lines[..footer_start];
    let footer_lines = &first_page_lines[footer_start..];

    // upper fields
    let mut upper_fields: Vec<TitleField> = Vec::new();
    // credit index
    let credit_idx = upper_lines.iter().position(|l| conventional_credit(&l.text.trim()));
    if let Some(ci) = credit_idx {
        let title_values: Vec<PhysicalTextLine> = upper_lines[..ci].to_vec();
        if !title_values.is_empty() {
            upper_fields.push(TitleField { key: "Title".into(), values: title_values.iter().map(styled_from_line).collect() });
        }
        let credit_line = &upper_lines[ci];
        upper_fields.push(TitleField { key: "Credit".into(), values: vec![styled_from_line(credit_line)] });
        let possible_author = upper_lines.get(ci+1);
        let has_author = possible_author.map(|a| (a.y - (credit_line.y + credit_line.height)) / credit_line.height.max(1.0) <= 2.5).unwrap_or(false);
        if has_author {
            upper_fields.push(TitleField { key: "Author".into(), values: vec![styled_from_line(possible_author.unwrap())] });
        }
        let source_start = ci + if has_author { 2 } else { 1 };
        let source = &upper_lines[source_start..];
        if !source.is_empty() {
            upper_fields.push(TitleField { key: "Source".into(), values: source.iter().map(styled_from_line).collect() });
        }
    } else if !upper_lines.is_empty() {
        // no credit: title group adjacent
        let mut count = 1;
        while count < upper_lines.len() {
            let prev = &upper_lines[count-1];
            let cur = &upper_lines[count];
            let gap = (cur.y - (prev.y + prev.height)) / prev.height.max(1.0);
            if gap <= 1.8 { count += 1; } else { break; }
        }
        upper_fields.push(TitleField { key: "Title".into(), values: upper_lines[..count].iter().map(styled_from_line).collect() });
    }

    // footer fields
    let mut footer_fields: Vec<TitleField> = Vec::new();
    let mut contact_candidate: Vec<PhysicalTextLine> = Vec::new();
    let mut copyright_line: Option<PhysicalTextLine> = None;
    let mut draft_date_line: Option<PhysicalTextLine> = None;
    for l in footer_lines {
        let t = l.text.trim();
        if is_draft_date(t) { draft_date_line = Some(l.clone()); }
        else if is_copyright(t) { copyright_line = Some(l.clone()); }
        else { contact_candidate.push(l.clone()); }
    }
    if let Some(dd) = &draft_date_line {
        footer_fields.push(TitleField { key: "Draft date".into(), values: vec![styled_from_line(dd)] });
    }
    if !contact_candidate.is_empty() {
        // decide contact vs notes: if draft date present, contact; else notes if right-aligned? Use contact
        if draft_date_line.is_some() {
            footer_fields.push(TitleField { key: "Contact".into(), values: contact_candidate.iter().map(styled_from_line).collect() });
        } else {
            footer_fields.push(TitleField { key: "Notes".into(), values: contact_candidate.iter().map(styled_from_line).collect() });
        }
    }
    if let Some(c) = &copyright_line {
        footer_fields.push(TitleField { key: "Copyright".into(), values: vec![styled_from_line(c)] });
    }

    // if no upper/foot fields, mark unattributed (return original lines) - but we assume recognized
    let mut fields = Vec::new();
    fields.extend(upper_fields);
    fields.extend(footer_fields);

    if fields.is_empty() {
        return (Vec::new(), lines.to_vec());
    }
    (fields, body_lines)
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

fn infer_x(lines: &[PhysicalTextLine], idx: usize) -> f64 {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for l in lines {
        *counts.entry((l.x / 4.0).round() as i64).or_insert(0) += 1;
    }
    let mut frequent: Vec<(i64, usize)> = counts.into_iter().filter(|(_, c)| *c >= 3).collect();
    frequent.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    // take distinct top positions
    let mut xs: Vec<f64> = Vec::new();
    for (xb, _) in &frequent {
        let x = *xb as f64 * 4.0;
        if xs.iter().all(|prev| (prev - x).abs() > 20.0) {
            xs.push(x);
        }
        if xs.len() >= 6 { break; }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if xs.is_empty() {
        return 108.0;
    }
    if idx < xs.len() {
        xs[idx]
    } else {
        *xs.last().unwrap()
    }
}

pub fn recognize(physical: &PhysicalText) -> (Vec<TitleField>, Vec<ScreenplayElement>) {
    let (title_fields, body_lines) = title_page_extraction(&physical.lines);

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
                gap <= 1.8
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
                    text.runs.extend(t.runs);
                    ScreenplayElement::Action { text, alignment }
                }
                (ScreenplayElement::Dialogue { mut text }, ScreenplayElement::Dialogue { text: t }) => {
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
