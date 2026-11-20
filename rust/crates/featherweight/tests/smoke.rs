use featherweight::{screenplay_to_fdx, screenplay_to_fountain, screenplay_to_json};

const PDF: &[u8] = include_bytes!("../../../../test/brick-and-steel.pdf");

#[test]
fn json_parses_and_counts() {
    let insp = featherweight::inspect_screenplay_pdf(PDF).unwrap();
    let idx: Vec<usize> = (0..insp.page_count).collect();
    let json = screenplay_to_json(PDF, &idx).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["titlePage"][0]["key"], "Title");
    assert_eq!(v["titlePage"][0]["values"][0]["runs"][0]["text"], "BRICK & STEEL");
    assert!(v["elements"].as_array().unwrap().len() >= 80);
}

#[test]
fn fountain_contains_expected_markers() {
    let insp = featherweight::inspect_screenplay_pdf(PDF).unwrap();
    let idx: Vec<usize> = (0..insp.page_count).collect();
    let out = screenplay_to_fountain(PDF, &idx).unwrap();
    assert!(out.contains("SMASH CUT TO:"));
    assert!(out.contains("BRICK & STEEL"));
}

#[test]
fn fdx_is_well_formed_enough() {
    let insp = featherweight::inspect_screenplay_pdf(PDF).unwrap();
    let idx: Vec<usize> = (0..insp.page_count).collect();
    let out = screenplay_to_fdx(PDF, &idx).unwrap();
    assert!(out.starts_with("<?xml version=\"1.0\""));
    assert!(out.contains("<FinalDraft"));
    assert!(out.contains("<TitlePage>"));
}