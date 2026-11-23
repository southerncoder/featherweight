//! One-call screenplay import for applications such as editors: reports pages
//! that need OCR, supports progress and cancellation, never panics, and returns
//! user-facing errors. Not in the TypeScript API.
//!
//! The document is identical to [`screenplay_to_document`](crate::screenplay_to_document)
//! with the routes from [`PdfInspection::page_routes`], except that each element
//! also carries its 0-based source `page`.
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::conversion::{
    PositionedTextPage, empty_page, normalize_pages, positioned_text_to_pages, recognize_normalized_text_with_pages,
};
use crate::core::positioned_text::SourceMethod;
use crate::core::screenplay_document::ScreenplayDocument;
use crate::error::ImportError;
use crate::extraction::{PdfInspection, extract_positioned_pdf_text, inspect_screenplay_pdf};

/// Native pages extracted per `pdf-inspector` call. `pdf-inspector` re-parses
/// the PDF on every call (about 5 ms for a feature script), so small batches make
/// extraction noticeably slower: 10-page batches cost 2.5x a single call on a
/// 133-page script, 25-page batches about 1.6x, while still reporting progress
/// several times per feature-length script.
pub const EXTRACTION_BATCH_PAGES: usize = 25;

/// A successful import.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Import {
    /// The recognized screenplay, each element with its 0-based source `page`.
    /// Pages needing OCR contribute nothing to it.
    pub document: ScreenplayDocument,
    /// Page count and the pages without usable embedded text.
    pub inspection: PdfInspection,
    /// Conditions the user should be told about.
    pub warnings: Vec<ImportWarning>,
}

/// Something the user should know about a successful import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ImportWarning {
    /// These pages have no usable embedded text (typically scanned images), so
    /// their content is missing from the document. `pages` are 0-based page
    /// indexes in ascending order; add 1 for page numbers shown to users.
    PagesNeedOcr { pages: Vec<usize> },
}

/// Options for [`import_screenplay_with`].
#[derive(Default, Clone, Copy)]
pub struct ImportOptions<'a> {
    /// Called on the importing thread at the start of each phase and after each
    /// extraction batch.
    pub on_progress: Option<&'a (dyn Fn(Progress) + Send + Sync)>,
    /// When set to `true` (from any thread), the import stops at the next batch
    /// or phase boundary and returns [`ImportError::Cancelled`].
    pub cancel: Option<&'a AtomicBool>,
}

/// Import progress.
///
/// During [`Phase::Extracting`], `done` and `total` count native pages, and
/// `done` only increases. Every other phase is reported once, at its start, with
/// `done: 0, total: 1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Progress {
    pub phase: Phase,
    pub done: usize,
    pub total: usize,
}

/// An import phase, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Inspecting,
    Extracting,
    Normalizing,
    Recognizing,
}

/// Imports a screenplay PDF with default options. See [`import_screenplay_with`].
pub fn import_screenplay(pdf_bytes: &[u8]) -> Result<Import, ImportError> {
    import_screenplay_with(pdf_bytes, ImportOptions::default())
}

/// Imports a screenplay PDF: inspects it once, extracts the pages with embedded
/// text in batches of [`EXTRACTION_BATCH_PAGES`], and recognizes the screenplay.
///
/// Pages without usable embedded text are reported in
/// [`ImportWarning::PagesNeedOcr`] and left out of the document. A PDF with no
/// usable embedded text at all (or no pages) is [`ImportError::NoEmbeddedText`]
/// rather than an empty document.
///
/// Never panics: the pipeline runs under [`catch_unwind`], because the PDF parser
/// (`pdf-inspector`) is third-party code fed untrusted input, and any panic
/// becomes [`ImportError::Internal`]. The default panic hook still prints the
/// panic message to stderr.
pub fn import_screenplay_with(pdf_bytes: &[u8], options: ImportOptions<'_>) -> Result<Import, ImportError> {
    catch_unwind(AssertUnwindSafe(|| {
        run_import(pdf_bytes, &options, EXTRACTION_BATCH_PAGES)
    }))
    .unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<&str>()
            .map(|message| (*message).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic while importing".to_owned());
        Err(ImportError::Internal {
            cause: Some(std::sync::Arc::from(Box::<dyn std::error::Error + Send + Sync>::from(
                message,
            ))),
        })
    })
}

