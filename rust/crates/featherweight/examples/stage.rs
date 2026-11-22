//! Run one pipeline stage on JSON from stdin, for the TS/Rust parity harness
//! (`scripts/parity.mjs`). Input and output use the TypeScript JSON shapes.
//!
//!   stage physical   < PositionedText                      > PhysicalText
//!   stage normalize  < {physicalText, pageIndexes}         > NormalizedText
//!   stage layout     < NormalizedText                      > ScreenplayLayout
//!   stage recognize  < {normalizedText, layout}            > ScreenplayDocument
//!   stage fountain   < ScreenplayDocument                  > text
//!   stage fdx        < ScreenplayDocument                  > text | ERROR <code>
//!   stage convert <json|fountain|fdx> <pdf> <native,..> <ocr,..>
use std::io::Read;

use featherweight::core::layout::infer_screenplay_layout::infer_screenplay_layout;
use featherweight::core::layout::screenplay_layout::ScreenplayLayout;
use featherweight::core::normalized_text::NormalizedText;
use featherweight::core::physical_lines::{PhysicalText, group_positioned_text_into_physical_lines};
use featherweight::core::positioned_text::PositionedText;
use featherweight::core::recognition::recognize_screenplay::recognize_screenplay;
use featherweight::core::screenplay_document::ScreenplayDocument;
use featherweight::fdx::screenplay_document_to_fdx::screenplay_document_to_fdx;
use featherweight::fountain::screenplay_document_to_fountain::screenplay_document_to_fountain;
use featherweight::normalization::physical_text::normalize_physical_text;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizeInput {
    physical_text: PhysicalText,
    page_indexes: Vec<usize>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecognizeInput {
    normalized_text: NormalizedText,
    layout: ScreenplayLayout,
}

fn input<T: serde::de::DeserializeOwned>() -> T {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).expect("read stdin");
    serde_json::from_str(&raw).unwrap_or_else(|error| panic!("invalid stage input: {error}"))
}

fn json(value: &impl serde::Serialize) {
    print!("{}", serde_json::to_string(value).expect("serialize"));
}

fn pages(arg: Option<String>) -> Vec<usize> {
    arg.filter(|value| !value.is_empty())
        .map(|value| value.split(',').map(|index| index.parse().expect("page index")).collect())
        .unwrap_or_default()
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("physical") => json(&group_positioned_text_into_physical_lines(&input::<PositionedText>())),
        Some("normalize") => {
            let NormalizeInput { physical_text, page_indexes } = input();
            json(&normalize_physical_text(&physical_text, &page_indexes));
        }
        Some("layout") => json(&infer_screenplay_layout(&input::<NormalizedText>())),
        Some("recognize") => {
            let RecognizeInput { normalized_text, layout } = input();
            json(&recognize_screenplay(&normalized_text, &layout));
        }
        Some("fountain") => print!("{}", screenplay_document_to_fountain(&input::<ScreenplayDocument>())),
        Some("fdx") => match screenplay_document_to_fdx(&input::<ScreenplayDocument>()) {
            Ok(fdx) => print!("{fdx}"),
            Err(error) => print!("ERROR {}", error.code()),
        },
        Some("convert") => {
            let format = args.next().expect("format");
            let pdf = std::fs::read(args.next().expect("pdf path")).expect("read pdf");
            let (native, ocr) = (pages(args.next()), pages(args.next()));
            let result = match format.as_str() {
                "json" => featherweight::screenplay_to_json(&pdf, &native, &ocr),
                "fountain" => featherweight::screenplay_to_fountain(&pdf, &native, &ocr),
                "fdx" => featherweight::screenplay_to_fdx(&pdf, &native, &ocr),
                other => panic!("unknown format {other}"),
            };
            match result {
                Ok(output) => print!("{output}"),
                Err(error) => print!("ERROR {}", error.code()),
            }
        }
        other => panic!("unknown stage {other:?}"),
    }
}
