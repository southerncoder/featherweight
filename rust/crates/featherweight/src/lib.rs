//! Featherweight: turn a screenplay PDF into Fountain, FDX, or structured JSON.
//!
//! Rust port of `@bantam-hq/featherweight`. Module paths mirror the TypeScript
//! sources one file per module (`src/core/layout/dialogue-occurrences.ts` ->
//! [`core::layout::dialogue_occurrences`]), so the two can be reviewed side by side.
//!
//! ```no_run
//! let pdf = std::fs::read("script.pdf")?;
//! let import = featherweight::import_screenplay(&pdf)?;
//! for warning in &import.warnings {
//!     eprintln!("{warning:?}"); // e.g. pages that need OCR
//! }
//! for element in &import.document.elements {
//!     if let featherweight::ScreenplayElement::SceneHeading { text, .. } = element {
//!         println!("{}", text.runs.iter().map(|run| run.text.as_str()).collect::<String>());
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Text offsets ([`core::physical_lines::PhysicalTextSpan::start`]/`end`) are
//! UTF-8 byte offsets into the line text, where the TypeScript port uses UTF-16
//! code units. Everything else follows the TypeScript semantics exactly.
//!
//! Features: `fountain` and `fdx` (default) enable those serializers and the
//! `featherweight` binary; `typescript` derives TypeScript declarations with
//! `ts-rs` (see `examples/export_types.rs`).

pub mod conversion;
pub mod core;
pub mod error;
pub mod extraction;
#[cfg(feature = "fdx")]
pub mod fdx;
#[cfg(feature = "fountain")]
pub mod fountain;
pub mod import;
pub mod normalization;
#[cfg(feature = "typescript")]
pub mod typescript;

#[cfg(feature = "fdx")]
pub use conversion::screenplay_to_fdx;
#[cfg(feature = "fountain")]
pub use conversion::screenplay_to_fountain;
pub use conversion::{
    PositionedTextPage, create_screenplay_document, screenplay_document, screenplay_to_document, screenplay_to_json,
};
pub use core::character_cue::{CharacterCue, split_character_cue};
pub use core::screenplay_document::{
    ActionAlignment, Character, DialogueContent, DialogueSequence, ScreenplayDocument, ScreenplayElement, StyledText,
    TextRun, TextStyle, TitlePageField,
};
pub use error::{ErrorCause, ImportError, PdfInspectionError, ScreenplayConversionError};
pub use extraction::{PdfInspection, inspect_screenplay_pdf};
pub use import::{
    EXTRACTION_BATCH_PAGES, Import, ImportOptions, ImportWarning, Phase, Progress, import_screenplay,
    import_screenplay_with,
};
