use crate::model::PhysicalTextLine;
use crate::page_extents::{is_line_in_bottom_page_band, is_line_in_top_page_band, measure_page_extents};
use crate::screenplay_text::{is_cue_parenthetical_fragment, is_cue_uppercase_text, is_parenthetical, is_parenthetical_fragment};
use crate::sequence_geometry::{is_vertically_adjacent, share_baseline};

#[derive(Debug, Clone)]
pub struct DialogueContentOccurrence { pub kind: &'static str, pub lines: Vec<PhysicalTextLine> }

#[derive(Debug, Clone)]
pub struct DialogueOccurrence {
    pub character_cue: PhysicalTextLine,
    pub content: Vec<DialogueContentOccurrence>,
    pub dialogue_lines: Vec<PhysicalTextLine>,
    pub parentheticals: Vec<PhysicalTextLine>,
}

#[derive(Debug, Clone)]
pub struct DualDialogueOccurrence {
    pub left_character_cue: PhysicalTextLine,
    pub left_dialogue: PhysicalTextLine,
    pub right_character_cue: PhysicalTextLine,
    pub right_dialogue: PhysicalTextLine,
    pub left_content: Vec<DialogueContentOccurrence>,
    pub right_content: Vec<DialogueContentOccurrence>,
    pub parentheticals: Vec<PhysicalTextLine>,
    pub lines: Vec<PhysicalTextLine>,
}

fn is_reserved(reserved: &[PhysicalTextLine], line: &PhysicalTextLine) -> bool {
    reserved.iter().any(|r| std::ptr::eq(r, line))
}

struct BaselineRow { lines: Vec<PhysicalTextLine>, next_index: usize }

fn read_baseline_row(lines: &[PhysicalTextLine], start: usize, tol: f64) -> BaselineRow {
    let first = lines[start].clone();
    let mut row = vec![first.clone()];
    let mut next = start + 1;
    while next < lines.len() && lines[next].page_index == first.page_index && share_baseline(&first, &lines[next], tol) {
        row.push(lines[next].clone());
        next += 1;
    }
    BaselineRow { lines: row, next_index: next }
}

fn group_dialogue_content(lines: &[PhysicalTextLine]) -> Vec<DialogueContentOccurrence> {
    let mut groups: Vec<DialogueContentOccurrence> = Vec::new();
    let mut parenthetical_open = false;
    for line in lines {
        let typ = if parenthetical_open || line.text.trim_start().starts_with('(') { "parenthetical" } else { "dialogue" };
        if let Some(g) = groups.last_mut() {
            if g.kind == typ { g.lines.push(line.clone()); }
            else { groups.push(DialogueContentOccurrence { kind: typ, lines: vec![line.clone()] }); }
        } else {
            groups.push(DialogueContentOccurrence { kind: typ, lines: vec![line.clone()] });
        }
        parenthetical_open = typ == "parenthetical" && !line.text.trim_end().ends_with(')');
    }
    groups
}

pub fn find_dual_dialogue_occurrences(lines: &[PhysicalTextLine], tol: f64) -> Vec<DualDialogueOccurrence> {
    let mut occurrences = Vec::new();
    let mut index = 0usize;
    while index + 1 < lines.len() {
        let lc = &lines[index];
        let rc = &lines[index + 1];
        if !is_cue_uppercase_text(&lc.text) || !is_cue_uppercase_text(&rc.text)
            || is_cue_parenthetical_fragment(&lc.text) || is_cue_parenthetical_fragment(&rc.text)
            || lc.page_index != rc.page_index || !share_baseline(lc, rc, tol) || lc.x >= rc.x {
            index += 1; continue;
        }
        let lane_boundary = (lc.x + rc.x) / 2.0;
        let mut occurrence_lines = vec![lc.clone(), rc.clone()];
        let mut parentheticals: Vec<PhysicalTextLine> = Vec::new();
        let mut left_dialogue: Option<PhysicalTextLine> = None;
        let mut right_dialogue: Option<PhysicalTextLine> = None;
        let mut previous_baseline = lc.clone();
        let mut next_index = index + 2;

        while next_index < lines.len() {
            let row = read_baseline_row(lines, next_index, tol);
            if row.lines.is_empty() || row.lines[0].page_index != lc.page_index || !is_vertically_adjacent(&previous_baseline, &row.lines[0], 1.5) {
                break;
            }
            occurrence_lines.extend(row.lines.clone());
            for line in &row.lines {
                if is_parenthetical_fragment(&line.text) {
                    if is_parenthetical(&line.text) { parentheticals.push(line.clone()); }
                    continue;
                }
                if line.x < lane_boundary && left_dialogue.is_none() { left_dialogue = Some(line.clone()); }
                if line.x > lane_boundary && right_dialogue.is_none() { right_dialogue = Some(line.clone()); }
            }
            previous_baseline = row.lines[0].clone();
            next_index = row.next_index;
            if left_dialogue.is_some() && right_dialogue.is_some() { break; }
        }

        let ld = match left_dialogue { Some(v) => v, None => { index += 1; continue; } };
        let rd = match right_dialogue { Some(v) => v, None => { index += 1; continue; } };
        if ld.x >= lc.x || rd.x <= lc.x || rd.x >= rc.x { index += 1; continue; }

        while next_index < lines.len() {
            let row = read_baseline_row(lines, next_index, tol);
            if row.lines.is_empty()
                || row.lines[0].page_index != lc.page_index
                || !is_vertically_adjacent(&previous_baseline, &row.lines[0], 1.5)
                || !row.lines.iter().all(|line| {
                    if is_parenthetical_fragment(&line.text) { return true; }
                    let expected = if line.x < lane_boundary { ld.x } else { rd.x };
                    (line.x - expected).abs() <= tol
                }) {
                break;
            }
            occurrence_lines.extend(row.lines.clone());
            for line in &row.lines { if is_parenthetical(&line.text) { parentheticals.push(line.clone()); } }
            previous_baseline = row.lines[0].clone();
            next_index = row.next_index;
        }

        let left_content = group_dialogue_content(&occurrence_lines[2..].iter().filter(|l| l.x < lane_boundary).cloned().collect::<Vec<_>>());
        let right_content = group_dialogue_content(&occurrence_lines[2..].iter().filter(|l| l.x > lane_boundary).cloned().collect::<Vec<_>>());

        occurrences.push(DualDialogueOccurrence {
            left_character_cue: lc.clone(),
            left_dialogue: ld,
            right_character_cue: rc.clone(),
            right_dialogue: rd,
            left_content,
            right_content,
            parentheticals,
            lines: occurrence_lines,
        });
        index = next_index.saturating_sub(1);
    }
    occurrences
}

pub fn find_dialogue_occurrences(lines: &[PhysicalTextLine], reserved: &[PhysicalTextLine], coordinate_tolerance: f64) -> Vec<DialogueOccurrence> {
    let mut occurrences = Vec::new();
    let extents = measure_page_extents(lines);
    let mut index = 0usize;
    while index + 1 < lines.len() {
        let cue = &lines[index];
        if is_reserved(reserved, cue) || is_cue_parenthetical_fragment(&cue.text) { index += 1; continue; }
        let mut dialogue_index = index + 1;
        let mut content_lines: Vec<PhysicalTextLine> = Vec::new();
        let mut previous_content = cue.clone();
        let mut parenthetical_open = false;

        while dialogue_index < lines.len() {
            let cand = &lines[dialogue_index];
            let continues = parenthetical_open || cand.text.trim_start().starts_with('(');
            if !continues || is_reserved(reserved, cand) || cand.page_index != cue.page_index
                || !is_vertically_adjacent(&previous_content, cand, 1.5) || cand.x >= cue.x {
                break;
            }
            content_lines.push(cand.clone());
            parenthetical_open = !cand.text.trim_end().ends_with(')');
            previous_content = cand.clone();
            dialogue_index += 1;
        }
        if parenthetical_open { index += 1; continue; }

        let first_dialogue = match lines.get(dialogue_index) { Some(v) => v, None => { index += 1; continue; } };
        let preceding = &lines[dialogue_index - 1];

        if is_reserved(reserved, first_dialogue)
            || first_dialogue.page_index != cue.page_index
            || !is_vertically_adjacent(preceding, first_dialogue, 1.5)
            || cue.x - first_dialogue.x <= cue.height
            || (!is_cue_uppercase_text(&cue.text) && cue.width >= first_dialogue.width) {
            index += 1; continue;
        }

        let mut dialogue_lines = vec![first_dialogue.clone()];
        content_lines.push(first_dialogue.clone());
        let mut previous_line = first_dialogue.clone();
        let mut continuation_index = dialogue_index + 1;
        let mut paren_open = false;

        while continuation_index < lines.len() {
            let cont = &lines[continuation_index];
            if is_reserved(reserved, cont) { break; }
            let continues_paren = paren_open || cont.text.trim_start().starts_with('(');
            if continues_paren
                && cont.page_index == previous_line.page_index
                && is_vertically_adjacent(&previous_line, cont, 1.5)
                && cont.x > first_dialogue.x
                && cont.x < cue.x {
                content_lines.push(cont.clone());
                paren_open = !cont.text.trim_end().ends_with(')');
                previous_line = cont.clone();
                continuation_index += 1;
                continue;
            }
            let continues_across_page = cont.page_index == previous_line.page_index + 1
                && is_line_in_bottom_page_band(&previous_line, &extents)
                && is_line_in_top_page_band(cont, &extents);
            let continues_on_page = cont.page_index == previous_line.page_index
                && is_vertically_adjacent(&previous_line, cont, 1.5);
            if (cont.x - first_dialogue.x).abs() > coordinate_tolerance || (!continues_across_page && !continues_on_page) {
                break;
            }
            dialogue_lines.push(cont.clone());
            content_lines.push(cont.clone());
            previous_line = cont.clone();
            continuation_index += 1;
        }

        let content = group_dialogue_content(&content_lines);
        let parentheticals: Vec<PhysicalTextLine> = content.iter().filter(|c| c.kind == "parenthetical").flat_map(|c| c.lines.clone()).collect();
        occurrences.push(DialogueOccurrence {
            character_cue: cue.clone(),
            content,
            dialogue_lines,
            parentheticals,
        });
        index = continuation_index.saturating_sub(1);
    }
    occurrences
}