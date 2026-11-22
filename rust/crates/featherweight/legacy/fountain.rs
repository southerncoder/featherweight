use crate::screenplay::{ScreenplayDocument, ScreenplayElement, StyledText};

pub fn screenplay_document_to_fountain(document: &ScreenplayDocument) -> String {
    let title_page = serialize_title_page(&document.title_page);
    let body_blocks = serialize_body(&document.elements, title_page.is_empty());
    let sections: Vec<String> = if !title_page.is_empty() {
        let mut v = vec![title_page]; v.extend(body_blocks); v
    } else { body_blocks };
    if sections.is_empty() { return String::new(); }
    format!("{}\n", sections.join("\n\n"))
}

fn serialize_title_page(fields: &[crate::screenplay::TitleField]) -> String {
    fields.iter().map(serialize_title_page_field).collect::<Vec<_>>().join("\n")
}

fn serialize_title_page_field(field: &crate::screenplay::TitleField) -> String {
    if field.values.is_empty() { return format!("{}:", field.key); }
    let serialized: Vec<String> = field.values.iter().map(styled_text_to_fountain).collect();
    let uses_indented = serialized.len() > 1 || serialized.iter().any(|v| v.contains('\n'));
    if !uses_indented { return format!("{}: {}", field.key, serialized[0]); }
    let indented: Vec<String> = serialized.iter().flat_map(|v| v.split('\n').map(|l| format!("    {l}"))).collect();
    let mut out = vec![format!("{}:", field.key)];
    out.extend(indented);
    out.join("\n")
}

fn serialize_body(elements: &[ScreenplayElement], body_starts_document: bool) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < elements.len() {
        if let ScreenplayElement::Character { .. } = &elements[i] {
            let mut dialogue_lines = vec![serialize_character(&elements[i], false)];
            let mut j = i + 1;
            while j < elements.len() && is_dialogue_content(&elements[j]) {
                dialogue_lines.push(serialize_dialogue_content(&elements[j]));
                j += 1;
            }
            blocks.push(dialogue_lines.join("\n"));
            i = j;
            continue;
        }
        blocks.push(serialize_element(&elements[i], body_starts_document && i == 0));
        i += 1;
    }
    blocks
}

fn is_dialogue_content(e: &ScreenplayElement) -> bool {
    matches!(e, ScreenplayElement::Parenthetical { .. } | ScreenplayElement::Dialogue { .. })
}

fn serialize_element(element: &ScreenplayElement, starts_document: bool) -> String {
    match element {
        ScreenplayElement::SceneHeading { text, scene_number } => {
            let txt = rendered_text(text);
            let force = if is_automatic_scene_heading(&txt) || txt.starts_with('.') { "" } else { "." };
            let num = scene_number.as_ref().map(|n| format!(" #{}#", n)).unwrap_or_default();
            format!("{}{}{}", force, unstyled_text_to_fountain(text), num)
        }
        ScreenplayElement::Action { text, alignment } => serialize_action(text, alignment, starts_document),
        ScreenplayElement::Character { text } => serialize_character(element, false),
        ScreenplayElement::Parenthetical { text } => styled_text_to_fountain(text),
        ScreenplayElement::Dialogue { text } => {
            let lines = styled_text_to_fountain(text).split('\n').map(|l| if l.is_empty() { "  ".to_string() } else { l.to_string() }).collect::<Vec<_>>();
            lines.join("\n")
        }
        ScreenplayElement::DualDialogue { left, right } => {
            let l = serialize_dialogue_sequence(left, false);
            let r = serialize_dialogue_sequence(right, true);
            format!("{}\n\n{}", l, r)
        }
        ScreenplayElement::Lyrics { text } => styled_text_to_fountain(text).split('\n').map(|l| format!("~{l}")).collect::<Vec<_>>().join("\n"),
        ScreenplayElement::Transition { text } => {
            let txt = rendered_text(text);
            let force = if is_automatic_transition(&txt) { "" } else { ">" };
            format!("{}{}", force, unstyled_text_to_fountain(text))
        }
        ScreenplayElement::PageBreak {} => "===".to_string(),
    }
}

fn serialize_action(text: &StyledText, alignment: &str, starts_document: bool) -> String {
    let styled = styled_text_to_fountain(text);
    if alignment == "center" {
        return styled.split('\n').map(|l| format!(">{l}<")).collect::<Vec<_>>().join("\n");
    }
    let lines: Vec<&str> = styled.split('\n').collect();
    let rendered = rendered_text(text);
    let unescaped: Vec<&str> = rendered.split('\n').collect();
    lines.iter().enumerate().map(|(idx, line)| {
        let syntax = line.trim_start();
        if line_requires_force_marker(&lines, &unescaped, idx, starts_document) {
            format!("!{}", line)
        } else { line.to_string() }
    }).collect::<Vec<_>>().join("\n")
}

