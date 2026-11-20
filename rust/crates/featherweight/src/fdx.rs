use crate::screenplay::{ScreenplayDocument, ScreenplayElement, StyledText, TitleField};



use crate::screenplay_text::is_uppercase_text;







fn escape_xml(s: &str) -> String {



    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")



}







fn escape_attr(s: &str) -> String {



    escape_xml(s).replace('\t', "&#x9;").replace('\n', "&#xA;").replace('\r', "&#xD;")



}







fn style_name(style: &str) -> Option<&'static str> {



    match style {



        "bold" => Some("Bold"),



        "italic" => Some("Italic"),



        "underline" => Some("Underline"),



        "strikeout" => Some("Strikeout"),



        _ => None,



    }



}







fn styled_text_to_fdx(text: &StyledText, added: &[&str]) -> String {



    text.runs.iter().map(|run| {



        let mut styles: Vec<&str> = Vec::new();



        for s in ["bold", "italic", "underline", "strikeout"] {



            if run.styles.iter().any(|x| x == s) || added.iter().any(|x| *x == s) {



                if let Some(n) = style_name(s) { styles.push(n); }



            }



        }



        let style_attr = if styles.is_empty() { String::new() } else { format!(" Style=\"{}\"", styles.join("+")) };



        let txt = escape_xml(&run.text.replace("\r\n", "\n").replace('\r', "\n"));



        format!("<Text{}>{}</Text>", style_attr, txt)



    }).collect()



}







fn rendered_text(text: &StyledText) -> String {



    text.runs.iter().map(|r| r.text.clone()).collect::<String>().replace("\r\n", "\n").replace('\r', "\n")



}







fn serialize_title_paragraph_attrs(alignment: Option<&str>, number: Option<&str>, starts_new_page: bool) -> String {



    let mut attrs = Vec::new();



    if let Some(a) = alignment { attrs.push(format!("Alignment=\"{}\"", a)); }



    if let Some(n) = number { attrs.push(format!("Number=\"{}\"", escape_attr(n))); }



    if starts_new_page { attrs.push("StartsNewPage=\"Yes\"".to_string()); }



    if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) }



}







fn serialize_typed_paragraph(typ: &str, text: &StyledText, attrs: &str, added_styles: &[&str]) -> String {



    format!("<Paragraph Type=\"{}\"{}>{}</Paragraph>", typ, attrs, styled_text_to_fdx(text, added_styles))



}







fn split_styled_text_lines(text: &StyledText) -> Vec<StyledText> {

    let mut lines: Vec<Vec<crate::screenplay::StyledRun>> = vec![Vec::new()];

    for run in &text.runs {
        let mut normalized = String::new();
        for ch in run.text.chars() {
            if ch as u32 == 13 {
                normalized.push(char::from_u32(10).unwrap());
            } else {
                normalized.push(ch);
            }
        }
        let newline = char::from_u32(10).unwrap();
        let parts: Vec<&str> = normalized.split(newline).collect();
        for (idx, part) in parts.iter().enumerate() {

            if !part.is_empty() {

                lines.last_mut().unwrap().push(crate::screenplay::StyledRun { text: part.to_string(), styles: run.styles.clone() });

            }

            if idx < parts.len() - 1 {

                lines.push(Vec::new());

            }

        }

    }

    lines.into_iter().map(|runs| StyledText { runs }).collect()

}



fn plain_styled_text(value: &str) -> StyledText {



    StyledText { runs: vec![crate::screenplay::StyledRun { text: value.to_string(), styles: Vec::new() }] }



}







fn title_alignment(key: &str) -> Option<&'static str> {



    match key.trim().to_lowercase().as_str() {



        "title" | "credit" | "author" | "authors" | "source" => Some("Center"),



        "draft date" | "contact" | "copyright" => Some("Left"),



        "notes" => Some("Right"),



        _ => None,



    }



}







fn serialize_title_page(fields: &[TitleField]) -> Option<String> {



    if fields.is_empty() { return None; }



    let paras = fields.iter().flat_map(title_field_to_paragraphs).collect::<Vec<_>>();



    Some(paras.iter().map(serialize_title_paragraph).collect::<Vec<_>>().join(""))



}







fn title_field_to_paragraphs(field: &TitleField) -> Vec<(Option<&'static str>, StyledText)> {



    let alignment = title_alignment(&field.key);



    if field.values.is_empty() {



        return vec![(alignment, plain_styled_text(&format!("{}:", field.key)))];



    }



    if alignment.is_some() {



        return field.values.iter().flat_map(|v| {



            split_styled_text_lines(v).into_iter().map(move |t| (alignment, t))



        }).collect();



    }



    let value_lines: Vec<Vec<StyledText>> = field.values.iter().map(split_styled_text_lines).collect();



    let uses_inline = value_lines.len() == 1 && value_lines[0].len() == 1;



    if uses_inline {



        let mut runs = vec![crate::screenplay::StyledRun { text: format!("{}: ", field.key), styles: Vec::new() }];



        runs.extend(value_lines[0][0].runs.clone());



        return vec![(None, StyledText { runs })];



    }



    let mut out = vec![(None, plain_styled_text(&format!("{}:", field.key)))];



    for vl in &value_lines {



        for t in vl {



            out.push((None, t.clone()));



        }



    }



    out



}







