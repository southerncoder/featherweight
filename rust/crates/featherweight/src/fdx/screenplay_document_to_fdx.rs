//! Port of `src/fdx/screenplay-document-to-fdx.ts`.
use crate::core::screenplay_document::{
    ActionAlignment, DialogueContent, DialogueSequence, ScreenplayDocument, ScreenplayElement, StyledText, TextRun,
    TextStyle, TitlePageField,
};
use crate::core::text::js_trim;
use crate::error::ScreenplayConversionError;

use super::styled_text_to_fdx::{
    assert_valid_fdx_text, assert_valid_styled_text, escape_xml_attribute, plain_styled_text, split_styled_text_lines,
    styled_text_to_fdx,
};

/// `ParagraphAttributes`.
#[derive(Default)]
struct ParagraphAttributes<'a> {
    center: bool,
    number: Option<&'a str>,
    starts_new_page: bool,
}

/// `TitleParagraph`. The alignment is the raw `titleAlignments` lookup result
/// rendered as a string (see [`title_alignment`]).
struct TitleParagraph {
    alignment: Option<&'static str>,
    text: StyledText,
}

/// `titleAlignments[key.trim().toLowerCase()]`.
///
/// `titleAlignments` is a plain object literal, so the lookup also reaches
/// `Object.prototype`: the only all-lowercase inherited keys are `constructor`
/// and `__proto__`, whose values stringify in the `Alignment` attribute as
/// below. This reproduces the TypeScript output exactly.
fn title_alignment(key: &str) -> Option<&'static str> {
    match js_trim(key).to_lowercase().as_str() {
        "title" | "credit" | "author" | "authors" | "source" => Some("Center"),
        "draft date" | "contact" | "copyright" => Some("Left"),
        "notes" => Some("Right"),
        "constructor" => Some("function Object() { [native code] }"),
        "__proto__" => Some("[object Object]"),
        _ => None,
    }
}

/// Port of `screenplayDocumentToFDX`: serializes a document to Final Draft
/// XML, or fails with [`ScreenplayConversionError::InvalidFdxText`] when any
/// text contains a code point XML cannot represent.
pub fn screenplay_document_to_fdx(document: &ScreenplayDocument) -> Result<String, ScreenplayConversionError> {
    validate_document(document)?;

    let content = serialize_body(&document.elements);
    let title_page = serialize_title_page(&document.title_page);

    let mut lines = vec![
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>".to_owned(),
        "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">".to_owned(),
        format!("  <Content>{content}</Content>"),
    ];
    if let Some(title_page) = title_page {
        lines.push(format!("  <TitlePage><Content>{title_page}</Content></TitlePage>"));
    }
    lines.push("</FinalDraft>".to_owned());

    Ok(lines.join("\n"))
}

fn validate_document(document: &ScreenplayDocument) -> Result<(), ScreenplayConversionError> {
    for field in &document.title_page {
        assert_valid_fdx_text(&field.key)?;

        for value in &field.values {
            assert_valid_styled_text(value)?;
        }
    }

    document.elements.iter().try_for_each(validate_element)
}

fn validate_element(element: &ScreenplayElement) -> Result<(), ScreenplayConversionError> {
    match element {
        ScreenplayElement::PageBreak => Ok(()),
        ScreenplayElement::DualDialogue { left, right, .. } => {
            validate_dialogue_sequence(left)?;
            validate_dialogue_sequence(right)
        }
        ScreenplayElement::SceneHeading { text, scene_number, .. } => {
            assert_valid_styled_text(text)?;
            match scene_number {
                Some(number) => assert_valid_fdx_text(number),
                None => Ok(()),
            }
        }
        ScreenplayElement::Action { text, .. }
        | ScreenplayElement::Character { text, .. }
        | ScreenplayElement::Parenthetical { text, .. }
        | ScreenplayElement::Dialogue { text, .. }
        | ScreenplayElement::Lyric { text, .. }
        | ScreenplayElement::Transition { text, .. } => assert_valid_styled_text(text),
    }
}