fn line_requires_force_marker(lines: &[&str], unescaped: &[&str], idx: usize, action_starts: bool) -> bool {
    let line = lines[idx];
    let syntax = line.trim_start();
    let unescaped_syntax = unescaped[idx].trim_start();
    if syntax.is_empty() { return false; }
    if "![=@~>".contains(syntax.chars().next().unwrap_or(' ')) { return true; } // crude
    let prev_blank = idx == 0 || is_blank(lines[idx-1]);
    if !prev_blank { return false; }
    let next_blank = idx == lines.len()-1 || is_blank(lines[idx+1]);
    if next_blank && (syntax.starts_with('.') && syntax.chars().nth(1).map(|c| c.is_alphanumeric()).unwrap_or(false) || automatic_scene_heading(syntax)) {
        return true;
    }
    if (!next_blank && is_automatic_character(syntax)) || (next_blank && is_automatic_transition(syntax)) {
        return true;
    }
    false
}

fn is_blank(s: &str) -> bool { s.trim().is_empty() }

fn serialize_character(element: &ScreenplayElement, dual: bool) -> String {
    if let ScreenplayElement::Character { text } = element {
        let txt = rendered_text(text);
        let force = if is_automatic_character(&txt) { "" } else { "@" };
        let dual_marker = if dual { " ^" } else { "" };
        format!("{}{}{}", force, unstyled_text_to_fountain(text), dual_marker)
    } else { String::new() }
}

fn serialize_dialogue_sequence(seq: &crate::screenplay::DialogueSequence, dual: bool) -> String {
    let mut parts = vec![serialize_character(&seq.character, dual)];
    for c in &seq.content { parts.push(serialize_dialogue_content(c)); }
    parts.join("\n")
}

fn serialize_dialogue_content(content: &ScreenplayElement) -> String {
    match content {
        ScreenplayElement::Parenthetical { text } => styled_text_to_fountain(text),
        ScreenplayElement::Dialogue { text } => {
            styled_text_to_fountain(text).split('\n').map(|l| if l.is_empty() { "  " } else { l }).collect::<Vec<_>>().join("\n")
        }
        _ => String::new(),
    }
}

fn automatic_scene_heading(s: &str) -> bool {
    let u = s.to_uppercase();
    u.starts_with("INT") || u.starts_with("EXT") || u.starts_with("I/E") || u.starts_with("EST")
}

fn rendered_text(text: &StyledText) -> String {
    text.runs.iter().map(|r| r.text.clone()).collect::<String>().replace("\r\n", "\n").replace('\r', "\n")
}

fn styled_text_to_fountain(text: &StyledText) -> String {
    let mut chars: Vec<(char, Vec<String>)> = Vec::new();
    for run in &text.runs {
        let styles: Vec<String> = normalize_styles(&run.styles);
        let normalized = run.text.replace("\r\n", "\n").replace('\r', "\n");
        for c in normalized.chars() { chars.push((c, styles.clone())); }
    }
    let mut output = Vec::new();
    let mut active: Vec<String> = Vec::new();
    for (i, (ch, styles)) in chars.iter().enumerate() {
        if *ch == '\n' {
            close_styles(&mut output, &active);
            active.clear();
            output.push("\n".to_string());
            continue;
        }
        let next = if ch.is_whitespace() {
            // whitespace styles
            let left = neighbor_styles(&chars, i, -1);
            let right = neighbor_styles(&chars, i, 1);
            let shared = common_prefix(&left, &right);
            left[..shared].to_vec()
        } else { styles.clone() };
        let shared = common_prefix(&active, &next);
        close_styles(&mut output, &active[shared..].to_vec());
        for s in &next[shared..] { output.push(marker(s).to_string()); }
        active = next;
        output.push(escape(ch.to_string()));
    }
    close_styles(&mut output, &active);
    output.join("")
}

fn normalize_styles(styles: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for s in ["underline", "bold", "italic"] {
        if styles.iter().any(|x| x == s) { out.push(s.to_string()); }
    }
    out
}

fn neighbor_styles(chars: &[(char, Vec<String>)], idx: usize, dir: i32) -> Vec<String> {
    let mut i = idx as i32 + dir;
    while i >= 0 && i < chars.len() as i32 {
        let (ch, styles) = &chars[i as usize];
        if *ch == '\n' { return Vec::new(); }
        if !ch.is_whitespace() { return styles.clone(); }
        i += dir;
    }
    Vec::new()
}

fn common_prefix(a: &[String], b: &[String]) -> usize {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] { return i; }
    }
    n
}

fn close_styles(output: &mut Vec<String>, styles: &[String]) {
    for s in styles.iter().rev() {
        output.push(marker(s).to_string());
    }
}

fn marker(s: &str) -> &'static str {
    match s { "underline" => "_", "bold" => "**", "italic" => "*", _ => "" }
}

fn escape(text: String) -> String {
    text.replace('\\', "\\\\").replace('*', "\\*").replace('_', "\\_")
}

fn unstyled_text_to_fountain(text: &StyledText) -> String {
    escape(rendered_text(text))
}

fn is_automatic_scene_heading(text: &str) -> bool {
    let u = text.to_uppercase();
    u.starts_with("INT") || u.starts_with("EXT") || u.starts_with("I/E") || u.starts_with("EST")
}

fn is_automatic_character(text: &str) -> bool {
    text == text.to_uppercase() && text.chars().any(|c| c.is_alphabetic())
}

fn is_automatic_transition(text: &str) -> bool {
    let u = text.to_uppercase();
    u == u && u.ends_with("TO:")
}