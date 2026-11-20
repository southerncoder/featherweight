use crate::model::PhysicalTextLine;
use crate::screenplay::{StyledRun as Run, StyledText};
use crate::sentence_spacing::{SentenceSpacingConvention, is_sentence_boundary};

pub type LineJoin = &'static str; // "wrap" or "newline"

const STYLE_ORDER: [&str; 4] = ["bold", "italic", "underline", "strikeout"];

pub fn is_continuation_dash(value: &str) -> bool {
    value == "-" || value == "\u{2013}" || value == "\u{2014}"
}

fn styles_for_span(span: Option<&crate::model::PhysicalTextSpan>) -> Vec<String> {
    match span {
        Some(s) => STYLE_ORDER.iter().filter(|st| {
            match **st { "bold" => s.bold, "italic" => s.italic, "underline" => false, "strikeout" => false, _ => false }
        }).map(|s| s.to_string()).collect(),
        None => Vec::new(),
    }
}

fn separator_styles_for_spans(prev: Option<&crate::model::PhysicalTextSpan>, next: &crate::model::PhysicalTextSpan) -> Vec<String> {
    let a = styles_for_span(prev);
    let b = styles_for_span(Some(next));
    if a == b { a } else { Vec::new() }
}

fn have_same_styles(a: &[String], b: &[String]) -> bool {
    a == b
}

fn append_run(runs: &mut Vec<Run>, text: &str, styles: Vec<String>) {
    if text.is_empty() { return; }
    if let Some(prev) = runs.last_mut() {
        if have_same_styles(&prev.styles, &styles) {
            prev.text.push_str(text);
            return;
        }
    }
    runs.push(Run { text: text.to_string(), styles });
}

pub fn create_styled_text(lines: &[PhysicalTextLine], line_joins: &[&str], conv: SentenceSpacingConvention) -> StyledText {
    // supports single join strategy "wrap"/"newline" for simplicity
    let mut runs = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            let prev_line = &lines[idx - 1];
                        let join = if line_joins.len() == 1 { line_joins[0] } else if line_joins.len() >= idx { line_joins[idx - 1] } else { "wrap" };
            let separator = create_line_separator(prev_line, line, join, conv);
            if !separator.is_empty() {
                let sep_styles = separator_styles(prev_line, line);
                append_run(&mut runs, &separator, sep_styles);
            }
        }
        let mut previous_span_end = 0usize;
        let chars: Vec<char> = line.text.chars().collect();
        for (si, span) in line.spans.iter().enumerate() {
            // span.start/end are char counts
            if span.start > previous_span_end {
                let gap_text: String = chars[previous_span_end..span.start].iter().collect();
                let prev_span = if si > 0 { line.spans.get(si - 1) } else { None };
                let styles = separator_styles_for_spans(prev_span, span);
                append_run(&mut runs, &gap_text, styles);
            }
            let span_text: String = chars[span.start..span.end].iter().collect();
            append_run(&mut runs, &span_text, styles_for_span(Some(span)));
            previous_span_end = span.end;
        }
    }
    StyledText { runs }
}

fn create_line_separator(prev: &PhysicalTextLine, line: &PhysicalTextLine, join: &str, conv: SentenceSpacingConvention) -> String {
    if join == "newline" { return "\n".to_string(); }
    let trailing = prev.text.chars().rev().take_while(|c| c.is_whitespace()).count();
    let leading = line.text.chars().take_while(|c| c.is_whitespace()).count();
    if let Some(conv_val) = conv {
        let c = conv_val as usize;
        let retained = trailing + leading;
        if is_sentence_boundary(&prev.text, &line.text) && (trailing + leading) > 0 {
            let spaces = c.saturating_sub(retained);
            return " ".repeat(spaces);
        }
    }
    if trailing > 0 || leading > 0 { return String::new(); }
    let next_text = line.text.trim_start();
    if next_text.starts_with(',') || next_text.starts_with(';') || next_text.starts_with(':') || next_text.starts_with('!') || next_text.starts_with('?') || next_text.starts_with(')') || next_text.starts_with('\u{2026}')
        || is_continuation_dash(prev.text.chars().last().unwrap_or(' ').to_string().as_str()) {
        return String::new();
    }
    " ".to_string()
}

fn separator_styles(prev: &PhysicalTextLine, line: &PhysicalTextLine) -> Vec<String> {
    let a = styles_for_span(prev.spans.last());
    let b = styles_for_span(line.spans.first());
    if a == b { a } else { Vec::new() }
}

pub fn styled_text_value(text: &StyledText) -> String {
    text.runs.iter().map(|r| r.text.clone()).collect()
}

pub fn slice_styled_text(text: &StyledText, end: usize) -> StyledText {
    let mut runs = Vec::new();
    let mut remaining = end;
    for run in &text.runs {
        if remaining <= 0 { break; }
        let chars: Vec<char> = run.text.chars().collect();
        let take = remaining.min(chars.len());
        let slice_text: String = chars[..take].iter().collect();
        append_run(&mut runs, &slice_text, run.styles.clone());
        remaining -= run.text.chars().count();
    }
    StyledText { runs }
}

pub fn trim_styled_text_end(text: StyledText) -> StyledText {
    let v = styled_text_value(&text);
    slice_styled_text(&text, v.trim_end().chars().count())
}