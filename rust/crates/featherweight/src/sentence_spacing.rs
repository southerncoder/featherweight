pub type SentenceSpacingConvention = Option<u8>;

fn is_terminal(ch: char) -> bool { matches!(ch, '.' | '!' | '?' | '\u{2026}') }

pub fn infer_sentence_spacing_convention(lines: &[crate::model::PhysicalTextLine]) -> SentenceSpacingConvention {
    let mut one = 0usize;
    let mut two = 0usize;
    for line in lines {
        let chars: Vec<char> = line.text.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if is_terminal(chars[i]) {
                // skip closing quotes/parens
                let mut j = i + 1;
                while j < chars.len() && matches!(chars[j], '"' | '\'' | '\u{2019}' | '\u{201d}' | ')' | ']') { j += 1; }
                // count spaces
                let space_start = j;
                while j < chars.len() && chars[j] == ' ' { j += 1; }
                let spacing_len = j - space_start;
                if spacing_len > 0 && j < chars.len() && (chars[j].is_uppercase() || chars[j].is_ascii_digit()) {
                    if spacing_len == 1 { one += 1; } else { two += 1; }
                }
                i = j.max(i + 1);
            } else {
                i += 1;
            }
        }
    }
    if one == two { None } else { Some(if one > two { 1 } else { 2 }) }
}

pub fn is_sentence_boundary(previous_text: &str, next_text: &str) -> bool {
    let prev = previous_text.trim_end();
    let next = next_text.trim_start();
    let last = prev.chars().last().unwrap_or(' ');
    if !matches!(last, '.' | '!' | '?' | '\u{2026}' | '"' | '\'' | '\u{2019}' | '\u{201d}' | ')' | ']') { return false; }
    // allow trailing quote
    let first = next.chars().next().unwrap_or(' ');
    first.is_uppercase() || first.is_ascii_digit()
}