fn validate_dialogue_sequence(sequence: &DialogueSequence) -> Result<(), ScreenplayConversionError> {
    assert_valid_styled_text(&sequence.character.text)?;

    sequence
        .content
        .iter()
        .try_for_each(|content| assert_valid_styled_text(dialogue_content_text(content)))
}

fn dialogue_content_text(content: &DialogueContent) -> &StyledText {
    match content {
        DialogueContent::Parenthetical { text } | DialogueContent::Dialogue { text } => text,
    }
}

fn serialize_body(elements: &[ScreenplayElement]) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut pending_page_breaks = 0usize;

    for element in elements {
        if matches!(element, ScreenplayElement::PageBreak) {
            pending_page_breaks += 1;
            continue;
        }

        for _ in 1..pending_page_breaks {
            paragraphs.push(serialize_empty_page_break());
        }

        paragraphs.push(serialize_element(element, pending_page_breaks > 0));
        pending_page_breaks = 0;
    }

    for _ in 0..pending_page_breaks {
        paragraphs.push(serialize_empty_page_break());
    }

    paragraphs.concat()
}

/// `serializeElement`; never called with a page break.
fn serialize_element(element: &ScreenplayElement, starts_new_page: bool) -> String {
    let attributes = ParagraphAttributes {
        starts_new_page,
        ..Default::default()
    };

    match element {
        ScreenplayElement::SceneHeading { text, scene_number, .. } => serialize_typed_paragraph(
            "Scene Heading",
            text,
            &ParagraphAttributes {
                number: scene_number.as_deref(),
                ..attributes
            },
            &[],
        ),
        ScreenplayElement::Action { text, alignment, .. } => serialize_typed_paragraph(
            "Action",
            text,
            &ParagraphAttributes {
                center: *alignment == ActionAlignment::Center,
                ..attributes
            },
            &[],
        ),
        ScreenplayElement::Character { text, .. } => serialize_typed_paragraph("Character", text, &attributes, &[]),
        ScreenplayElement::Parenthetical { text, .. } => {
            serialize_typed_paragraph("Parenthetical", text, &attributes, &[])
        }
        ScreenplayElement::Dialogue { text, .. } => serialize_typed_paragraph("Dialogue", text, &attributes, &[]),
        ScreenplayElement::DualDialogue { left, right, .. } => serialize_dual_dialogue(left, right, starts_new_page),
        // Lyrics become italic Action paragraphs (`serializeLyric`).
        ScreenplayElement::Lyric { text, .. } => {
            serialize_typed_paragraph("Action", text, &attributes, &[TextStyle::Italic])
        }
        ScreenplayElement::Transition { text, .. } => serialize_typed_paragraph("Transition", text, &attributes, &[]),
        // `serialize_body` turns page breaks into the next paragraph's
        // `StartsNewPage`; one reaching here has no paragraph of its own.
        ScreenplayElement::PageBreak => String::new(),
    }
}

fn serialize_dual_dialogue(left: &DialogueSequence, right: &DialogueSequence, starts_new_page: bool) -> String {
    let attributes = serialize_paragraph_attributes(&ParagraphAttributes {
        starts_new_page,
        ..Default::default()
    });
    let dialogue = serialize_dialogue_sequence(left) + &serialize_dialogue_sequence(right);

    format!("<Paragraph{attributes}><DualDialogue>{dialogue}</DualDialogue></Paragraph>")
}

fn serialize_dialogue_sequence(sequence: &DialogueSequence) -> String {
    let mut output = serialize_typed_paragraph(
        "Character",
        &sequence.character.text,
        &ParagraphAttributes::default(),
        &[],
    );
    for content in &sequence.content {
        output.push_str(&serialize_dialogue_content(content));
    }
    output
}

fn serialize_dialogue_content(content: &DialogueContent) -> String {
    let paragraph_type = match content {
        DialogueContent::Parenthetical { .. } => "Parenthetical",
        DialogueContent::Dialogue { .. } => "Dialogue",
    };

    serialize_typed_paragraph(
        paragraph_type,
        dialogue_content_text(content),
        &ParagraphAttributes::default(),
        &[],
    )
}

