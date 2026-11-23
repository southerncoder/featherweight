//! Port of `src/screenplay/screenplay-conversion.ts`,
//! `create-screenplay-document.ts` and `positioned-text-pages.ts`.
use std::collections::{HashMap, HashSet};

use crate::core::layout::infer_screenplay_layout::infer_screenplay_layout;
use crate::core::normalized_text::NormalizedText;
use crate::core::physical_lines::group_positioned_text_into_physical_lines;
use crate::core::positioned_text::{PositionedText, PositionedTextItem, SourceMethod};
use crate::core::recognition::recognize_screenplay::{recognize_screenplay, recognize_screenplay_with_pages};
use crate::core::screenplay_document::ScreenplayDocument;
use crate::error::ScreenplayConversionError;
use crate::extraction::{extract_positioned_pdf_text, inspect_screenplay_pdf};
#[cfg(feature = "fdx")]
use crate::fdx::screenplay_document_to_fdx::screenplay_document_to_fdx;
#[cfg(feature = "fountain")]
use crate::fountain::screenplay_document_to_fountain::screenplay_document_to_fountain;
use crate::normalization::physical_text::normalize_physical_text;

/// Port of `PositionedTextPage`: one page's items. `source_index`,
/// `source_method` and `page_index` on the items are reassigned when assembled.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedTextPage {
    pub page_index: usize,
    pub items: Vec<PositionedTextItem>,
}

/// Superseded by [`import_screenplay`](crate::import_screenplay), which parses
/// the PDF less often, reports pages needing OCR, supports progress and cancel,
/// and returns user-facing errors. Kept unchanged for existing callers.
///
/// Converts a whole PDF in one call, routing pages with
/// [`PdfInspection::page_routes`](crate::PdfInspection::page_routes): embedded
/// text natively, scanned pages to OCR (empty until OCR is available). Not in
/// the TypeScript API; equivalent to inspecting, then calling
/// [`screenplay_to_document`] with those routes.
pub fn screenplay_document(pdf_bytes: &[u8]) -> Result<ScreenplayDocument, ScreenplayConversionError> {
    let inspection = inspect_screenplay_pdf(pdf_bytes).map_err(|_| ScreenplayConversionError::PdfProcessingFailed)?;
    let (native, ocr) = inspection.page_routes();
    screenplay_to_document(pdf_bytes, &native, &ocr)
}

/// Port of `screenplayToJSON`: pretty JSON with a trailing newline, byte-identical
/// to `JSON.stringify(document, null, 2) + "\n"`.
pub fn screenplay_to_json(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
    ocr_page_indexes: &[usize],
) -> Result<String, ScreenplayConversionError> {
    let document = screenplay_to_document(pdf_bytes, native_page_indexes, ocr_page_indexes)?;
    // invariant: documents hold only strings, vectors and string-keyed structs,
    // which serde_json always serializes.
    let json = serde_json::to_string_pretty(&document).expect("documents always serialize");
    Ok(json + "\n")
}

/// Port of `screenplayToFountain`. Requires the `fountain` feature (default).
#[cfg(feature = "fountain")]
pub fn screenplay_to_fountain(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
    ocr_page_indexes: &[usize],
) -> Result<String, ScreenplayConversionError> {
    let document = screenplay_to_document(pdf_bytes, native_page_indexes, ocr_page_indexes)?;
    Ok(screenplay_document_to_fountain(&document))
}

/// Port of `screenplayToFDX`. Requires the `fdx` feature (default).
#[cfg(feature = "fdx")]
pub fn screenplay_to_fdx(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
    ocr_page_indexes: &[usize],
) -> Result<String, ScreenplayConversionError> {
    let document = screenplay_to_document(pdf_bytes, native_page_indexes, ocr_page_indexes)?;
    screenplay_document_to_fdx(&document)
}

/// Port of `createDocumentFromPdf`: the structured document, for callers (such as
/// an editor) that want the elements rather than a serialization. Pages routed to
/// OCR are represented as empty pages until OCR is available.
pub fn screenplay_to_document(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
    ocr_page_indexes: &[usize],
) -> Result<ScreenplayDocument, ScreenplayConversionError> {
    let (native, ocr) = validate_page_routes(native_page_indexes, ocr_page_indexes)?;
    let inspection = inspect_screenplay_pdf(pdf_bytes).map_err(|_| ScreenplayConversionError::PdfProcessingFailed)?;

    if native
        .iter()
        .chain(&ocr)
        .any(|&page_index| page_index >= inspection.page_count)
    {
        return Err(ScreenplayConversionError::InvalidPageIndex);
    }

    let native_pages = extract_native_pages(pdf_bytes, &native)?;
    let ocr_pages: Vec<PositionedTextPage> = ocr.iter().map(|&page_index| empty_page(page_index)).collect();

    Ok(create_screenplay_document(&native_pages, &ocr_pages))
}