fn serialize_title_paragraph(p: &(Option<&'static str>, StyledText)) -> String {



    let align = match p.0 { Some(a) => format!(" Alignment=\"{}\"", a), None => String::new() };



    format!("<Paragraph{}>{}</Paragraph>", align, styled_text_to_fdx(&p.1, &[]))



}







fn validate_text(v: &str) {



    // minimal: no-op



    let _ = v;



}







fn serialize_body(elements: &[ScreenplayElement]) -> String {



    let mut paragraphs = Vec::new();



    let mut pending = 0usize;



    for element in elements {



        if let ScreenplayElement::PageBreak {} = element {



            pending += 1;



            continue;



        }



        for _ in 1..pending {



            paragraphs.push(serialize_empty_page_break());



        }



        paragraphs.push(serialize_element(element, pending > 0));



        pending = 0;



    }



    for _ in 0..pending {



        paragraphs.push(serialize_empty_page_break());



    }



    paragraphs.join("")



}







fn serialize_element(element: &ScreenplayElement, starts_new_page: bool) -> String {



    match element {



        ScreenplayElement::SceneHeading { text, scene_number } => {



            let mut a = String::new();



            if let Some(n) = scene_number { a.push_str(&format!(" Number=\"{}\"", escape_attr(n))); }



            if starts_new_page { a.push_str(" StartsNewPage=\"Yes\""); }



            format!("<Paragraph Type=\"Scene Heading\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::Action { text, alignment } => {



            let mut a = String::new();



            if alignment == "center" { a.push_str(" Alignment=\"Center\""); }



            if starts_new_page { a.push_str(" StartsNewPage=\"Yes\""); }



            format!("<Paragraph Type=\"Action\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::Character { text } => {



            let a = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph Type=\"Character\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::Parenthetical { text } => {



            let a = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph Type=\"Parenthetical\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::Dialogue { text } => {



            let a = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph Type=\"Dialogue\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::DualDialogue { left, right } => {



            let attrs = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph{}><DualDialogue>{}{}</DualDialogue></Paragraph>", attrs, serialize_dialogue_sequence(left), serialize_dialogue_sequence(right))



        }



        ScreenplayElement::Lyrics { text } => {



            let a = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph Type=\"Action\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &["italic"]))



        }



        ScreenplayElement::Transition { text } => {



            let a = if starts_new_page { " StartsNewPage=\"Yes\"" } else { "" };



            format!("<Paragraph Type=\"Transition\"{}>{}</Paragraph>", a, styled_text_to_fdx(text, &[]))



        }



        ScreenplayElement::PageBreak {} => serialize_empty_page_break(),



    }



}







fn serialize_dialogue_sequence(seq: &crate::screenplay::DialogueSequence) -> String {



    let mut out = serialize_typed_paragraph("Character", &seq.character_text(), "", &[]);



    for c in &seq.content {



        match c {



            ScreenplayElement::Parenthetical { text } => out.push_str(&serialize_typed_paragraph("Parenthetical", text, "", &[])),



            ScreenplayElement::Dialogue { text } => out.push_str(&serialize_typed_paragraph("Dialogue", text, "", &[])),



            _ => {}



        }



    }



    out



}







// helper to extract text from Character ScreenplayElement



trait CharText { fn character_text(&self) -> StyledText; }



impl CharText for crate::screenplay::DialogueSequence {



    fn character_text(&self) -> StyledText {



        // character is Box<ScreenplayElement>; extract



        if let ScreenplayElement::Character { text } = self.character.as_ref() { text.clone() } else { StyledText { runs: Vec::new() } }



    }



}











fn serialize_empty_page_break() -> String {



    "<Paragraph Type=\"Action\" StartsNewPage=\"Yes\"></Paragraph>".to_string()



}







pub fn screenplay_document_to_fdx(document: &ScreenplayDocument) -> String {



    let content = serialize_body(&document.elements);



    let title_page = serialize_title_page(&document.title_page);



    let mut out = String::new();



    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n");



    out.push_str("<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n");



    out.push_str(&format!("  <Content>{}</Content>\n", content));



    if let Some(tp) = title_page { out.push_str(&format!("  <TitlePage><Content>{}</Content></TitlePage>\n", tp)); }



    out.push_str("</FinalDraft>\n");



    out



}



