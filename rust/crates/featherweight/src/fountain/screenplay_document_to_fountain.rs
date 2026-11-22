//! Port of `src/fountain/screenplay-document-to-fountain.ts`.
use std::sync::LazyLock;

use regex::Regex;

use crate::core::screenplay_document::{
    ActionAlignment, Character, DialogueContent, DialogueSequence, ScreenplayDocument,
    ScreenplayElement, StyledText, TitlePageField,
};
use crate::core::text::{js_trim, js_trim_start};

use super::styled_text_to_fountain::{
    rendered_text, styled_text_to_fountain, unstyled_text_to_fountain,
};

/// The alternatives of `automaticSceneHeadingPattern`
/// (`/^(?:INT\.\/EXT|INT\/EXT|INT|EXT|EST|I\/E)(?:[. ])/i`).
const AUTOMATIC_SCENE_HEADING_PREFIXES: [&str; 6] = ["INT./EXT", "INT/EXT", "INT", "EXT", "EST", "I/E"];

static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}").unwrap());
static TITLE_SHAPED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{N} _-]+:").unwrap());
static FORCED_SCENE_HEADING_SHAPED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.[\p{L}\p{N}]").unwrap());

/// Port of `screenplayDocumentToFountain`: serializes a document to Fountain
/// text, adding force markers only where automatic recognition would misread
/// an element.
pub fn screenplay_document_to_fountain(document: &ScreenplayDocument) -> String {
    let title_page = serialize_title_page(&document.title_page);
    let body_blocks = serialize_body(&document.elements, title_page.is_empty());
    let mut document_sections = Vec::with_capacity(body_blocks.len() + 1);

    if !title_page.is_empty() {
        document_sections.push(title_page);
    }
    document_sections.extend(body_blocks);

    if document_sections.is_empty() {
        return String::new();
    }

    format!("{}\n", document_sections.join("\n\n"))
}

fn serialize_title_page(fields: &[TitlePageField]) -> String {
    fields.iter().map(serialize_title_page_field).collect::<Vec<_>>().join("\n")
}

fn serialize_title_page_field(field: &TitlePageField) -> String {
    if field.values.is_empty() {
        return format!("{}:", field.key);
    }

    let serialized_values: Vec<String> = field.values.iter().map(styled_text_to_fountain).collect();
    let uses_indented_layout =
        serialized_values.len() > 1 || serialized_values.iter().any(|value| value.contains('\n'));

    if !uses_indented_layout {
        return format!("{}: {}", field.key, serialized_values[0]);
    }

    let mut lines = vec![format!("{}:", field.key)];
    lines.extend(
        serialized_values
            .iter()
            .flat_map(|value| value.split('\n').map(|line| format!("    {line}"))),
    );
    lines.join("\n")
}

fn serialize_body(elements: &[ScreenplayElement], body_starts_document: bool) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut element_index = 0;

    while element_index < elements.len() {
        let element = &elements[element_index];

        if let ScreenplayElement::Character { text } = element {
            let mut dialogue_lines = vec![serialize_character(text, false)];

            while let Some(content) = elements.get(element_index + 1).and_then(dialogue_content) {
                element_index += 1;
                dialogue_lines.push(content);
            }

            blocks.push(dialogue_lines.join("\n"));
        } else {
            blocks.push(serialize_element(element, body_starts_document && element_index == 0));
        }

        element_index += 1;
    }

    blocks
}

/// `isDialogueContent` combined with `serializeDialogueContent` for a
/// top-level element.
fn dialogue_content(element: &ScreenplayElement) -> Option<String> {
    match element {
        ScreenplayElement::Parenthetical { text } => Some(serialize_parenthetical(text)),
        ScreenplayElement::Dialogue { text } => Some(serialize_dialogue(text)),
        _ => None,
    }
}

