//! Port of `src/core/recognition/recognize-screenplay.ts`.
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::core::layout::coordinate_evidence::infer_coordinate_tolerance;
use crate::core::layout::dialogue_occurrences::{
    DialogueContentOccurrence, DialogueContentType, DialogueOccurrence, DualDialogueOccurrence,
};
use crate::core::layout::line_set::LineSet;
use crate::core::layout::screenplay_layout::ScreenplayLayout;
use crate::core::layout::screenplay_sequences::find_screenplay_sequences;
use crate::core::normalized_text::NormalizedText;
use crate::core::physical_lines::PhysicalTextLine;
use crate::core::positioned_text::PositionedTextStyle;
use crate::core::recognition::action_lines::{
    CollectActionLinesOptions, collect_action_lines, create_action, infer_action_line_measure,
};
use crate::core::recognition::centered_action::find_centered_action_lines;
use crate::core::recognition::line_measure::{VISIBLE_LINE_SHORTFALL_MULTIPLIER, infer_line_measure};
use crate::core::recognition::scene_headings::{create_scene_heading, find_scene_headings};
use crate::core::recognition::sentence_spacing::{SentenceSpacingConvention, infer_sentence_spacing_convention};
use crate::core::recognition::styled_text::{
    LineJoin, LineJoins, create_styled_text, slice_styled_text, styled_text_value,
};
use crate::core::recognition::title_page::extract_title_page;
use crate::core::screenplay_document::{
    Character, DialogueContent, DialogueSequence, ScreenplayDocument, ScreenplayElement,
};
use crate::core::text::{js_len, js_trim_end};

/// JavaScript's `\s` class, spelled out because the `regex` crate's `\s`
/// differs on U+0085 and U+FEFF.
const JS_WHITESPACE_CLASS: &str =
    r"[\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

/// `/\s*\(CONT['’]D\)\s*$/iu`.
static CUE_CONTINUATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i){JS_WHITESPACE_CLASS}*\(CONT['’]D\){JS_WHITESPACE_CLASS}*$")).expect("valid regex")
});
/// `/[.!?…]["'’”)]?\s*$/u`.
static DIALOGUE_SENTENCE_ENDING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r#"[.!?…]["'’”)]?{JS_WHITESPACE_CLASS}*$"#)).expect("valid regex")
});

