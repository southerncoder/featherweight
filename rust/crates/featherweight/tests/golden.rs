//! Golden JSON for consumers (`test/brick-and-steel.json`,
//! `test/brick-and-steel.import.json`). Fails when they are stale; regenerate with
//! `FEATHERWEIGHT_UPDATE_GOLDEN=1 cargo test -p featherweight --test golden`.
use std::path::PathBuf;

use featherweight::{import_screenplay, screenplay_to_json};

const PDF: &[u8] = include_bytes!("../../../../test/brick-and-steel.pdf");

fn check_golden(file: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test")
        .join(file);
    if std::env::var_os("FEATHERWEIGHT_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert!(
        expected == actual,
        "{} is stale; set FEATHERWEIGHT_UPDATE_GOLDEN=1 to regenerate",
        path.display()
    );
}

#[test]
fn document_json_is_current() {
    check_golden(
        "brick-and-steel.json",
        &screenplay_to_json(PDF, &[0, 1, 2, 3, 4], &[]).unwrap(),
    );
}

#[test]
fn import_json_is_current() {
    let import = import_screenplay(PDF).unwrap();
    check_golden(
        "brick-and-steel.import.json",
        &(serde_json::to_string_pretty(&import).unwrap() + "\n"),
    );
}