/// Port of `createScreenplayDocument`.
pub fn create_screenplay_document(
    native_pages: &[PositionedTextPage],
    ocr_pages: &[PositionedTextPage],
) -> ScreenplayDocument {
    let mut pages: Vec<(&PositionedTextPage, SourceMethod)> = native_pages
        .iter()
        .map(|page| (page, SourceMethod::EmbeddedText))
        .chain(ocr_pages.iter().map(|page| (page, SourceMethod::Ocr)))
        .collect();
    pages.sort_by_key(|(page, _)| page.page_index);

    recognize_normalized_text(&normalize_pages(&pages))
}

/// The first half of [`create_screenplay_document`]: assemble, group into
/// physical lines, normalize. `pages` must be sorted by page index.
pub(crate) fn normalize_pages(pages: &[(&PositionedTextPage, SourceMethod)]) -> NormalizedText {
    let positioned_text = assemble_positioned_text(pages);
    let physical_text = group_positioned_text_into_physical_lines(&positioned_text);
    let page_indexes: Vec<usize> = pages.iter().map(|(page, _)| page.page_index).collect();
    normalize_physical_text(&physical_text, &page_indexes)
}

/// The second half of [`create_screenplay_document`]: infer layout, recognize.
pub(crate) fn recognize_normalized_text(normalized_text: &NormalizedText) -> ScreenplayDocument {
    let layout = infer_screenplay_layout(normalized_text);
    recognize_screenplay(normalized_text, &layout)
}

/// [`recognize_normalized_text`] with each element's source `page` set.
pub(crate) fn recognize_normalized_text_with_pages(normalized_text: &NormalizedText) -> ScreenplayDocument {
    let layout = infer_screenplay_layout(normalized_text);
    recognize_screenplay_with_pages(normalized_text, &layout)
}

fn assemble_positioned_text(pages: &[(&PositionedTextPage, SourceMethod)]) -> PositionedText {
    let mut items = Vec::new();

    for (page, source_method) in pages {
        for item in &page.items {
            items.push(PositionedTextItem {
                source_index: items.len(),
                source_method: *source_method,
                page_index: page.page_index,
                ..item.clone()
            });
        }
    }

    PositionedText { items }
}

fn validate_page_routes(
    native_page_indexes: &[usize],
    ocr_page_indexes: &[usize],
) -> Result<(Vec<usize>, Vec<usize>), ScreenplayConversionError> {
    let native = validate_page_indexes(native_page_indexes)?;
    let ocr = validate_page_indexes(ocr_page_indexes)?;
    let native_set: HashSet<usize> = native.iter().copied().collect();

    if ocr.iter().any(|page_index| native_set.contains(page_index)) {
        return Err(ScreenplayConversionError::OverlappingPageIndex);
    }

    let mut native = native;
    let mut ocr = ocr;
    native.sort_unstable();
    ocr.sort_unstable();
    Ok((native, ocr))
}

fn validate_page_indexes(value: &[usize]) -> Result<Vec<usize>, ScreenplayConversionError> {
    let mut unique = HashSet::new();

    for &page_index in value {
        if !unique.insert(page_index) {
            return Err(ScreenplayConversionError::DuplicatePageIndex);
        }
    }

    Ok(value.to_vec())
}

fn extract_native_pages(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
) -> Result<Vec<PositionedTextPage>, ScreenplayConversionError> {
    if native_page_indexes.is_empty() {
        return Ok(Vec::new());
    }

    let positioned_text = extract_positioned_pdf_text(pdf_bytes, Some(native_page_indexes))
        .map_err(|_| ScreenplayConversionError::PdfProcessingFailed)?;

    Ok(positioned_text_to_pages(positioned_text, native_page_indexes))
}

/// Port of `positionedTextToPages` plus the per-route lookup in
/// `extractNativePages`: one page per requested index, in request order, empty
/// when the PDF had no text items on it.
pub(crate) fn positioned_text_to_pages(
    positioned_text: PositionedText,
    page_indexes: &[usize],
) -> Vec<PositionedTextPage> {
    let mut extracted: HashMap<usize, Vec<PositionedTextItem>> = HashMap::new();

    for item in positioned_text.items {
        extracted.entry(item.page_index).or_default().push(item);
    }

    page_indexes
        .iter()
        .map(|&page_index| PositionedTextPage {
            page_index,
            items: extracted.remove(&page_index).unwrap_or_default(),
        })
        .collect()
}

pub(crate) fn empty_page(page_index: usize) -> PositionedTextPage {
    PositionedTextPage {
        page_index,
        items: Vec::new(),
    }
}