/// Port of `recognizeScreenplay`.
pub fn recognize_screenplay(normalized_text: &NormalizedText, layout: &ScreenplayLayout) -> ScreenplayDocument {
    let lines: Vec<&PhysicalTextLine> = normalized_text.pages.iter().flat_map(|page| page.lines.iter()).collect();

    if lines.is_empty() {
        return ScreenplayDocument { title_page: Vec::new(), elements: Vec::new() };
    }

    let coordinate_tolerance = infer_coordinate_tolerance(&lines);
    let sentence_spacing_convention = infer_sentence_spacing_convention(&lines);
    let title_page = extract_title_page(&lines, layout, coordinate_tolerance);
    let body_lines = &title_page.body_lines;
    let unattributed_lines: LineSet = title_page.unattributed_lines.iter().copied().collect();
    let recognition_lines: Vec<&PhysicalTextLine> =
        body_lines.iter().copied().filter(|line| !unattributed_lines.contains(line)).collect();
    let sequences = find_screenplay_sequences(&recognition_lines, coordinate_tolerance);
    // `Map` keyed by line identity; later entries overwrite earlier ones, as
    // with `new Map(entries)`. Only looked up, never iterated.
    let dual_dialogue_by_cue: HashMap<*const PhysicalTextLine, &DualDialogueOccurrence> = sequences
        .dual_dialogue_occurrences
        .iter()
        .map(|occurrence| (std::ptr::from_ref(occurrence.left_character_cue), occurrence))
        .collect();
    let dialogue_by_cue: HashMap<*const PhysicalTextLine, &DialogueOccurrence> = sequences
        .dialogue_occurrences
        .iter()
        .map(|occurrence| (std::ptr::from_ref(occurrence.character_cue), occurrence))
        .collect();
    let transition_lines: LineSet = sequences.transition_lines.iter().copied().collect();
    let action_line_measure = infer_action_line_measure(&recognition_lines, layout, coordinate_tolerance);
    let scene_headings = find_scene_headings(
        &recognition_lines,
        &sequences.reserved_lines,
        layout,
        coordinate_tolerance,
        action_line_measure,
    );
    let scene_heading_by_line: HashMap<*const PhysicalTextLine, usize> = scene_headings
        .iter()
        .enumerate()
        .map(|(index, evidence)| (std::ptr::from_ref(evidence.heading), index))
        .collect();
    let scene_number_fragments: LineSet =
        scene_headings.iter().flat_map(|evidence| evidence.number_fragments.iter().copied()).collect();
    let mut consumed_lines: LineSet = LineSet::new();
    let mut elements: Vec<ScreenplayElement> = Vec::new();
    let centered_evidence_blocked_lines: LineSet = title_page
        .unattributed_lines
        .iter()
        .copied()
        .chain(sequences.reserved_lines.iter().filter(|line| !transition_lines.contains(line)))
        .chain(scene_headings.iter().flat_map(|evidence| evidence.heading_lines.iter().copied()))
        .chain(scene_number_fragments.iter())
        .collect();
    let dialogue_line_measure = infer_dialogue_line_measure(&recognition_lines, layout, coordinate_tolerance);
    let centered_action_lines =
        find_centered_action_lines(body_lines, &centered_evidence_blocked_lines, layout, coordinate_tolerance);
    let centered_transition_lines: LineSet = sequences
        .transition_lines
        .iter()
        .copied()
        .filter(|line| centered_action_lines.contains(line))
        .collect();
    let blocked_action_lines: LineSet = title_page
        .unattributed_lines
        .iter()
        .copied()
        .chain(sequences.reserved_lines.iter().filter(|line| !centered_transition_lines.contains(line)))
        .chain(scene_headings.iter().flat_map(|evidence| evidence.heading_lines.iter().copied()))
        .chain(scene_number_fragments.iter())
        .collect();

    let mut index = 0;
    while index < body_lines.len() {
        let line = body_lines[index];
        let key = std::ptr::from_ref(line);

        if consumed_lines.contains(line) || scene_number_fragments.contains(line) {
            index += 1;
            continue;
        }

        if let Some(dual_dialogue) = dual_dialogue_by_cue.get(&key) {
            elements.push(create_dual_dialogue(dual_dialogue, sentence_spacing_convention));
            consumed_lines.extend(dual_dialogue.lines.iter().copied());
            index += 1;
            continue;
        }

        if let Some(dialogue) = dialogue_by_cue.get(&key) {
            elements.extend(create_ordinary_dialogue(dialogue, sentence_spacing_convention, dialogue_line_measure));
            consumed_lines.insert(dialogue.character_cue);
            consumed_lines.extend(dialogue.content.iter().flat_map(|content| content.lines.iter().copied()));
            index += 1;
            continue;
        }

        if let Some(&heading_index) = scene_heading_by_line.get(&key) {
            let scene_heading = &scene_headings[heading_index];
            elements.push(create_scene_heading(scene_heading));
            consumed_lines.extend(scene_heading.heading_lines.iter().copied());
            consumed_lines.extend(scene_heading.number_fragments.iter().copied());
            index += 1;
            continue;
        }

        if transition_lines.contains(line) && !centered_transition_lines.contains(line) {
            elements.push(create_transition(line));
            consumed_lines.insert(line);
            index += 1;
            continue;
        }

        let action_group = collect_action_lines(&CollectActionLinesOptions {
            lines: body_lines,
            start_index: index,
            blocked_lines: &blocked_action_lines,
            layout,
            coordinate_tolerance,
            action_line_measure,
            centered_action_lines: &centered_action_lines,
        });
        elements.push(create_action(
            &action_group.lines,
            &action_group.line_joins,
            &centered_action_lines,
            sentence_spacing_convention,
        ));
        consumed_lines.extend(action_group.lines.iter().copied());
        // TypeScript sets `index = nextIndex - 1` and the loop increments it.
        index = action_group.next_index;
    }

    ScreenplayDocument { title_page: title_page.fields, elements }
}

fn create_ordinary_dialogue(
    occurrence: &DialogueOccurrence<'_>,
    sentence_spacing_convention: SentenceSpacingConvention,
    dialogue_line_measure: Option<f64>,
) -> Vec<ScreenplayElement> {
    let character = create_character(occurrence.character_cue);

    std::iter::once(ScreenplayElement::Character { text: character.text })
        .chain(occurrence.content.iter().map(|content| {
            match create_dialogue_content(content, sentence_spacing_convention, dialogue_line_measure) {
                DialogueContent::Parenthetical { text } => ScreenplayElement::Parenthetical { text },
                DialogueContent::Dialogue { text } => ScreenplayElement::Dialogue { text },
            }
        }))
        .collect()
}

