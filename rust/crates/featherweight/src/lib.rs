//! Featherweight: turn a screenplay PDF into structured screenplay data.
//!
//! Early Rust port of the TypeScript `@bantam-hq/featherweight` library.

use thiserror::Error;

pub mod coordinate_evidence;
pub mod layout_evidence;
pub mod normalized_text;
pub mod screenplay_sequences;
pub mod dialogue_occurrences;
pub mod transition_evidence;
pub mod screenplay_text;
pub mod pdf_normalization;
pub mod page_extents;
pub mod sequence_geometry;
pub mod fountain;
pub mod fdx;
pub mod scene_headings;
pub mod screenplay_layout;
pub mod line_measure;
pub mod sentence_spacing;
pub mod styled_text;
pub mod action_lines;
pub mod centered_action;
pub mod page_artifact_text;
pub mod page_artifact_decisions;
pub mod model;
pub mod recognition;
pub mod screenplay;

use model::{PdfInspection, PositionedTextItem, PhysicalText};
use recognition::recognize;

#[derive(Debug, Error)]
pub enum FeatherweightError {
    #[error("unable to open pdf: {0}")]
    Pdf(#[from] pdf_oxide::error::Error),
    #[error("invalid page index {0}")]
    InvalidPageIndex(usize),
}

pub fn inspect_screenplay_pdf(pdf_bytes: &[u8]) -> Result<PdfInspection, FeatherweightError> {
    let doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec())?;
    let page_count = doc.page_count()?;
    Ok(PdfInspection {
        page_count,
        pages_needing_ocr: Vec::new(),
    })
}


pub fn screenplay_to_fountain(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
) -> Result<String, FeatherweightError> {
    let mut items = Vec::new();
    let mut doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec())?;
    let page_count = doc.page_count()?;
    let mut source_index = 0usize;
    for &page_index in native_page_indexes {
        if page_index >= page_count { return Err(FeatherweightError::InvalidPageIndex(page_index)); }
        let spans = doc.extract_spans(page_index)?;
        for span in spans.iter() {
            items.push(PositionedTextItem {
                source_index,
                page_index,
                text: span.text.clone(),
                x: span.bbox.x as f64,
                y: span.bbox.y as f64,
                width: span.bbox.width as f64,
                height: span.bbox.height as f64,
                font_name: span.font_name.clone(),
                font_size: span.font_size as f64,
                bold: span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold,
                italic: span.is_italic,
            });
            source_index += 1;
        }
    }
    let physical = model::group_into_physical_lines(items);
    let (title_page, elements) = recognition::recognize(&physical);
    let document = screenplay::ScreenplayDocument { title_page, elements };
    Ok(fountain::screenplay_document_to_fountain(&document))
}


pub fn screenplay_to_fdx(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
) -> Result<String, FeatherweightError> {
    let mut items = Vec::new();
    let mut doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec())?;
    let page_count = doc.page_count()?;
    let mut source_index = 0usize;
    for &page_index in native_page_indexes {
        if page_index >= page_count { return Err(FeatherweightError::InvalidPageIndex(page_index)); }
        let spans = doc.extract_spans(page_index)?;
        for span in spans.iter() {
            items.push(PositionedTextItem {
                source_index,
                page_index,
                text: span.text.clone(),
                x: span.bbox.x as f64,
                y: span.bbox.y as f64,
                width: span.bbox.width as f64,
                height: span.bbox.height as f64,
                font_name: span.font_name.clone(),
                font_size: span.font_size as f64,
                bold: span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold,
                italic: span.is_italic,
            });
            source_index += 1;
        }
    }
    let physical = model::group_into_physical_lines(items);
    let (title_page, elements) = recognition::recognize(&physical);
    let document = screenplay::ScreenplayDocument { title_page, elements };
    Ok(fdx::screenplay_document_to_fdx(&document))
}

pub fn physical_text_json(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
) -> Result<String, FeatherweightError> {
    let mut items = Vec::new();
    let mut doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec())?;
    let page_count = doc.page_count()?;
    let mut source_index = 0usize;
    for &page_index in native_page_indexes {
        if page_index >= page_count {
            return Err(FeatherweightError::InvalidPageIndex(page_index));
        }
        let spans = doc.extract_spans(page_index)?;
        for span in spans.iter() {
            items.push(PositionedTextItem {
                source_index,
                page_index,
                text: span.text.clone(),
                x: span.bbox.x as f64,
                y: span.bbox.y as f64,
                width: span.bbox.width as f64,
                height: span.bbox.height as f64,
                font_name: span.font_name.clone(),
                font_size: span.font_size as f64,
                bold: span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold,
                italic: span.is_italic,
            });
            source_index += 1;
        }
    }
    let physical = model::group_into_physical_lines(items);
    Ok(serde_json::to_string_pretty(&physical).unwrap())
}

pub fn screenplay_to_json(
    pdf_bytes: &[u8],
    native_page_indexes: &[usize],
) -> Result<String, FeatherweightError> {
    let mut items = Vec::new();
    let mut doc = pdf_oxide::PdfDocument::from_bytes(pdf_bytes.to_vec())?;
    let page_count = doc.page_count()?;
    let mut source_index = 0usize;
    for &page_index in native_page_indexes {
        if page_index >= page_count {
            return Err(FeatherweightError::InvalidPageIndex(page_index));
        }
        let spans = doc.extract_spans(page_index)?;
        for span in spans.iter() {
            items.push(PositionedTextItem {
                source_index,
                page_index,
                text: span.text.clone(),
                x: span.bbox.x as f64,
                y: span.bbox.y as f64,
                width: span.bbox.width as f64,
                height: span.bbox.height as f64,
                font_name: span.font_name.clone(),
                font_size: span.font_size as f64,
                bold: span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold,
                italic: span.is_italic,
            });
            source_index += 1;
        }
    }
    let physical = model::group_into_physical_lines(items);
    let (title_page, elements) = recognize(&physical);
    let document = screenplay::ScreenplayDocument {
        title_page,
        elements,
    };
    Ok(serde_json::to_string_pretty(&document).unwrap())
}