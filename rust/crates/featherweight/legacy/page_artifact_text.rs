use crate::model::PhysicalTextLine;

pub const MORE_MARKER: &str = "(MORE)";
pub const CONTINUED_MARKER: &str = "(CONTINUED)";

pub fn is_explicit_pagination_text(text: &str) -> bool {
    let t = text.trim();
    if t == MORE_MARKER || t == CONTINUED_MARKER { return true; }
    if t.starts_with("CONTINUED:") { return true; }
    if t.ends_with("(CONT'D)") || t.ends_with("(cont'd)") { return true; }
    false
}

pub struct TextFragment {
    pub font_family: String,
    pub font_size: f64,
    pub text: String,
    pub x: f64,
    pub y: f64,
}

pub fn create_text_fragments(line: &PhysicalTextLine) -> Vec<TextFragment> {
    let mut map = std::collections::BTreeMap::new();
    for span in &line.spans {
        let start = span.start;
        let end = span.end;
        let chars: Vec<char> = line.text.chars().collect();
        if end <= start || end > chars.len() { continue; }
        let text: String = chars[start..end].iter().collect::<String>().trim().to_string();
        if text.is_empty() { continue; }
        map.insert(text.clone(), TextFragment {
            font_family: span.bold.to_string(),
            font_size: line.height,
            text,
            x: span.x,
            y: span.y,
        });
    }
    if !map.contains_key(&line.text.trim().to_string()) && !line.spans.is_empty() {
        let text = line.text.trim().to_string();
        if !text.is_empty() {
            let s = &line.spans[0];
            map.insert(text.clone(), TextFragment {
                font_family: s.bold.to_string(),
                font_size: s.height,
                text,
                x: s.x,
                y: s.y,
            });
        }
    }
    map.into_values().collect()
}

pub struct CharacterCue { pub continued: bool, pub name: String, pub line: PhysicalTextLine }

pub fn parse_character_cue(line: &PhysicalTextLine) -> Option<CharacterCue> {
    let text = line.text.trim();
    // continued cue
    if text.ends_with("(CONT'D)") || text.ends_with("(cont'd)") {
        let name = text[..text.len()-9].trim().to_string();
        if name.chars().filter(|c| c.is_alphabetic()).count() >= 2 {
            return Some(CharacterCue { continued: true, name, line: line.clone() });
        }
    }
    // bare cue: all uppercase words
    if text.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c=='.' || c=='-' || c=='\'' || c=='(' || c==')') {
        if text.chars().filter(|c| c.is_alphabetic()).count() >= 2 {
            return Some(CharacterCue { continued: false, name: text.to_string(), line: line.clone() });
        }
    }
    None
}