fn run_import(pdf_bytes: &[u8], options: &ImportOptions<'_>, batch_pages: usize) -> Result<Import, ImportError> {
    let check_cancel = || -> Result<(), ImportError> {
        match options.cancel {
            Some(cancel) if cancel.load(Ordering::Relaxed) => Err(ImportError::Cancelled),
            _ => Ok(()),
        }
    };
    let report = |phase: Phase, done: usize, total: usize| -> Result<(), ImportError> {
        check_cancel()?;
        if let Some(on_progress) = options.on_progress {
            on_progress(Progress { phase, done, total });
        }
        Ok(())
    };

    report(Phase::Inspecting, 0, 1)?;
    let inspection = inspect_screenplay_pdf(pdf_bytes)?;

    if inspection.pages_needing_ocr.len() >= inspection.page_count {
        return Err(ImportError::NoEmbeddedText);
    }

    let (native, ocr) = inspection.page_routes();
    let mut native_pages: Vec<PositionedTextPage> = Vec::with_capacity(native.len());
    report(Phase::Extracting, 0, native.len())?;

    for batch in native.chunks(batch_pages.max(1)) {
        check_cancel()?;
        let positioned_text = extract_positioned_pdf_text(pdf_bytes, Some(batch))?;
        native_pages.extend(positioned_text_to_pages(positioned_text, batch));
        report(Phase::Extracting, native_pages.len(), native.len())?;
    }

    report(Phase::Normalizing, 0, 1)?;
    let ocr_pages: Vec<PositionedTextPage> = ocr.iter().map(|&page_index| empty_page(page_index)).collect();
    let mut pages: Vec<(&PositionedTextPage, SourceMethod)> = native_pages
        .iter()
        .map(|page| (page, SourceMethod::EmbeddedText))
        .chain(ocr_pages.iter().map(|page| (page, SourceMethod::Ocr)))
        .collect();
    pages.sort_by_key(|(page, _)| page.page_index);
    let normalized_text = normalize_pages(&pages);

    report(Phase::Recognizing, 0, 1)?;
    let document = recognize_normalized_text_with_pages(&normalized_text);
    let warnings = if ocr.is_empty() {
        Vec::new()
    } else {
        vec![ImportWarning::PagesNeedOcr { pages: ocr }]
    };

    Ok(Import {
        document,
        inspection,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    const MIXED: &[u8] = include_bytes!("../../../../test/brick-and-steel-mixed.pdf");
    const NATIVE: &[u8] = include_bytes!("../../../../test/brick-and-steel.pdf");

    #[test]
    fn batching_does_not_change_the_document() {
        for pdf in [NATIVE, MIXED] {
            let whole = run_import(pdf, &ImportOptions::default(), EXTRACTION_BATCH_PAGES).unwrap();
            let page_by_page = run_import(pdf, &ImportOptions::default(), 1).unwrap();
            assert_eq!(whole, page_by_page);
        }
    }

    #[test]
    fn reports_phases_in_order_with_monotonic_extraction_progress() {
        let events = Mutex::new(Vec::new());
        let on_progress = |progress: Progress| events.lock().unwrap().push(progress);
        let options = ImportOptions {
            on_progress: Some(&on_progress),
            cancel: None,
        };
        run_import(NATIVE, &options, 2).unwrap();
        let events = events.into_inner().unwrap();

        let phases: Vec<Phase> = events.iter().map(|event| event.phase).collect();
        assert_eq!(phases.first(), Some(&Phase::Inspecting));
        assert_eq!(phases.last(), Some(&Phase::Recognizing));
        assert!(phases.windows(2).all(|pair| pair[0] as u8 <= pair[1] as u8));

        let extracting: Vec<(usize, usize)> = events
            .iter()
            .filter(|event| event.phase == Phase::Extracting)
            .map(|event| (event.done, event.total))
            .collect();
        assert_eq!(extracting, [(0, 5), (2, 5), (4, 5), (5, 5)]);
    }

    #[test]
    fn a_preset_cancel_flag_cancels_before_any_work() {
        let cancel = AtomicBool::new(true);
        let options = ImportOptions {
            on_progress: None,
            cancel: Some(&cancel),
        };
        assert!(matches!(run_import(NATIVE, &options, 1), Err(ImportError::Cancelled)));
    }

    #[test]
    fn cancelling_during_extraction_stops_before_the_next_batch() {
        let cancel = AtomicBool::new(false);
        let extracted = Mutex::new(Vec::new());
        let on_progress = |progress: Progress| {
            if progress.phase == Phase::Extracting {
                extracted.lock().unwrap().push(progress.done);
                if progress.done == 1 {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
        };
        let options = ImportOptions {
            on_progress: Some(&on_progress),
            cancel: Some(&cancel),
        };

        assert!(matches!(run_import(NATIVE, &options, 1), Err(ImportError::Cancelled)));
        assert_eq!(extracted.into_inner().unwrap(), [0, 1]);
    }
}
