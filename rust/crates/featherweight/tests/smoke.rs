//! End-to-end checks against the TypeScript golden outputs. The exhaustive,
//! stage-by-stage comparison lives in `scripts/parity.mjs`.
use featherweight::{ScreenplayConversionError, inspect_screenplay_pdf, screenplay_to_fountain, screenplay_to_json};

const PDF: &[u8] = include_bytes!("../../../../test/brick-and-steel.pdf");
const GOLDEN_FOUNTAIN: &str = include_str!("../../../../test/fixtures/fountain/brick-and-steel.expected.fountain");

fn all_pages() -> Vec<usize> {
    (0..inspect_screenplay_pdf(PDF).unwrap().page_count).collect()
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

#[test]
fn inspects_page_count() {
    let inspection = inspect_screenplay_pdf(PDF).unwrap();
    assert_eq!(inspection.page_count, 5);
    assert!(inspection.pages_needing_ocr.is_empty());
}

#[test]
fn fountain_matches_typescript_golden() {
    let fountain = screenplay_to_fountain(PDF, &all_pages(), &[]).unwrap();
    assert_eq!(normalize_newlines(&fountain), normalize_newlines(GOLDEN_FOUNTAIN));
}

#[test]
fn json_is_deterministic_and_route_order_independent() {
    let forward = screenplay_to_json(PDF, &[0, 1, 2, 3, 4], &[]).unwrap();
    let shuffled = screenplay_to_json(PDF, &[4, 2, 0, 3, 1], &[]).unwrap();
    assert_eq!(forward, shuffled);
}

#[test]
fn rejects_invalid_routes() {
    assert_eq!(
        screenplay_to_json(PDF, &[0, 0], &[]),
        Err(ScreenplayConversionError::DuplicatePageIndex)
    );
    assert_eq!(
        screenplay_to_json(PDF, &[0], &[0]),
        Err(ScreenplayConversionError::OverlappingPageIndex)
    );
    assert_eq!(
        screenplay_to_json(PDF, &[9], &[]),
        Err(ScreenplayConversionError::InvalidPageIndex)
    );
}

#[test]
fn one_call_document_matches_explicit_routes() {
    let explicit = featherweight::screenplay_to_document(PDF, &all_pages(), &[]).unwrap();
    assert_eq!(featherweight::screenplay_document(PDF).unwrap(), explicit);
    assert_eq!(
        inspect_screenplay_pdf(PDF).unwrap().page_routes(),
        (all_pages(), vec![])
    );
}