fn serialize_typed_paragraph(
    paragraph_type: &str,
    text: &StyledText,
    attributes: &ParagraphAttributes,
    added_styles: &[TextStyle],
) -> String {
    let paragraph_attributes = serialize_paragraph_attributes(attributes);

    format!(
        "<Paragraph Type=\"{paragraph_type}\"{paragraph_attributes}>{}</Paragraph>",
        styled_text_to_fdx(text, added_styles)
    )
}

fn serialize_paragraph_attributes(attributes: &ParagraphAttributes) -> String {
    let mut serialized: Vec<String> = Vec::new();

    if attributes.center {
        serialized.push("Alignment=\"Center\"".to_owned());
    }

    if let Some(number) = attributes.number {
        serialized.push(format!("Number=\"{}\"", escape_xml_attribute(number)));
    }

    if attributes.starts_new_page {
        serialized.push("StartsNewPage=\"Yes\"".to_owned());
    }

    if serialized.is_empty() {
        String::new()
    } else {
        format!(" {}", serialized.join(" "))
    }
}

fn serialize_empty_page_break() -> String {
    "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"></Paragraph>".to_owned()
}

fn serialize_title_page(fields: &[TitlePageField]) -> Option<String> {
    if fields.is_empty() {
        return None;
    }

    Some(
        fields
            .iter()
            .flat_map(title_field_to_paragraphs)
            .map(|p| serialize_title_paragraph(&p))
            .collect(),
    )
}

fn title_field_to_paragraphs(field: &TitlePageField) -> Vec<TitleParagraph> {
    let alignment = title_alignment(&field.key);

    if field.values.is_empty() {
        return vec![TitleParagraph {
            alignment,
            text: plain_styled_text(&format!("{}:", field.key)),
        }];
    }

    if alignment.is_some() {
        return field
            .values
            .iter()
            .flat_map(split_styled_text_lines)
            .map(|text| TitleParagraph { alignment, text })
            .collect();
    }

    let value_lines: Vec<Vec<StyledText>> = field.values.iter().map(split_styled_text_lines).collect();
    let uses_inline_layout = value_lines.len() == 1 && value_lines[0].len() == 1;

    if uses_inline_layout {
        let mut runs = vec![TextRun {
            text: format!("{}: ", field.key),
            styles: Vec::new(),
        }];
        runs.extend(value_lines[0][0].runs.iter().cloned());
        return vec![TitleParagraph {
            alignment: None,
            text: StyledText { runs },
        }];
    }

    let mut paragraphs = vec![TitleParagraph {
        alignment: None,
        text: plain_styled_text(&format!("{}:", field.key)),
    }];
    paragraphs.extend(
        value_lines
            .into_iter()
            .flatten()
            .map(|text| TitleParagraph { alignment: None, text }),
    );
    paragraphs
}

fn serialize_title_paragraph(paragraph: &TitleParagraph) -> String {
    let alignment = paragraph
        .alignment
        .map(|alignment| format!(" Alignment=\"{alignment}\""))
        .unwrap_or_default();

    format!(
        "<Paragraph{alignment}>{}</Paragraph>",
        styled_text_to_fdx(&paragraph.text, &[])
    )
}