fn serialize_element(element: &ScreenplayElement, starts_document: bool) -> String {
    match element {
        ScreenplayElement::SceneHeading { text, scene_number } => {
            serialize_scene_heading(text, scene_number.as_deref())
        }
        ScreenplayElement::Action { text, alignment } => {
            serialize_action(text, *alignment, starts_document)
        }
        ScreenplayElement::Character { text } => serialize_character(text, false),
        ScreenplayElement::Parenthetical { text } => serialize_parenthetical(text),
        ScreenplayElement::Dialogue { text } => serialize_dialogue(text),
        ScreenplayElement::DualDialogue { left, right } => serialize_dual_dialogue(left, right),
        ScreenplayElement::Lyric { text } => serialize_lyric(text),
        ScreenplayElement::Transition { text } => serialize_transition(text),
        ScreenplayElement::PageBreak => "===".to_owned(),
    }
}

fn serialize_scene_heading(text: &StyledText, scene_number: Option<&str>) -> String {
    let rendered = rendered_text(text);
    let force_marker =
        if is_automatic_scene_heading(&rendered) || rendered.starts_with('.') { "" } else { "." };
    let scene_number = scene_number.map(|number| format!(" #{number}#")).unwrap_or_default();

    format!("{force_marker}{}{scene_number}", unstyled_text_to_fountain(text))
}

