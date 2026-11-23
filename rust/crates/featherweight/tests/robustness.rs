//! Malformed input never panics: every case returns an error or a well-formed
//! document, through both the import API and the original API. Deterministic
//! (fixed seeds) and fast.
use std::panic::{AssertUnwindSafe, catch_unwind};

use featherweight::{ImportError, import_screenplay, screenplay_document};

const PDF: &[u8] = include_bytes!("../../../../test/brick-and-steel.pdf");
const MIXED: &[u8] = include_bytes!("../../../../test/brick-and-steel-mixed.pdf");

/// xorshift64: deterministic bytes without a dependency.
fn random_bytes(seed: u64, length: usize) -> Vec<u8> {
    let mut state = seed;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn with_flipped_bytes(pdf: &[u8], seed: u64) -> Vec<u8> {
    let mut bytes = pdf.to_vec();
    let middle = bytes.len() / 2;
    for (byte, noise) in bytes[middle..middle + 512].iter_mut().zip(random_bytes(seed, 512)) {
        *byte ^= noise | 1;
    }
    bytes
}

fn cases() -> Vec<(String, Vec<u8>)> {
    let mut cases = vec![("empty".to_owned(), Vec::new())];
    for seed in 1..=4 {
        cases.push((format!("garbage {seed}"), random_bytes(seed, 4096)));
        let mut with_header = b"%PDF-1.7\n".to_vec();
        with_header.extend(random_bytes(seed + 100, 4096));
        cases.push((format!("garbage after a PDF header {seed}"), with_header));
    }
    for (name, pdf) in [("native", PDF), ("mixed", MIXED)] {
        for percent in [25, 50, 90] {
            cases.push((
                format!("{name} truncated at {percent}%"),
                pdf[..pdf.len() * percent / 100].to_vec(),
            ));
        }
        for seed in 1..=3 {
            cases.push((
                format!("{name} with flipped bytes {seed}"),
                with_flipped_bytes(pdf, seed),
            ));
        }
    }
    cases
}

#[test]
fn import_never_panics_on_malformed_input() {
    for (name, bytes) in cases() {
        match import_screenplay(&bytes) {
            // A caught panic: the pipeline must not rely on catch_unwind.
            Err(error @ ImportError::Internal { .. }) => panic!("{name}: internal error {error:?}"),
            Err(_) => {}
            Ok(import) => {
                assert!(serde_json::to_string(&import).is_ok(), "{name}");
                assert!(
                    import
                        .inspection
                        .pages_needing_ocr
                        .iter()
                        .all(|&page| page < import.inspection.page_count)
                );
            }
        }
    }
}

#[test]
fn the_original_api_never_panics_on_malformed_input() {
    for (name, bytes) in cases() {
        let result = catch_unwind(AssertUnwindSafe(|| screenplay_document(&bytes)));
        assert!(result.is_ok(), "{name}: panicked");
    }
}
