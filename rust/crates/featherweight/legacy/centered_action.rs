use crate::model::PhysicalTextLine;
use crate::screenplay_layout::ScreenplayLayout;
use crate::sequence_geometry::is_vertically_adjacent;

fn ptr_in(slice: &[PhysicalTextLine], l: &PhysicalTextLine) -> bool {
    slice.iter().any(|r| std::ptr::eq(r, l))
}

fn line_center(l: &PhysicalTextLine) -> f64 { l.x + l.width / 2.0 }

fn has_baseline_sibling(line: &PhysicalTextLine, lines: &[PhysicalTextLine], tol: f64) -> bool {
    lines.iter().any(|c| !std::ptr::eq(c, line) && c.page_index == line.page_index && (c.y - line.y).abs() <= tol)
}

fn matches_established_cue_geometry(line: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    let mut anchors = Vec::new();
    if let Some(a) = &layout.character_cue { anchors.push(a.x); }
    if let Some(dd) = &layout.dual_dialogue { anchors.push(dd.left_character_cue_x); anchors.push(dd.right_character_cue_x); }
    anchors.iter().any(|a| (line.x - *a).abs() <= tol)
}

fn matches_established_content_geometry(line: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    let mut anchors = Vec::new();
    if let Some(a) = &layout.dialogue { anchors.push(a.x); }
    if let Some(a) = &layout.parenthetical { anchors.push(a.x); }
    if let Some(dd) = &layout.dual_dialogue { anchors.push(dd.left_dialogue_x); anchors.push(dd.right_dialogue_x); }
    let role_tol = tol.max(line.height);
    if anchors.iter().any(|a| (line.x - *a).abs() <= role_tol) { return true; }
    if let Some(t) = &layout.transition {
        if ((line.x + line.width) - t.x).abs() <= role_tol { return true; }
    }
    false
}

fn matches_established_role_geometry(line: &PhysicalTextLine, layout: &ScreenplayLayout, tol: f64) -> bool {
    matches_established_cue_geometry(line, layout, tol.max(line.height)) || matches_established_content_geometry(line, layout, tol)
}

fn is_isolated_from_role_geometry(line: &PhysicalTextLine, lines: &[PhysicalTextLine], layout: &ScreenplayLayout, tol: f64) -> bool {
    !lines.iter().any(|c| if std::ptr::eq(c,line) { false } else {
        (is_vertically_adjacent(c,line,1.5) || is_vertically_adjacent(line,c,1.5)) && matches_established_role_geometry(c, layout, tol)
    })
}

fn is_at_established_center(line: &PhysicalTextLine, layout: &ScreenplayLayout) -> bool {
    if let Some(a) = &layout.action {
        let centers = [16.5, 18.0].iter().map(|m| a.x + line.height * m).collect::<Vec<_>>();
        centers.iter().any(|c| (line_center(line) - *c).abs() <= line.height)
    } else { false }
}

