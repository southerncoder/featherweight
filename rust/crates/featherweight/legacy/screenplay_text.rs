pub fn is_uppercase_text(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase())
}

pub fn remove_trailing_parenthesized_groups(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut prefix_end = chars.len();
    while prefix_end > 0 && chars[prefix_end - 1].is_whitespace() {
        prefix_end -= 1;
    }
    loop {
        if prefix_end == 0 || chars[prefix_end - 1] != ')' {
            break;
        }
        let mut depth = 0usize;
        let mut opening = None;
        for idx in (0..prefix_end).rev() {
            if chars[idx] == ')' {
                depth += 1;
            } else if chars[idx] == '(' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    opening = Some(idx);
                    break;
                }
            }
        }
        match opening {
            Some(oi) => {
                let mut new_end = oi;
                while new_end > 0 && chars[new_end - 1].is_whitespace() {
                    new_end -= 1;
                }
                prefix_end = new_end;
            }
            None => break,
        }
    }
    chars[..prefix_end].iter().collect()
}

pub fn is_cue_uppercase_text(text: &str) -> bool {
    is_uppercase_text(&remove_trailing_parenthesized_groups(text))
}

pub fn is_parenthetical(text: &str) -> bool {
    let t = text.trim();
    t.starts_with('(') && t.ends_with(')')
}

pub fn is_parenthetical_fragment(text: &str) -> bool {
    let t = text.trim();
    t.starts_with('(') || t.ends_with(')')
}

pub fn is_cue_parenthetical_fragment(text: &str) -> bool {
    let t = text.trim();
    t.starts_with('(') || (t.ends_with(')') && !t.contains('('))
}