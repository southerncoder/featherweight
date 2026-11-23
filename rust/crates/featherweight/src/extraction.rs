//! Port of `src/pdf/extraction/pdf-inspector.ts` and `src/pdf/inspection.ts`.
//!
//! Uses the `pdf-inspector` crate the TypeScript port reaches through its napi
//! binding, pinned to the same build, so extracted items match exactly.
use std::collections::HashSet;

use pdf_inspector::TextItem;
use pdf_inspector::types::ItemType;

use crate::core::positioned_text::{
    PositionedText, PositionedTextBounds, PositionedTextFont, PositionedTextItem, PositionedTextStyle, SourceMethod,
};
use crate::error::{PdfExtractionError, PdfInspectionError};

/// Port of `PdfInspection`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PdfInspection {
    pub page_count: usize,
    pub pages_needing_ocr: Vec<usize>,
}

impl PdfInspection {
    /// The default page routes: `(native, ocr)`, with every page that has
    /// usable embedded text routed natively and the rest to OCR. Both ascending.
    pub fn page_routes(&self) -> (Vec<usize>, Vec<usize>) {
        (0..self.page_count).partition(|page_index| !self.pages_needing_ocr.contains(page_index))
    }
}

/// Port of `inspectScreenplayPdf`: physical page count and the 0-based pages
/// that have no usable embedded text.
pub fn inspect_screenplay_pdf(pdf_bytes: &[u8]) -> Result<PdfInspection, PdfInspectionError> {
    let classification =
        pdf_inspector::classify_pdf_mem(pdf_bytes).map_err(|cause| PdfInspectionError { cause: Box::new(cause) })?;
    let page_count = classification.page_count as usize;
    let mut seen = HashSet::new();

    for &page_index in &classification.pages_needing_ocr {
        if page_index as usize >= page_count || !seen.insert(page_index) {
            return Err(PdfInspectionError {
                cause: "Native PDF classification has invalid OCR pages.".into(),
            });
        }
    }

    Ok(PdfInspection {
        page_count,
        pages_needing_ocr: classification.pages_needing_ocr.iter().map(|&p| p as usize).collect(),
    })
}

/// Port of `extractPositionedPdfText`. `page_indexes` are 0-based; `None`
/// extracts every page.
pub fn extract_positioned_pdf_text(
    pdf_bytes: &[u8],
    page_indexes: Option<&[usize]>,
) -> Result<PositionedText, PdfExtractionError> {
    let page_filter: Option<HashSet<u32>> =
        page_indexes.map(|indexes| indexes.iter().map(|&index| index as u32 + 1).collect());
    let native_items = pdf_inspector::extractor::extract_text_with_positions_mem_pages(pdf_bytes, page_filter.as_ref())
        .map_err(|cause| PdfExtractionError::ExtractionFailed(Box::new(cause)))?;

    translate_pdf_inspector_text_items(&native_items)
}

/// Port of `translatePdfInspectorTextItems`. `sourceIndex` counts every native
/// item, including the non-text items that are skipped.
pub fn translate_pdf_inspector_text_items(items: &[TextItem]) -> Result<PositionedText, PdfExtractionError> {
    let mut translated = Vec::with_capacity(items.len());

    for (source_index, item) in items.iter().enumerate() {
        if !matches!(item.item_type, ItemType::Text) {
            continue;
        }

        assert_valid_text_item(item)?;
        translated.push(PositionedTextItem {
            source_index,
            source_method: SourceMethod::EmbeddedText,
            page_index: item.page as usize - 1,
            text: item.text.clone(),
            bounds: PositionedTextBounds {
                x: item.x as f64,
                y: item.y as f64,
                width: item.width as f64,
                height: item.height as f64,
            },
            font: PositionedTextFont {
                name: item.font.clone(),
                size: item.font_size as f64,
            },
            style: PositionedTextStyle {
                bold: item.is_bold,
                italic: item.is_italic,
                underline: item.is_underline,
                strikeout: item.is_strikeout,
            },
        });
    }

    Ok(PositionedText { items: translated })
}

fn assert_valid_text_item(item: &TextItem) -> Result<(), PdfExtractionError> {
    if item.page < 1 {
        return Err(PdfExtractionError::ItemTranslationFailed(
            "Native PDF text item has an invalid page.",
        ));
    }

    let numbers = [item.x, item.y, item.width, item.height, item.font_size];
    if numbers.iter().any(|value| !value.is_finite()) {
        return Err(PdfExtractionError::ItemTranslationFailed(
            "Native PDF text item has invalid numeric metadata.",
        ));
    }

    Ok(())
}
