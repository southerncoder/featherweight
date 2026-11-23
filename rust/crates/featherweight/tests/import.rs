//! `import_screenplay`: same documents as the explicit API, OCR reporting, and
//! user-facing error classification.
use std::error::Error;

use featherweight::{
    ImportError, ImportWarning, ScreenplayDocument, ScreenplayElement, import_screenplay, inspect_screenplay_pdf,
    screenplay_to_document,
};

const FIXTURES: &[(&str, &[u8])] = &[
    (
        "brick-and-steel",
        include_bytes!("../../../../test/brick-and-steel.pdf"),
    ),
    (
        "brick-and-steel-mixed",
        include_bytes!("../../../../test/brick-and-steel-mixed.pdf"),
    ),
    (
        "brick-and-steel-scanned",
        include_bytes!("../../../../test/brick-and-steel-scanned.pdf"),
    ),
    (
        "normalization-gauntlet-clean",
        include_bytes!("../../../../test/normalization-gauntlet-clean.pdf"),
    ),
    (
        "normalization-gauntlet-artifacts",
        include_bytes!("../../../../test/normalization-gauntlet-artifacts.pdf"),
    ),
];

/// The document as JSON without the `page` fields only `import_screenplay` sets.
fn without_pages(document: &ScreenplayDocument) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("page");
                map.values_mut().for_each(strip);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(document).unwrap();
    strip(&mut value);
    value
}

fn element_page(element: &ScreenplayElement) -> Option<usize> {
    match element {
        ScreenplayElement::SceneHeading { page, .. }
        | ScreenplayElement::Action { page, .. }
        | ScreenplayElement::Character { page, .. }
        | ScreenplayElement::Parenthetical { page, .. }
        | ScreenplayElement::Dialogue { page, .. }
        | ScreenplayElement::DualDialogue { page, .. }
        | ScreenplayElement::Lyric { page, .. }
        | ScreenplayElement::Transition { page, .. } => *page,
        ScreenplayElement::PageBreak => None,
    }
}

fn fixture(name: &str) -> &'static [u8] {
    FIXTURES.iter().find(|(fixture, _)| *fixture == name).unwrap().1
}

#[test]
fn matches_the_explicit_api_on_every_fixture_with_embedded_text() {
    for (name, pdf) in FIXTURES {
        let inspection = inspect_screenplay_pdf(pdf).unwrap();
        if inspection.pages_needing_ocr.len() == inspection.page_count {
            continue;
        }
        let (native, ocr) = inspection.page_routes();
        let import = import_screenplay(pdf).unwrap_or_else(|error| panic!("{name}: {error}"));
        let explicit = screenplay_to_document(pdf, &native, &ocr).unwrap();
        assert_eq!(without_pages(&import.document), without_pages(&explicit), "{name}");
        assert_eq!(import.inspection, inspection, "{name}");
    }
}

#[test]
fn reports_scanned_pages_of_a_mixed_pdf() {
    let import = import_screenplay(fixture("brick-and-steel-mixed")).unwrap();
    assert_eq!(import.inspection.page_count, 5);
    assert_eq!(import.inspection.pages_needing_ocr, [1, 3]);
    assert_eq!(import.warnings, [ImportWarning::PagesNeedOcr { pages: vec![1, 3] }]);
    assert!(!import.document.elements.is_empty());
}

#[test]
fn every_element_records_its_source_page() {
    for name in ["brick-and-steel", "brick-and-steel-mixed"] {
        let import = import_screenplay(fixture(name)).unwrap();
        let pages: Vec<usize> = import
            .document
            .elements
            .iter()
            .map(|element| element_page(element).unwrap())
            .collect();
        assert!(pages.windows(2).all(|pair| pair[0] <= pair[1]), "{name}: {pages:?}");
        assert!(
            pages
                .iter()
                .all(|page| !import.inspection.pages_needing_ocr.contains(page)),
            "{name}"
        );
        assert!(pages.iter().all(|&page| page < import.inspection.page_count), "{name}");
    }
    // The original API leaves pages unset, so its JSON is unchanged.
    let explicit = screenplay_to_document(fixture("brick-and-steel"), &[0, 1, 2, 3, 4], &[]).unwrap();
    assert!(explicit.elements.iter().all(|element| element_page(element).is_none()));
}

#[test]
fn a_fully_native_pdf_has_no_warnings() {
    assert!(
        import_screenplay(fixture("brick-and-steel"))
            .unwrap()
            .warnings
            .is_empty()
    );
}

#[test]
fn a_fully_scanned_pdf_is_an_error_not_an_empty_document() {
    let error = import_screenplay(fixture("brick-and-steel-scanned")).unwrap_err();
    assert!(matches!(error, ImportError::NoEmbeddedText), "{error:?}");
}

#[test]
fn classifies_inputs_that_are_not_pdfs() {
    for bytes in [&b""[..], b"hello, this is plain text", b"\x89PNG\r\n\x1a\n"] {
        let error = import_screenplay(bytes).unwrap_err();
        assert!(matches!(error, ImportError::NotAPdf { .. }), "{bytes:?}: {error:?}");
        assert_eq!(error.code(), "NOT_A_PDF");
        assert!(error.source().is_some(), "the parser's cause is kept");
    }
}

#[test]
fn classifies_truncated_pdfs_as_corrupt() {
    let pdf = fixture("brick-and-steel");
    for bytes in [&b"%PDF-1.4\n"[..], &pdf[..pdf.len() / 2]] {
        let error = import_screenplay(bytes).unwrap_err();
        assert!(matches!(error, ImportError::Corrupt { .. }), "{error:?}");
    }
}

/// A minimal one-page PDF with a standard security handler whose (garbage)
/// password data rejects the empty user password.
fn encrypted_pdf() -> Vec<u8> {
    let password_hash = format!("<{}>", "41".repeat(32));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_owned(),
        format!("<< /Filter /Standard /V 1 /R 2 /O {password_hash} /U {password_hash} /P -4 >>"),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let id = "<00112233445566778899AABBCCDDEEFF>";
    pdf.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Encrypt 4 0 R /ID [{id} {id}] >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[test]
fn classifies_password_protected_pdfs() {
    let error = import_screenplay(&encrypted_pdf()).unwrap_err();
    assert!(matches!(error, ImportError::Encrypted { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        "That PDF is password-protected. Remove the password and try again."
    );
}

#[test]
fn errors_serialize_as_code_and_message_without_the_cause() {
    let error = import_screenplay(b"not a pdf").unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::json!({ "code": "NOT_A_PDF", "message": "That file isn't a PDF." })
    );
}