fn serialize_action(text: &StyledText, alignment: ActionAlignment, starts_document: bool) -> String {
    let serialized = styled_text_to_fountain(text);

    if alignment == ActionAlignment::Center {
        return serialized.split('\n').map(|line| format!(">{line}<")).collect::<Vec<_>>().join("\n");
    }

    let lines: Vec<&str> = serialized.split('\n').collect();
    let rendered = rendered_text(text);
    let unescaped_lines: Vec<&str> = rendered.split('\n').collect();

    lines
        .iter()
        .enumerate()
        .map(|(line_index, line)| {
            if action_line_requires_force_marker(&lines, &unescaped_lines, line_index, starts_document)
            {
                format!("!{line}")
            } else {
                (*line).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn serialize_character(text: &StyledText, dual: bool) -> String {
    let rendered = rendered_text(text);
    let force_marker = if is_automatic_character(&rendered) { "" } else { "@" };
    let dual_marker = if dual { " ^" } else { "" };

    format!("{force_marker}{}{dual_marker}", unstyled_text_to_fountain(text))
}

fn serialize_parenthetical(text: &StyledText) -> String {
    styled_text_to_fountain(text)
}

fn serialize_dialogue(text: &StyledText) -> String {
    styled_text_to_fountain(text)
        .split('\n')
        .map(|line| if line.is_empty() { "  " } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

fn serialize_dual_dialogue(left: &DialogueSequence, right: &DialogueSequence) -> String {
    [serialize_dialogue_sequence(left, false), serialize_dialogue_sequence(right, true)].join("\n\n")
}

fn serialize_dialogue_sequence(sequence: &DialogueSequence, dual: bool) -> String {
    let Character { text } = &sequence.character;
    let mut lines = vec![serialize_character(text, dual)];
    lines.extend(sequence.content.iter().map(serialize_dialogue_content));
    lines.join("\n")
}

fn serialize_dialogue_content(content: &DialogueContent) -> String {
    match content {
        DialogueContent::Parenthetical { text } => serialize_parenthetical(text),
        DialogueContent::Dialogue { text } => serialize_dialogue(text),
    }
}

fn serialize_lyric(text: &StyledText) -> String {
    styled_text_to_fountain(text).split('\n').map(|line| format!("~{line}")).collect::<Vec<_>>().join("\n")
}

fn serialize_transition(text: &StyledText) -> String {
    let rendered = rendered_text(text);
    let force_marker = if is_automatic_transition(&rendered) { "" } else { ">" };

    format!("{force_marker}{}", unstyled_text_to_fountain(text))
}

/// `automaticSceneHeadingPattern.test(text)`. The pattern's `/i` flag has no
/// `u` flag, so only ASCII letters fold; matching by hand with ASCII-only case
/// folding avoids the regex crate's Unicode folds (e.g. U+017F → `s`).
fn is_automatic_scene_heading(text: &str) -> bool {
    let bytes = text.as_bytes();

    AUTOMATIC_SCENE_HEADING_PREFIXES.iter().any(|prefix| {
        bytes.len() > prefix.len()
            && bytes[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
            && matches!(bytes[prefix.len()], b'.' | b' ')
    })
}

fn is_automatic_character(text: &str) -> bool {
    text == text.to_uppercase() && LETTER.is_match(text)
}

fn is_automatic_transition(text: &str) -> bool {
    text == text.to_uppercase() && text.ends_with("TO:")
}

fn action_line_requires_force_marker(
    lines: &[&str],
    unescaped_lines: &[&str],
    line_index: usize,
    action_starts_document: bool,
) -> bool {
    let syntax = js_trim_start(lines[line_index]);
    // Deliberate divergence: TypeScript reads `unescapedLines[lineIndex]!.trimStart()`
    // and throws a TypeError when per-run line-ending normalization yields more
    // lines than the joined text (a run ending in `\r` followed by one starting
    // with `\n`). An editor must not crash on that, so a missing line is treated
    // as empty, which only skips the title-shaped force-marker check below.
    let unescaped_syntax = js_trim_start(unescaped_lines.get(line_index).copied().unwrap_or_default());

    if syntax.is_empty() {
        return false;
    }

    if syntax.starts_with(['!', '@', '#', '=', '~', '>']) {
        return true;
    }

    if action_starts_document && line_index == 0 && TITLE_SHAPED.is_match(unescaped_syntax) {
        return true;
    }

    let previous_line_is_blank = line_index == 0 || is_blank_fountain_line(lines[line_index - 1]);

    if !previous_line_is_blank {
        return false;
    }

    let next_line_is_blank =
        line_index == lines.len() - 1 || is_blank_fountain_line(lines[line_index + 1]);

    if next_line_is_blank
        && (FORCED_SCENE_HEADING_SHAPED.is_match(syntax) || is_automatic_scene_heading(syntax))
    {
        return true;
    }

    (!next_line_is_blank && is_automatic_character(syntax))
        || (next_line_is_blank && is_automatic_transition(syntax))
}

fn is_blank_fountain_line(line: &str) -> bool {
    js_trim(line).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::screenplay_document::{TextRun, TextStyle};

    use TextStyle::{Bold, Italic, Strikeout, Underline};

    fn run(text: &str, styles: &[TextStyle]) -> TextRun {
        TextRun { text: text.to_owned(), styles: styles.to_vec() }
    }

    fn styled(runs: Vec<TextRun>) -> StyledText {
        StyledText { runs }
    }

    fn plain(text: &str) -> StyledText {
        styled(vec![run(text, &[])])
    }

    fn scene(text: StyledText) -> ScreenplayElement {
        ScreenplayElement::SceneHeading { text, scene_number: None }
    }

    fn action(text: StyledText) -> ScreenplayElement {
        ScreenplayElement::Action { text, alignment: ActionAlignment::Standard }
    }

    fn action_str(text: &str) -> ScreenplayElement {
        action(plain(text))
    }

    fn character(text: StyledText) -> ScreenplayElement {
        ScreenplayElement::Character { text }
    }

    fn parenthetical(text: StyledText) -> ScreenplayElement {
        ScreenplayElement::Parenthetical { text }
    }

    fn dialogue(text: &str) -> ScreenplayElement {
        ScreenplayElement::Dialogue { text: plain(text) }
    }

    fn transition(text: StyledText) -> ScreenplayElement {
        ScreenplayElement::Transition { text }
    }

    fn field(key: &str, values: Vec<StyledText>) -> TitlePageField {
        TitlePageField { key: key.to_owned(), values }
    }

    fn doc(title_page: Vec<TitlePageField>, elements: Vec<ScreenplayElement>) -> ScreenplayDocument {
        ScreenplayDocument { title_page, elements }
    }

    fn body(elements: Vec<ScreenplayElement>) -> String {
        screenplay_document_to_fountain(&doc(vec![], elements))
    }

    #[test]
    fn serializes_every_model_only_variant_and_shape_driven_title_layout() {
        let document = doc(
            vec![
                field("Open key", vec![plain("One line")]),
                field("Gallery", vec![styled(vec![run("first", &[Underline])]), plain("second\nline")]),
                field("Empty", vec![]),
            ],
            vec![
                ScreenplayElement::SceneHeading {
                    text: plain("INT. LAB - NIGHT"),
                    scene_number: Some("A1.2-B".to_owned()),
                },
                scene(plain("MEMORY")),
                action_str("The machine wakes."),
                ScreenplayElement::Action {
                    text: styled(vec![
                        run("CENTER ", &[]),
                        run("ONE\nCENTER", &[Bold]),
                        run(" TWO", &[Underline]),
                    ]),
                    alignment: ActionAlignment::Center,
                },
                ScreenplayElement::Lyric {
                    text: styled(vec![
                        run("Sing ", &[]),
                        run("one\nSing", &[Italic]),
                        run(" two", &[Underline]),
                    ]),
                },
                transition(plain("DISSOLVE THROUGH TIME")),
                ScreenplayElement::PageBreak,
            ],
        );

        assert_eq!(
            screenplay_document_to_fountain(&document),
            concat!(
                "Open key: One line\n",
                "Gallery:\n",
                "    _first_\n",
                "    second\n",
                "    line\n",
                "Empty:\n\n",
                "INT. LAB - NIGHT #A1.2-B#\n\n",
                ".MEMORY\n\n",
                "The machine wakes.\n\n",
                ">CENTER **ONE**<\n",
                ">**CENTER** _TWO_<\n\n",
                "~Sing *one*\n",
                "~*Sing* _two_\n\n",
                ">DISSOLVE THROUGH TIME\n\n",
                "===\n",
            )
        );
    }

    #[test]
    fn uses_automatic_recognition_minimally_while_omitting_styled_structural_role_emphasis() {
        let output = body(vec![
            scene(styled(vec![run("EXT. ROAD - DAY", &[Bold, Underline])])),
            scene(styled(vec![run("int. cellar - night", &[Italic])])),
            scene(styled(vec![run("FLASHBACK", &[Bold])])),
            character(styled(vec![run("GUIDE", &[Bold])])),
            dialogue("Automatic cue."),
            character(styled(vec![run("Guide Two", &[Underline])])),
            dialogue("Forced cue."),
            transition(styled(vec![run("CUT TO:", &[Italic])])),
            transition(styled(vec![run("MATCH DISSOLVE", &[Bold])])),
            action_str("A safe action line."),
            action_str("THE END"),
            action_str("...where"),
            action_str("    INT. ROOM"),
            action_str("Lead.\n\nINT. ROOM"),
            action_str("- VOICE\nDialogue"),
            action_str("int. false\ncontinuation"),
            action_str("INT. FALSE HEADING - DAY"),
            action_str("FALSE CUE\nDialogue-looking continuation."),
            action_str("SMASH TO:"),
            action_str(">Centered impostor<"),
            action_str("~Lyric impostor"),
            action_str("# Section impostor"),
            action_str("= Synopsis impostor"),
            action_str("==="),
        ]);

        assert_eq!(
            output,
            concat!(
                "EXT. ROAD - DAY\n\n",
                "int. cellar - night\n\n",
                ".FLASHBACK\n\n",
                "GUIDE\n",
                "Automatic cue.\n\n",
                "@Guide Two\n",
                "Forced cue.\n\n",
                "CUT TO:\n\n",
                ">MATCH DISSOLVE\n\n",
                "A safe action line.\n\n",
                "THE END\n\n",
                "...where\n\n",
                "!    INT. ROOM\n\n",
                "Lead.\n\n",
                "!INT. ROOM\n\n",
                "!- VOICE\n",
                "Dialogue\n\n",
                "int. false\n",
                "continuation\n\n",
                "!INT. FALSE HEADING - DAY\n\n",
                "!FALSE CUE\n",
                "Dialogue-looking continuation.\n\n",
                "!SMASH TO:\n\n",
                "!>Centered impostor<\n\n",
                "!~Lyric impostor\n\n",
                "!# Section impostor\n\n",
                "!= Synopsis impostor\n\n",
                "!===\n",
            )
        );
    }

    #[test]
    fn preserves_ordinary_and_dual_dialogue_ownership_including_empty_interior_lines() {
        let output = body(vec![
            character(plain("CAPTAIN")),
            parenthetical(styled(vec![run("(", &[]), run("quietly", &[Italic]), run(")", &[])])),
            dialogue("first line\n\nsecond line"),
            character(plain("Crew mate")),
            dialogue("Still here."),
            ScreenplayElement::DualDialogue {
                left: DialogueSequence {
                    character: Character { text: plain("LEFT VOICE") },
                    content: vec![
                        DialogueContent::Parenthetical { text: plain("(reading)") },
                        DialogueContent::Dialogue { text: plain("Left one\nLeft two") },
                    ],
                },
                right: DialogueSequence {
                    character: Character { text: plain("Right voice") },
                    content: vec![
                        DialogueContent::Dialogue { text: plain("Right one") },
                        DialogueContent::Parenthetical { text: plain("(then)") },
                        DialogueContent::Dialogue { text: plain("Right two") },
                    ],
                },
            },
        ]);

        assert_eq!(
            output,
            concat!(
                "CAPTAIN\n",
                "(*quietly*)\n",
                "first line\n",
                "  \n",
                "second line\n\n",
                "@Crew mate\n",
                "Still here.\n\n",
                "LEFT VOICE\n",
                "(reading)\n",
                "Left one\n",
                "Left two\n\n",
                "@Right voice ^\n",
                "Right one\n",
                "(then)\n",
                "Right two\n",
            )
        );
    }

    #[test]
    fn serializes_supported_styling_escaping_newline_boundaries_and_representable_boneyards() {
        let output = body(vec![
            action(styled(vec![
                run("Literal \\ * _ markers. ", &[]),
                run("italic", &[Italic]),
                run(", ", &[]),
                run("bold", &[Bold]),
                run(", ", &[]),
                run("underline", &[Underline]),
                run(", ", &[]),
                run("bold italic", &[Italic, Bold]),
                run(", ", &[]),
                run("bold underline", &[Bold, Underline]),
                run(", ", &[]),
                run("italic underline", &[Underline, Italic]),
                run(", and ", &[]),
                run("all", &[Italic, Underline, Bold]),
                run(".", &[]),
            ])),
            action(styled(vec![
                run("under", &[Underline]),
                run("bold", &[Underline, Bold]),
                run("italic", &[Underline, Bold, Italic]),
                run("bold again", &[Underline, Bold]),
                run("under again", &[Underline]),
            ])),
            action(styled(vec![run("before\nafter", &[Italic])])),
            action(styled(vec![
                run("Strike text", &[Strikeout]),
                run(" and ", &[]),
                run("supported bold", &[Strikeout, Bold]),
            ])),
            action_str("Visible /* example */ text."),
        ]);

        assert_eq!(
            output,
            concat!(
                "Literal \\\\ \\* \\_ markers. *italic*, **bold**, _underline_, ***bold italic***, _**bold underline**_, _*italic underline*_, and _***all***_.\n\n",
                "_under**bold*italic*bold again**under again_\n\n",
                "*before*\n",
                "*after*\n\n",
                "Strike text and **supported bold**\n\n",
                "Visible /\\* example \\*/ text.\n",
            )
        );
    }

    #[test]
    fn is_deterministic() {
        let document = doc(
            vec![field("Styled", vec![styled(vec![run("stable", &[Italic, Underline, Bold])])])],
            vec![action(styled(vec![run("repeatable", &[Bold, Italic])]))],
        );
        let expected = "Styled: _***stable***_\n\n***repeatable***\n";

        assert_eq!(screenplay_document_to_fountain(&document), expected);
        assert_eq!(screenplay_document_to_fountain(&document), expected);
    }

    #[test]
    fn returns_the_exact_empty_string_for_an_empty_document() {
        assert_eq!(body(vec![]), "");
    }

    #[test]
    fn forces_title_page_shaped_action_only_at_the_true_document_start() {
        assert_eq!(
            body(vec![action_str("Note: The room is empty."), action_str("He said: run.")]),
            "!Note: The room is empty.\n\nHe said: run.\n"
        );
        assert_eq!(body(vec![action_str("He said: run.")]), "!He said: run.\n");
        assert_eq!(body(vec![action_str("Revision_2-A: Blue")]), "!Revision\\_2-A: Blue\n");
        assert_eq!(body(vec![action_str("He said, quietly: run.")]), "He said, quietly: run.\n");
        assert_eq!(
            body(vec![scene(plain("INT. ROOM - DAY")), action_str("Note: The room is empty.")]),
            "INT. ROOM - DAY\n\nNote: The room is empty.\n"
        );
        assert_eq!(
            screenplay_document_to_fountain(&doc(
                vec![field("Title", vec![plain("Example")])],
                vec![action_str("Note: The room is empty.")],
            )),
            "Title: Example\n\nNote: The room is empty.\n"
        );
    }

    #[test]
    fn preserves_a_scene_heading_that_already_has_a_force_marker() {
        assert_eq!(body(vec![scene(plain(".MEMORY"))]), ".MEMORY\n");
    }

    #[test]
    fn emits_text_with_unrepresentable_fountain_role_syntax_verbatim() {
        assert_eq!(
            body(vec![
                action_str("Open [[TOP SECRET]] now."),
                character(plain("@HANDLE")),
                dialogue("Ready."),
            ]),
            "Open [[TOP SECRET]] now.\n\n@HANDLE\nReady.\n"
        );
    }

    /// Edge cases checked against the TypeScript build: ASCII-only `/i` folding,
    /// orphan dialogue content, line-ending normalization, and Unicode case.
    #[test]
    fn matches_typescript_edge_cases() {
        assert_eq!(body(vec![scene(plain("E\u{17F}T. YARD"))]), ".E\u{17F}T. YARD\n");
        assert_eq!(body(vec![scene(plain("i/e car"))]), "i/e car\n");
        assert_eq!(body(vec![scene(plain("INT"))]), ".INT\n");
        assert_eq!(
            body(vec![dialogue("orphan"), parenthetical(plain("(p)"))]),
            "orphan\n\n(p)\n"
        );
        assert_eq!(body(vec![action_str("a\r\nb\rc")]), "a\nb\nc\n");
        assert_eq!(body(vec![character(plain("STRA\u{DF}E"))]), "@STRA\u{DF}E\n");
        assert_eq!(body(vec![character(plain("123"))]), "@123\n");
        assert_eq!(body(vec![action_str("\u{C9}T\u{C9}: hot")]), "!\u{C9}T\u{C9}: hot\n");
        assert_eq!(body(vec![action_str("x\n\n.scene\n\ny")]), "x\n\n!.scene\n\ny\n");
    }

    /// TypeScript throws a TypeError for this input (more styled lines than
    /// rendered lines); the port deliberately serializes it instead of panicking.
    #[test]
    fn split_carriage_return_line_feed_across_runs_does_not_panic() {
        let fountain = body(vec![action(styled(vec![run("a\r", &[]), run("\nb", &[])]))]);
        assert!(fountain.contains('a') && fountain.contains('b'));
    }
}
