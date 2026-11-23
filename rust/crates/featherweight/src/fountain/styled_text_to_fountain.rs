//! Port of `src/fountain/styled-text-to-fountain.ts`.
use crate::core::screenplay_document::{StyledText, TextStyle};
use crate::core::text::is_js_whitespace;

/// `supportedStyles`: the styles Fountain can express, in marker nesting order.
const SUPPORTED_STYLES: [TextStyle; 3] = [TextStyle::Underline, TextStyle::Bold, TextStyle::Italic];

/// One code point with its normalized supported styles (`StyledCharacter`).
struct StyledCharacter {
    character: char,
    styles: Vec<TextStyle>,
}

/// `styleMarkers`.
fn style_marker(style: TextStyle) -> &'static str {
    match style {
        TextStyle::Underline => "_",
        TextStyle::Bold => "**",
        TextStyle::Italic => "*",
        // `normalizeStyles` drops strikeout (Fountain has no syntax for it); never
        // reached, but emit nothing rather than panic on hand-built documents.
        TextStyle::Strikeout => "",
    }
}

/// Port of `styledTextToFountain`: escapes the text and wraps styled spans in
/// Fountain emphasis markers, closing every style at each newline.
pub fn styled_text_to_fountain(text: &StyledText) -> String {
    let characters = styled_characters(text);
    let mut output = String::new();
    let mut active_styles: Vec<TextStyle> = Vec::new();

    for (character_index, styled_character) in characters.iter().enumerate() {
        if styled_character.character == '\n' {
            close_styles(&mut output, &active_styles);
            active_styles.clear();
            output.push('\n');
            continue;
        }

        let next_styles = if is_js_whitespace(styled_character.character) {
            whitespace_styles(&characters, character_index)
        } else {
            styled_character.styles.clone()
        };

        transition_styles(&mut output, &active_styles, &next_styles);
        active_styles = next_styles;
        push_escaped(&mut output, styled_character.character);
    }

    close_styles(&mut output, &active_styles);
    output
}

/// Port of `unstyledTextToFountain`.
pub fn unstyled_text_to_fountain(text: &StyledText) -> String {
    escape_fountain_text(&rendered_text(text))
}

/// Port of `renderedText`: the concatenated run text with `\r\n` and `\r`
/// normalized to `\n`.
pub fn rendered_text(text: &StyledText) -> String {
    let joined: String = text.runs.iter().map(|run| run.text.as_str()).collect();
    normalize_line_endings(&joined)
}

/// `value.replace(/\r\n|\r/g, "\n")`.
pub(crate) fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn styled_characters(text: &StyledText) -> Vec<StyledCharacter> {
    text.runs
        .iter()
        .flat_map(|run| {
            let styles = normalize_styles(&run.styles);
            normalize_line_endings(&run.text)
                .chars()
                .map(|character| StyledCharacter {
                    character,
                    styles: styles.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn whitespace_styles(characters: &[StyledCharacter], character_index: usize) -> Vec<TextStyle> {
    let left_styles = neighboring_content_styles(characters, character_index, Direction::Left);
    let right_styles = neighboring_content_styles(characters, character_index, Direction::Right);

    left_styles[..common_prefix_length(left_styles, right_styles)].to_vec()
}

#[derive(Clone, Copy)]
enum Direction {
    Left,
    Right,
}

fn neighboring_content_styles(
    characters: &[StyledCharacter],
    character_index: usize,
    direction: Direction,
) -> &[TextStyle] {
    let neighbors: Box<dyn Iterator<Item = &StyledCharacter>> = match direction {
        Direction::Left => Box::new(characters[..character_index].iter().rev()),
        Direction::Right => Box::new(characters[character_index + 1..].iter()),
    };

    for neighbor in neighbors {
        if neighbor.character == '\n' {
            return &[];
        }

        if !is_js_whitespace(neighbor.character) {
            return &neighbor.styles;
        }
    }

    &[]
}

fn normalize_styles(styles: &[TextStyle]) -> Vec<TextStyle> {
    SUPPORTED_STYLES
        .into_iter()
        .filter(|style| styles.contains(style))
        .collect()
}

fn transition_styles(output: &mut String, active_styles: &[TextStyle], next_styles: &[TextStyle]) {
    let shared_style_count = common_prefix_length(active_styles, next_styles);

    close_styles(output, &active_styles[shared_style_count..]);

    for &style in &next_styles[shared_style_count..] {
        output.push_str(style_marker(style));
    }
}

fn close_styles(output: &mut String, styles: &[TextStyle]) {
    for &style in styles.iter().rev() {
        output.push_str(style_marker(style));
    }
}

fn common_prefix_length(left: &[TextStyle], right: &[TextStyle]) -> usize {
    left.iter().zip(right).take_while(|(l, r)| l == r).count()
}

fn push_escaped(output: &mut String, character: char) {
    if matches!(character, '\\' | '*' | '_') {
        output.push('\\');
    }
    output.push(character);
}

/// `escapeFountainText`: backslash-escapes `\`, `*` and `_`.
fn escape_fountain_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        push_escaped(&mut output, character);
    }
    output
}