fn create_dual_dialogue(
    occurrence: &DualDialogueOccurrence<'_>,
    sentence_spacing_convention: SentenceSpacingConvention,
) -> ScreenplayElement {
    ScreenplayElement::DualDialogue {
        left: create_dialogue_sequence(
            occurrence.left_character_cue,
            &occurrence.left_content,
            sentence_spacing_convention,
        ),
        right: create_dialogue_sequence(
            occurrence.right_character_cue,
            &occurrence.right_content,
            sentence_spacing_convention,
        ),
    }
}

fn create_dialogue_sequence(
    character_cue: &PhysicalTextLine,
    content: &[DialogueContentOccurrence<'_>],
    sentence_spacing_convention: SentenceSpacingConvention,
) -> DialogueSequence {
    DialogueSequence {
        character: create_character(character_cue),
        content: content.iter().map(|item| create_dialogue_content(item, sentence_spacing_convention, None)).collect(),
    }
}

/// Port of `createDialogueContent`; `dialogue_line_measure` defaults to
/// `null` in TypeScript, passed here as `None`.
fn create_dialogue_content(
    occurrence: &DialogueContentOccurrence<'_>,
    sentence_spacing_convention: SentenceSpacingConvention,
    dialogue_line_measure: Option<f64>,
) -> DialogueContent {
    match occurrence.content_type {
        DialogueContentType::Parenthetical => DialogueContent::Parenthetical {
            text: create_styled_text(&occurrence.lines, LineJoins::Uniform(LineJoin::Wrap), sentence_spacing_convention),
        },
        DialogueContentType::Dialogue => DialogueContent::Dialogue {
            text: create_styled_text(
                &occurrence.lines,
                LineJoins::PerBoundary(&create_dialogue_line_joins(&occurrence.lines, dialogue_line_measure)),
                sentence_spacing_convention,
            ),
        },
    }
}

fn infer_dialogue_line_measure(
    lines: &[&PhysicalTextLine],
    layout: &ScreenplayLayout,
    coordinate_tolerance: f64,
) -> Option<f64> {
    let dialogue = layout.dialogue?;

    infer_line_measure(lines, dialogue.x, coordinate_tolerance)
}

fn create_dialogue_line_joins(lines: &[&PhysicalTextLine], dialogue_line_measure: Option<f64>) -> Vec<LineJoin> {
    let Some(dialogue_line_measure) = dialogue_line_measure else {
        return Vec::new();
    };

    let mut line_joins: Vec<LineJoin> = Vec::new();

    for pair in lines.windows(2) {
        let (previous_line, line) = (pair[0], pair[1]);
        let visible_shortfall =
            previous_line.bounds.height.min(line.bounds.height) * VISIBLE_LINE_SHORTFALL_MULTIPLIER;
        let previous_line_is_short = previous_line.bounds.width + visible_shortfall < dialogue_line_measure;
        let line_is_short = line.bounds.width + visible_shortfall < dialogue_line_measure;
        let previous_line_ends_sentence = DIALOGUE_SENTENCE_ENDING.is_match(&previous_line.text);
        let continues_established_lineation =
            line_joins.last() == Some(&LineJoin::Newline) && previous_line_ends_sentence;
        let preserves_deliberate_break = previous_line.page_index == line.page_index
            && has_emphasized_style(previous_line)
            && have_matching_line_styles(previous_line, line)
            && ((previous_line_is_short && (line_is_short || previous_line_ends_sentence))
                || continues_established_lineation);

        line_joins.push(if preserves_deliberate_break { LineJoin::Newline } else { LineJoin::Wrap });
    }

    line_joins
}

fn has_emphasized_style(line: &PhysicalTextLine) -> bool {
    line.spans
        .iter()
        .any(|span| span.style.bold || span.style.italic || span.style.underline || span.style.strikeout)
}

fn have_matching_line_styles(left: &PhysicalTextLine, right: &PhysicalTextLine) -> bool {
    let (Some(left_span), Some(right_span)) = (left.spans.first(), right.spans.first()) else {
        return false;
    };
    let (left_style, right_style) = (&left_span.style, &right_span.style);

    left.spans.iter().all(|span| have_matching_styles(&span.style, left_style))
        && right.spans.iter().all(|span| have_matching_styles(&span.style, right_style))
        && have_matching_styles(left_style, right_style)
}

fn have_matching_styles(left: &PositionedTextStyle, right: &PositionedTextStyle) -> bool {
    left.bold == right.bold
        && left.italic == right.italic
        && left.underline == right.underline
        && left.strikeout == right.strikeout
}

fn create_character(line: &PhysicalTextLine) -> Character {
    let text = create_styled_text(&[line], LineJoins::Uniform(LineJoin::Wrap), None);
    let value = styled_text_value(&text);

    let Some(continuation) = CUE_CONTINUATION.find(&value) else {
        return Character { text };
    };

    let prefix_length = js_len(js_trim_end(&value[..continuation.start()]));
    Character { text: slice_styled_text(&text, prefix_length) }
}

fn create_transition(line: &PhysicalTextLine) -> ScreenplayElement {
    ScreenplayElement::Transition { text: create_styled_text(&[line], LineJoins::Uniform(LineJoin::Wrap), None) }
}

#[cfg(test)]
mod tests {
    //! Translated from `test/unit/core/screenplay-centered-action.test.ts`.
    use super::*;
    use crate::core::layout::screenplay_layout::{HorizontalAlignment, LayoutAnchor};
    use crate::core::normalized_text::NormalizedTextPage;
    use crate::core::positioned_text::{PositionedTextBounds, PositionedTextFont, SourceMethod};
    use crate::core::physical_lines::PhysicalTextSpan;
    use crate::core::screenplay_document::{ActionAlignment, StyledText, TextRun};

    fn anchor(alignment: HorizontalAlignment, x: f64) -> Option<LayoutAnchor> {
        Some(LayoutAnchor { alignment, x })
    }

    fn established_layout() -> ScreenplayLayout {
        ScreenplayLayout {
            action: anchor(HorizontalAlignment::Left, 100.0),
            character_cue: anchor(HorizontalAlignment::Left, 240.0),
            dialogue: anchor(HorizontalAlignment::Left, 176.0),
            parenthetical: anchor(HorizontalAlignment::Left, 210.0),
            transition: anchor(HorizontalAlignment::Right, 406.0),
            dual_dialogue: None,
            diagnostics: Vec::new(),
        }
    }

    fn null_layout() -> ScreenplayLayout {
        ScreenplayLayout {
            action: None,
            character_cue: None,
            dialogue: None,
            parenthetical: None,
            transition: None,
            dual_dialogue: None,
            diagnostics: Vec::new(),
        }
    }

    fn normalized_text(lines: &[(&str, f64, f64, f64)]) -> NormalizedText {
        let lines = lines
            .iter()
            .enumerate()
            .map(|(source_index, &(text, x, y, width))| {
                let bounds = PositionedTextBounds { x, y, width, height: 12.0 };
                PhysicalTextLine {
                    page_index: 0,
                    text: text.to_owned(),
                    bounds,
                    indentation: None,
                    spans: vec![PhysicalTextSpan {
                        start: 0,
                        end: text.len(),
                        source_index,
                        source_method: SourceMethod::EmbeddedText,
                        bounds,
                        font: PositionedTextFont { name: "Courier Prime".to_owned(), size: 12.0 },
                        style: PositionedTextStyle::default(),
                    }],
                }
            })
            .collect();

        NormalizedText {
            pages: vec![NormalizedTextPage { page_index: 0, lines }],
            suppressed: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn plain(text: &str) -> StyledText {
        StyledText { runs: vec![TextRun { text: text.to_owned(), styles: Vec::new() }] }
    }

    fn action(text: &str) -> ScreenplayElement {
        ScreenplayElement::Action { text: plain(text), alignment: ActionAlignment::Standard }
    }

    fn centered(text: &str) -> ScreenplayElement {
        ScreenplayElement::Action { text: plain(text), alignment: ActionAlignment::Center }
    }

    fn document(elements: Vec<ScreenplayElement>) -> ScreenplayDocument {
        ScreenplayDocument { title_page: Vec::new(), elements }
    }

    #[test]
    fn does_not_center_fallback_text_at_established_screenplay_role_geometry() {
        let input = normalized_text(&[
            ("orphan cue text", 240.0, 700.0, 116.0),
            ("orphan dialogue fragment", 176.0, 676.0, 244.0),
            ("orphan parenthetical fragment", 210.0, 652.0, 176.0),
            ("orphan transition fragment", 190.0, 628.0, 216.0),
            ("CENTERED SINGLE", 268.0, 604.0, 60.0),
            ("CENTERED FIRST", 280.0, 580.0, 100.0),
            ("CENTERED SECOND", 294.0, 568.0, 72.0),
        ]);

        assert_eq!(
            recognize_screenplay(&input, &established_layout()),
            document(vec![
                action("orphan cue text"),
                action("orphan dialogue fragment"),
                action("orphan parenthetical fragment"),
                action("orphan transition fragment"),
                centered("CENTERED SINGLE"),
                centered("CENTERED FIRST\nCENTERED SECOND"),
            ])
        );
    }

    #[test]
    fn keeps_every_retained_line_as_standard_action_when_the_action_layout_is_null() {
        let input = normalized_text(&[
            ("unattributable first", 268.0, 700.0, 60.0),
            ("unattributable second", 280.0, 676.0, 100.0),
        ]);

        assert_eq!(
            recognize_screenplay(&input, &null_layout()),
            document(vec![action("unattributable first"), action("unattributable second")])
        );
    }

    #[test]
    fn centers_context_supported_display_lines_at_a_stable_exporter_center() {
        let input = normalized_text(&[
            ("TITLE OVER:", 100.0, 700.0, 79.0),
            ("BIG FISH", 288.0, 676.0, 56.0),
            ("INT. HOSPITAL - DAY", 100.0, 640.0, 144.0),
            ("CUT TO BLACK.", 326.0, 604.0, 94.0),
            ("THE END", 291.5, 568.0, 49.0),
        ]);

        assert_eq!(
            recognize_screenplay(&input, &established_layout()),
            document(vec![
                action("TITLE OVER:"),
                centered("BIG FISH"),
                ScreenplayElement::SceneHeading { text: plain("INT. HOSPITAL - DAY"), scene_number: None },
                ScreenplayElement::Transition { text: plain("CUT TO BLACK.") },
                centered("THE END"),
            ])
        );
    }

    #[test]
    fn keeps_repeated_isolated_inset_action_standard_away_from_the_screenplay_center() {
        let input = normalized_text(&[
            ("first inset action", 330.0, 700.0, 100.0),
            ("second inset action", 340.0, 640.0, 80.0),
        ]);

        assert_eq!(
            recognize_screenplay(&input, &established_layout()),
            document(vec![action("first inset action"), action("second inset action")])
        );
    }

    #[test]
    fn strips_cue_continuations_by_utf16_length() {
        let input = normalized_text(&[("B\u{1F3AC}B  (cont\u{2019}d) ", 240.0, 700.0, 100.0)]);
        let character = create_character(&input.pages[0].lines[0]);

        assert_eq!(character.text, plain("B\u{1F3AC}B"));
    }

    #[test]
    fn returns_an_empty_document_without_lines() {
        assert_eq!(recognize_screenplay(&NormalizedText::default(), &null_layout()), document(Vec::new()));
    }
}

#[cfg(test)]
mod text_formatted_tests {
    //! Mirrors the "text-formatted and shared-page screenplays" cases in
    //! `test/unit/core/screenplay-recognition-boundaries.test.ts`.
    use serde_json::json;

    use super::*;
    use crate::core::screenplay_document::{ActionAlignment, ScreenplayElement, StyledText, TextRun, TextStyle, TitlePageField};

    /// A page of lines as `(text, x, y, bold)`, built like the TypeScript
    /// `createPhysicalLine` helper (7.2 per character, Courier Prime 12).
    fn normalized_text(pages: &[&[(&str, f64, f64, bool)]]) -> NormalizedText {
        let mut source_index = 0;
        let pages: Vec<_> = pages
            .iter()
            .enumerate()
            .map(|(page_index, lines)| {
                let lines: Vec<_> = lines
                    .iter()
                    .map(|&(text, x, y, bold)| {
                        let bounds = json!({ "x": x, "y": y, "width": crate::core::text::js_len(text) as f64 * 7.2, "height": 12.0 });
                        source_index += 1;
                        json!({
                            "pageIndex": page_index, "text": text, "bounds": bounds,
                            "spans": [{
                                "start": 0, "end": text.len(), "sourceIndex": source_index - 1,
                                "sourceMethod": "embedded-text", "bounds": bounds,
                                "font": { "name": "Courier Prime", "size": 12.0 },
                                "style": { "bold": bold, "italic": false, "underline": false, "strikeout": false },
                            }],
                        })
                    })
                    .collect();
                json!({ "pageIndex": page_index, "lines": lines })
            })
            .collect();
        serde_json::from_value(json!({ "pages": pages, "suppressed": [], "diagnostics": [] })).unwrap()
    }

    fn layout() -> ScreenplayLayout {
        serde_json::from_value(json!({
            "action": { "alignment": "left", "x": 108.0 },
            "characterCue": { "alignment": "left", "x": 252.0 },
            "dialogue": { "alignment": "left", "x": 180.0 },
            "parenthetical": { "alignment": "left", "x": 216.0 },
            "transition": { "alignment": "right", "x": 540.0 },
            "dualDialogue": null,
            "diagnostics": [],
        }))
        .unwrap()
    }

    fn plain(text: &str) -> StyledText {
        StyledText { runs: vec![TextRun { text: text.to_owned(), styles: Vec::new() }] }
    }

    fn field(key: &str, values: &[&str]) -> TitlePageField {
        TitlePageField { key: key.to_owned(), values: values.iter().map(|value| plain(value)).collect() }
    }

    fn action(text: &str) -> ScreenplayElement {
        ScreenplayElement::Action { text: plain(text), alignment: ActionAlignment::Standard }
    }

    /// Centered at x = 306.
    fn centered(text: &str, y: f64) -> (&str, f64, f64, bool) {
        (text, 306.0 - text.len() as f64 * 7.2 / 2.0, y, false)
    }

    #[test]
    fn separates_a_title_block_that_shares_the_first_page_with_the_script() {
        let input = normalized_text(&[&[
            centered("TITLE", 700.0),
            centered("by", 676.0),
            centered("Writer Name", 652.0),
            ("INT. ROOM - DAY", 108.0, 580.0, true),
            ("A door opens.", 108.0, 556.0, false),
        ]]);
        let document = recognize_screenplay(&input, &layout());

        assert_eq!(
            document.title_page,
            [field("Title", &["TITLE"]), field("Credit", &["by"]), field("Author", &["Writer Name"])]
        );
        assert_eq!(
            document.elements,
            [
                ScreenplayElement::SceneHeading {
                    text: StyledText {
                        runs: vec![TextRun { text: "INT. ROOM - DAY".to_owned(), styles: vec![TextStyle::Bold] }]
                    },
                    scene_number: None,
                },
                action("A door opens."),
            ]
        );
    }

    #[test]
    fn keeps_title_page_lines_no_field_claims_as_notes() {
        let input = normalized_text(&[
            &[
                centered("TITLE", 700.0),
                centered("by", 676.0),
                centered("Writer Name", 652.0),
                ("FIRST DRAFT", 360.0, 120.0, false),
                ("March 1, 2026", 360.0, 96.0, false),
            ],
            &[("INT. ROOM - DAY", 108.0, 700.0, false)],
        ]);

        assert_eq!(
            recognize_screenplay(&input, &layout()).title_page,
            [
                field("Title", &["TITLE"]),
                field("Credit", &["by"]),
                field("Author", &["Writer Name"]),
                field("Notes", &["FIRST DRAFT", "March 1, 2026"]),
            ]
        );
    }

    #[test]
    fn recognizes_a_title_block_centered_with_spaces_in_a_proportional_font() {
        // Centered by character count (center column 20.5), so not geometrically.
        let mut input = normalized_text(&[
            &[
                ("TITLE", 250.0, 700.0, false),
                ("Screenplay by", 230.0, 676.0, false),
                ("Writer Name", 245.0, 652.0, false),
            ],
            &[("INT. ROOM - DAY", 108.0, 700.0, false)],
        ]);
        for (line, indentation) in input.pages[0].lines.iter_mut().zip([18, 14, 15]) {
            line.indentation = Some(indentation);
        }

        assert_eq!(
            recognize_screenplay(&input, &layout()).title_page,
            [field("Title", &["TITLE"]), field("Credit", &["Screenplay by"]), field("Author", &["Writer Name"])]
        );
    }

    #[test]
    fn treats_line_end_padding_as_a_single_word_space_and_trims_element_edges() {
        let input = normalized_text(&[&[("He waits for  ", 108.0, 700.0, false), ("the train.   ", 108.0, 688.0, false)]]);

        assert_eq!(recognize_screenplay(&input, &layout()).elements, [action("He waits for the train.")]);
    }
}