pub fn find_centered_action_lines(lines: &[PhysicalTextLine], blocked: &[PhysicalTextLine], layout: &ScreenplayLayout, tol: f64) -> Vec<PhysicalTextLine> {
    if layout.action.is_none() { return Vec::new(); }
    let ax = layout.action.as_ref().unwrap().x;
    let candidate_lines: Vec<PhysicalTextLine> = lines.iter().filter(|l| !ptr_in(blocked,l) && l.x > ax + l.height*6.0 && !has_baseline_sibling(l, lines, tol)).cloned().collect();
    let candidates: Vec<PhysicalTextLine> = candidate_lines.iter().filter(|l| !matches_established_role_geometry(l, layout, tol)).cloned().collect();
    let candidate_set_ptr: Vec<*const PhysicalTextLine> = candidates.iter().map(|l| l as *const PhysicalTextLine).collect();
    let mut centered: Vec<PhysicalTextLine> = Vec::new();
    let mut supported_groups: Vec<(f64, PhysicalTextLine, PhysicalTextLine)> = Vec::new();

    for i in 0..lines.len().saturating_sub(1) {
        let upper=&lines[i]; let lower=&lines[i+1];
        if ! (ptr_in(&candidates, upper) && ptr_in(&candidates, lower)
            && is_vertically_adjacent(upper, lower, 1.5)
            && (line_center(upper)-line_center(lower)).abs() <= tol
            && (upper.x - lower.x).abs() > tol) { continue; }
        if matches_established_content_geometry(upper, layout, tol) || matches_established_content_geometry(lower, layout, tol) { continue; }
        if matches_established_cue_geometry(upper, layout, tol.max(upper.height/2.0)) && matches_established_cue_geometry(lower, layout, tol.max(lower.height/2.0)) { continue; }
        let group_center = (line_center(upper)+line_center(lower))/2.0;
        // isolation content
        let isolated = ![preceding_line(lines,i), following_line(lines,i)].iter().any(|opt| if let Some(lineopt)=opt {
            (is_vertically_adjacent(lineopt, &lines[i],1.5) || is_vertically_adjacent(&lines[i+1], lineopt,1.5)) && matches_established_content_geometry(lineopt, layout, tol)
        } else { false });
        if !isolated { continue; }
        supported_groups.push((group_center, upper.clone(), lower.clone()));
        // add group lines
        // range not fully implemented; add upper/lower
        if !ptr_in(&centered, upper) { centered.push(upper.clone()); }
        if !ptr_in(&centered, lower) { centered.push(lower.clone()); }
    }

    // isolated candidates near supported groups
    for line in &candidates {
        if is_isolated_from_role_geometry(line, lines, layout, tol)
            && supported_groups.iter().any(|g| (line_center(line)-g.0).abs() <= line.height/4.0)
        {
            if !ptr_in(&centered, line) { centered.push(line.clone()); }
        }
    }

    // stable center peer
    for line in &candidates {
        let has_peer = candidates.iter().any(|c| !std::ptr::eq(c,line) && (line_center(c)-line_center(line)).abs() <= tol);
        if has_peer && is_at_established_center(line, layout) && is_isolated_from_role_geometry(line, lines, layout, tol) {
            if !ptr_in(&centered, line) { centered.push(line.clone()); }
        }
    }

    // centered pairs local support
    for i in 0..lines.len().saturating_sub(1) {
        let upper=&lines[i]; let lower=&lines[i+1];
        if !(ptr_in(&candidates, upper) && ptr_in(&candidates, lower)
            && is_vertically_adjacent(upper, lower, 1.5)
            && (line_center(upper)-line_center(lower)).abs() <= tol
            && (upper.x-lower.x).abs() > tol) { continue; }
        let group_center=(line_center(upper)+line_center(lower))/2.0;
        let has_support = supported_groups.iter().any(|g| {
            upper.page_index == g.1.page_index && ((line_center(upper)+line_center(lower))/2.0 - g.0).abs() <= tol
        });
        if has_support && is_centered_group_isolated(lines, i, group_center, &candidates, layout, tol) {
            if !ptr_in(&centered, upper) { centered.push(upper.clone()); }
            if !ptr_in(&centered, lower) { centered.push(lower.clone()); }
        }
    }
    centered
}

fn preceding_line(lines: &[PhysicalTextLine], idx: usize) -> Option<&PhysicalTextLine> {
    if idx == 0 { None } else { Some(&lines[idx-1]) }
}
fn following_line(lines: &[PhysicalTextLine], idx: usize) -> Option<&PhysicalTextLine> {
    if idx + 1 >= lines.len() { None } else { Some(&lines[idx+1]) }
}

fn is_centered_group_isolated(lines: &[PhysicalTextLine], seed: usize, _gc: f64, candidates: &[PhysicalTextLine], layout: &ScreenplayLayout, tol: f64) -> bool {
    let first=&lines[seed];
    let last=&lines[seed+1];
    let preceding = if seed==0 { None } else { Some(&lines[seed-1]) };
    let following = if seed+2 >= lines.len() { None } else { Some(&lines[seed+2]) };
    ![preceding, following].iter().any(|opt| {
        if let Some(line)=opt {
            (is_vertically_adjacent(line, first, 1.5) || is_vertically_adjacent(last, line, 1.5))
                && matches_established_content_geometry(line, layout, tol)
        } else { false }
    })
}