/// Expected strings are the raw output of the TypeScript `screenplayDocumentToFDX`
/// (dist build) for the documents of `test/unit/fdx/screenplay-document-to-fdx.test.ts`,
/// plus extra edge cases.
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> ScreenplayDocument {
        serde_json::from_str(json).expect("valid test document")
    }

    #[test]
    fn maps_every_direct_model_variant_and_positional_title_field() {
        let document = parse(
            r#"{"titlePage":[{"key":" title ","values":[{"runs":[{"text":"FIRST\nSECOND","styles":["bold"]}]}]},{"key":"Credit","values":[{"runs":[{"text":"Written by","styles":[]}]}]},{"key":"Authors","values":[{"runs":[{"text":"Ada & Bea","styles":[]}]}]},{"key":"Source","values":[{"runs":[{"text":"A novel","styles":[]}]}]},{"key":"Draft date","values":[{"runs":[{"text":"2026-08-05","styles":[]}]}]},{"key":"Contact","values":[{"runs":[{"text":"mail@example.test","styles":[]}]}]},{"key":"Copyright","values":[{"runs":[{"text":"Copyright 2026","styles":[]}]}]},{"key":"Notes","values":[{"runs":[{"text":"Private note","styles":[]}]}]},{"key":"Custom","values":[{"runs":[{"text":"one line","styles":[]}]}]},{"key":"Gallery","values":[{"runs":[{"text":"first","styles":[]}]},{"runs":[{"text":"second\n\nthird","styles":[]}]}]},{"key":"Empty","values":[]},{"key":"Author","values":[]}],"elements":[{"type":"scene-heading","text":{"runs":[{"text":"INT. LAB & CELLAR - NIGHT","styles":[]}]},"sceneNumber":"A&\"7\""},{"type":"scene-heading","text":{"runs":[{"text":"MEMORY","styles":[]}]},"sceneNumber":null},{"type":"action","text":{"runs":[{"text":"Standard action.","styles":[]}]},"alignment":"standard"},{"type":"action","text":{"runs":[{"text":"Centered action.","styles":[]}]},"alignment":"center"},{"type":"character","text":{"runs":[{"text":"ADA","styles":[]}]}},{"type":"parenthetical","text":{"runs":[{"text":"(quietly)","styles":[]}]}},{"type":"dialogue","text":{"runs":[{"text":"Exact words.","styles":[]}]}},{"type":"transition","text":{"runs":[{"text":"CUT TO:","styles":[]}]}},{"type":"lyric","text":{"runs":[{"text":"Sing ","styles":[]},{"text":"bright","styles":["bold","underline"]}]}}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Scene Heading\" Number=\"A&amp;&quot;7&quot;\"><Text>INT. LAB &amp; CELLAR - NIGHT</Text></Paragraph>",
                "<Paragraph Type=\"Scene Heading\"><Text>MEMORY</Text></Paragraph>",
                "<Paragraph Type=\"Action\"><Text>Standard action.</Text></Paragraph>",
                "<Paragraph Type=\"Action\" Alignment=\"Center\"><Text>Centered action.</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>ADA</Text></Paragraph>",
                "<Paragraph Type=\"Parenthetical\"><Text>(quietly)</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Exact words.</Text></Paragraph>",
                "<Paragraph Type=\"Transition\"><Text>CUT TO:</Text></Paragraph>",
                "<Paragraph Type=\"Action\"><Text Style=\"Italic\">Sing </Text><Text Style=\"Bold+Italic+Underline\">bright</Text></Paragraph>",
                "</Content>\n",
                "  <TitlePage><Content><Paragraph Alignment=\"Center\"><Text Style=\"Bold\">FIRST</Text></Paragraph>",
                "<Paragraph Alignment=\"Center\"><Text Style=\"Bold\">SECOND</Text></Paragraph>",
                "<Paragraph Alignment=\"Center\"><Text>Written by</Text></Paragraph>",
                "<Paragraph Alignment=\"Center\"><Text>Ada &amp; Bea</Text></Paragraph>",
                "<Paragraph Alignment=\"Center\"><Text>A novel</Text></Paragraph>",
                "<Paragraph Alignment=\"Left\"><Text>2026-08-05</Text></Paragraph>",
                "<Paragraph Alignment=\"Left\"><Text>mail@example.test</Text></Paragraph>",
                "<Paragraph Alignment=\"Left\"><Text>Copyright 2026</Text></Paragraph>",
                "<Paragraph Alignment=\"Right\"><Text>Private note</Text></Paragraph>",
                "<Paragraph><Text>Custom: </Text><Text>one line</Text></Paragraph>",
                "<Paragraph><Text>Gallery:</Text></Paragraph>",
                "<Paragraph><Text>first</Text></Paragraph>",
                "<Paragraph><Text>second</Text></Paragraph>",
                "<Paragraph></Paragraph>",
                "<Paragraph><Text>third</Text></Paragraph>",
                "<Paragraph><Text>Empty:</Text></Paragraph>",
                "<Paragraph Alignment=\"Center\"><Text>Author:</Text></Paragraph>",
                "</Content></TitlePage>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn preserves_run_boundaries_styles_xml_characters_unicode_and_multiline_text() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"scene-heading","text":{"runs":[{"text":"INT. <LAB> & CAFÉ — NIGHT","styles":[]}]},"sceneNumber":"A<&\"2\""},{"type":"action","text":{"runs":[{"text":"plain ","styles":[]},{"text":"bold","styles":["bold"]},{"text":" italic","styles":["italic"]},{"text":" underline","styles":["underline"]},{"text":" strike","styles":["strikeout"]},{"text":" all","styles":["strikeout","underline","italic","bold"]},{"text":" together","styles":[]}]},"alignment":"standard"},{"type":"action","text":{"runs":[{"text":"first\r\nsecond\rthird","styles":[]}]},"alignment":"standard"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Scene Heading\" Number=\"A&lt;&amp;&quot;2&quot;\"><Text>INT. &lt;LAB&gt; &amp; CAF\u{C9} \u{2014} NIGHT</Text></Paragraph>",
                "<Paragraph Type=\"Action\"><Text>plain </Text><Text Style=\"Bold\">bold</Text><Text Style=\"Italic\"> italic</Text><Text Style=\"Underline\"> underline</Text><Text Style=\"Strikeout\"> strike</Text><Text Style=\"Bold+Italic+Underline+Strikeout\"> all</Text><Text> together</Text></Paragraph>",
                "<Paragraph Type=\"Action\"><Text>first\n",
                "second\n",
                "third</Text></Paragraph>",
                "</Content>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn rejects_control_character_in_action() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"action","text":{"runs":[{"text":"before\u0001after","styles":[]}]},"alignment":"standard"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document),
            Err(ScreenplayConversionError::InvalidFdxText)
        );
    }

    #[test]
    fn rejects_control_character_in_scene_number() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"scene-heading","text":{"runs":[{"text":"INT. LAB - NIGHT","styles":[]}]},"sceneNumber":"before\u0001after"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document),
            Err(ScreenplayConversionError::InvalidFdxText)
        );
    }

    #[test]
    fn rejects_control_character_in_title_key() {
        let document = parse(r#"{"titlePage":[{"key":"before\u0001after","values":[]}],"elements":[]}"#);
        assert_eq!(
            screenplay_document_to_fdx(&document),
            Err(ScreenplayConversionError::InvalidFdxText)
        );
    }

    #[test]
    fn rejects_control_character_in_title_value() {
        let document = parse(
            r#"{"titlePage":[{"key":"Title","values":[{"runs":[{"text":"before\u0001after","styles":[]}]}]}],"elements":[]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document),
            Err(ScreenplayConversionError::InvalidFdxText)
        );
    }

    #[test]
    fn rejects_noncharacter_fffe_in_dual_dialogue() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"dual-dialogue","left":{"character":{"type":"character","text":{"runs":[{"text":"A","styles":[]}]}},"content":[]},"right":{"character":{"type":"character","text":{"runs":[{"text":"B","styles":[]}]}},"content":[{"type":"dialogue","text":{"runs":[{"text":"x￾y","styles":[]}]}}]}}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document),
            Err(ScreenplayConversionError::InvalidFdxText)
        );
    }

    #[test]
    fn accepts_boundary_code_points() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"action","text":{"runs":[{"text":"\t\n\r ퟿�𐀀􏿿","styles":[]}]},"alignment":"standard"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Action\"><Text>\u{9}\n",
                "\n",
                " \u{D7FF}\u{E000}\u{FFFD}\u{10000}\u{10FFFF}</Text></Paragraph>",
                "</Content>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn keeps_ordinary_orphan_dual_and_empty_dialogue_ownership_in_source_order() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"character","text":{"runs":[{"text":"CAPTAIN","styles":[]}]}},{"type":"parenthetical","text":{"runs":[{"text":"(low)","styles":[]}]}},{"type":"dialogue","text":{"runs":[{"text":"Ordinary.","styles":[]}]}},{"type":"dialogue","text":{"runs":[{"text":"Orphan dialogue.","styles":[]}]}},{"type":"parenthetical","text":{"runs":[{"text":"(orphan)","styles":[]}]}},{"type":"character","text":{"runs":[{"text":"ORPHAN CHARACTER","styles":[]}]}},{"type":"dual-dialogue","left":{"character":{"type":"character","text":{"runs":[{"text":"LEFT","styles":[]}]}},"content":[{"type":"parenthetical","text":{"runs":[{"text":"(first)","styles":[]}]}},{"type":"dialogue","text":{"runs":[{"text":"Left line.","styles":[]}]}}]},"right":{"character":{"type":"character","text":{"runs":[{"text":"RIGHT","styles":[]}]}},"content":[{"type":"dialogue","text":{"runs":[{"text":"Right one.","styles":[]}]}},{"type":"parenthetical","text":{"runs":[{"text":"(then)","styles":[]}]}},{"type":"dialogue","text":{"runs":[{"text":"Right two.","styles":[]}]}}]}},{"type":"dual-dialogue","left":{"character":{"type":"character","text":{"runs":[{"text":"EMPTY LEFT","styles":[]}]}},"content":[]},"right":{"character":{"type":"character","text":{"runs":[{"text":"EMPTY RIGHT","styles":[]}]}},"content":[]}}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Character\"><Text>CAPTAIN</Text></Paragraph>",
                "<Paragraph Type=\"Parenthetical\"><Text>(low)</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Ordinary.</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Orphan dialogue.</Text></Paragraph>",
                "<Paragraph Type=\"Parenthetical\"><Text>(orphan)</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>ORPHAN CHARACTER</Text></Paragraph>",
                "<Paragraph><DualDialogue><Paragraph Type=\"Character\"><Text>LEFT</Text></Paragraph>",
                "<Paragraph Type=\"Parenthetical\"><Text>(first)</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Left line.</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>RIGHT</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Right one.</Text></Paragraph>",
                "<Paragraph Type=\"Parenthetical\"><Text>(then)</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Right two.</Text></Paragraph>",
                "</DualDialogue></Paragraph>",
                "<Paragraph><DualDialogue><Paragraph Type=\"Character\"><Text>EMPTY LEFT</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>EMPTY RIGHT</Text></Paragraph>",
                "</DualDialogue></Paragraph>",
                "</Content>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn attaches_page_breaks_to_following_content_with_empty_action_fallbacks() {
        let document = parse(
            r#"{"titlePage":[],"elements":[{"type":"page-break"},{"type":"action","text":{"runs":[{"text":"Leading target.","styles":[]}]},"alignment":"standard"},{"type":"character","text":{"runs":[{"text":"VOICE","styles":[]}]}},{"type":"page-break"},{"type":"dialogue","text":{"runs":[{"text":"Dialogue target.","styles":[]}]}},{"type":"page-break"},{"type":"page-break"},{"type":"dual-dialogue","left":{"character":{"type":"character","text":{"runs":[{"text":"LEFT","styles":[]}]}},"content":[{"type":"dialogue","text":{"runs":[{"text":"One.","styles":[]}]}}]},"right":{"character":{"type":"character","text":{"runs":[{"text":"RIGHT","styles":[]}]}},"content":[{"type":"dialogue","text":{"runs":[{"text":"Two.","styles":[]}]}}]}},{"type":"action","text":{"runs":[{"text":"After dual.","styles":[]}]},"alignment":"standard"},{"type":"page-break"},{"type":"page-break"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Action\" StartsNewPage=\"Yes\"><Text>Leading target.</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>VOICE</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\" StartsNewPage=\"Yes\"><Text>Dialogue target.</Text></Paragraph>",
                "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"></Paragraph>",
                "<Paragraph StartsNewPage=\"Yes\"><DualDialogue><Paragraph Type=\"Character\"><Text>LEFT</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>One.</Text></Paragraph>",
                "<Paragraph Type=\"Character\"><Text>RIGHT</Text></Paragraph>",
                "<Paragraph Type=\"Dialogue\"><Text>Two.</Text></Paragraph>",
                "</DualDialogue></Paragraph>",
                "<Paragraph Type=\"Action\"><Text>After dual.</Text></Paragraph>",
                "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"></Paragraph>",
                "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"></Paragraph>",
                "</Content>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn emits_empty_fdx() {
        let document = parse(r#"{"titlePage":[],"elements":[]}"#);
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content></Content>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn emits_stable_title_and_body() {
        let document = parse(
            r#"{"titlePage":[{"key":"Title","values":[{"runs":[{"text":"Stable title","styles":[]}]}]}],"elements":[{"type":"action","text":{"runs":[{"text":"Stable ","styles":[]},{"text":"body","styles":["italic"]}]},"alignment":"standard"}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Action\"><Text>Stable </Text><Text Style=\"Italic\">body</Text></Paragraph>",
                "</Content>\n",
                "  <TitlePage><Content><Paragraph Alignment=\"Center\"><Text>Stable title</Text></Paragraph>",
                "</Content></TitlePage>\n",
                "</FinalDraft>",
            )
        );
    }

    #[test]
    fn title_page_edge_cases() {
        let document = parse(
            r#"{"titlePage":[{"key":"Constructor","values":[]},{"key":" __proto__ ","values":[{"runs":[{"text":"a\nb","styles":[]}]}]},{"key":"toString","values":[{"runs":[{"text":"x","styles":[]}]}]},{"key":"Multi","values":[{"runs":[{"text":"one\r\n","styles":["italic"]},{"text":"two","styles":["bold"]}]}]},{"key":"Tab\tKey","values":[{"runs":[{"text":"it's \"q\"","styles":["underline"]}]}]},{"key":"Empty value","values":[{"runs":[]}]}],"elements":[{"type":"scene-heading","text":{"runs":[{"text":"X","styles":[]}]},"sceneNumber":"a\tb\nc\rd"},{"type":"page-break"},{"type":"lyric","text":{"runs":[{"text":"la","styles":["italic"]}]}}]}"#,
        );
        assert_eq!(
            screenplay_document_to_fdx(&document).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n",
                "<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n",
                "  <Content><Paragraph Type=\"Scene Heading\" Number=\"a&#x9;b&#xA;c&#xD;d\"><Text>X</Text></Paragraph>",
                "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"><Text Style=\"Italic\">la</Text></Paragraph>",
                "</Content>\n",
                "  <TitlePage><Content><Paragraph Alignment=\"function Object() { [native code] }\"><Text>Constructor:</Text></Paragraph>",
                "<Paragraph Alignment=\"[object Object]\"><Text>a</Text></Paragraph>",
                "<Paragraph Alignment=\"[object Object]\"><Text>b</Text></Paragraph>",
                "<Paragraph><Text>toString: </Text><Text>x</Text></Paragraph>",
                "<Paragraph><Text>Multi:</Text></Paragraph>",
                "<Paragraph><Text Style=\"Italic\">one</Text></Paragraph>",
                "<Paragraph><Text Style=\"Bold\">two</Text></Paragraph>",
                "<Paragraph><Text>Tab\u{9}Key: </Text><Text Style=\"Underline\">it&apos;s &quot;q&quot;</Text></Paragraph>",
                "<Paragraph><Text>Empty value: </Text></Paragraph>",
                "</Content></TitlePage>\n",
                "</FinalDraft>",
            )
        );
    }
}
