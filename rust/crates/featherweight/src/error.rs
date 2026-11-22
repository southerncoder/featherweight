//! Error types. Port of `screenplay-conversion-error.ts`, `inspection-error.ts`
//! and `extraction/errors.ts`; codes and messages match the TypeScript port.

/// Port of `ScreenplayConversionError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScreenplayConversionError {
    #[error("Page routing contains an invalid page index.")]
    InvalidPageIndex,
    #[error("Page routing contains a duplicate page index.")]
    DuplicatePageIndex,
    #[error("Native and OCR page routes overlap.")]
    OverlappingPageIndex,
    #[error("PDF could not be converted.")]
    PdfProcessingFailed,
    #[error("Screenplay text cannot be represented in FDX XML.")]
    InvalidFdxText,
}

impl ScreenplayConversionError {
    /// The stable error code used by the TypeScript API.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidPageIndex => "INVALID_PAGE_INDEX",
            Self::DuplicatePageIndex => "DUPLICATE_PAGE_INDEX",
            Self::OverlappingPageIndex => "OVERLAPPING_PAGE_INDEX",
            Self::PdfProcessingFailed => "PDF_PROCESSING_FAILED",
            Self::InvalidFdxText => "INVALID_FDX_TEXT",
        }
    }
}

/// Port of `PdfInspectionError` (`PDF_INSPECTION_FAILED`). The binding is linked
/// statically, so `PDF_BINDING_UNAVAILABLE` cannot occur in Rust.
#[derive(Debug, thiserror::Error)]
#[error("PDF inspection failed.")]
pub struct PdfInspectionError {
    #[source]
    pub cause: Box<dyn std::error::Error + Send + Sync>,
}

impl PdfInspectionError {
    pub fn code(&self) -> &'static str {
        "PDF_INSPECTION_FAILED"
    }
}

/// Port of `PdfExtractionError`.
#[derive(Debug, thiserror::Error)]
pub enum PdfExtractionError {
    #[error("PDF extraction failed.")]
    ExtractionFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("PDF item translation failed.")]
    ItemTranslationFailed(&'static str),
}
