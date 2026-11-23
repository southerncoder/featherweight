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

/// The underlying cause of an [`ImportError`], kept for logging via
/// [`std::error::Error::source`] and never shown to users or serialized.
pub type ErrorCause = std::sync::Arc<dyn std::error::Error + Send + Sync>;

/// Errors from [`import_screenplay`](crate::import_screenplay), with messages
/// written for end users.
///
/// Serializes as `{ "code": "NOT_A_PDF", "message": "That file isn't a PDF." }`;
/// the cause is omitted. Classification of `pdf-inspector` failures: an empty or
/// non-`%PDF-` input is [`NotAPdf`](Self::NotAPdf); a PDF whose password is
/// required (or whose security handler is unsupported) is
/// [`Encrypted`](Self::Encrypted); owner-password-only PDFs open without one and
/// convert normally. Every other parse or extraction failure is
/// [`Corrupt`](Self::Corrupt).
#[derive(Debug, Clone, thiserror::Error)]
pub enum ImportError {
    #[error("That file isn't a PDF.")]
    NotAPdf {
        #[source]
        cause: Option<ErrorCause>,
    },
    #[error("That PDF is password-protected. Remove the password and try again.")]
    Encrypted {
        #[source]
        cause: Option<ErrorCause>,
    },
    #[error("That PDF appears to be damaged and couldn't be read.")]
    Corrupt {
        #[source]
        cause: Option<ErrorCause>,
    },
    #[error("That PDF has no readable text. It looks like a scanned document, which can't be imported yet.")]
    NoEmbeddedText,
    #[error("The import was cancelled.")]
    Cancelled,
    #[error("Something went wrong while importing that PDF.")]
    Internal {
        #[source]
        cause: Option<ErrorCause>,
    },
}

impl ImportError {
    /// A stable, machine-readable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotAPdf { .. } => "NOT_A_PDF",
            Self::Encrypted { .. } => "ENCRYPTED",
            Self::Corrupt { .. } => "CORRUPT",
            Self::NoEmbeddedText => "NO_EMBEDDED_TEXT",
            Self::Cancelled => "CANCELLED",
            Self::Internal { .. } => "INTERNAL",
        }
    }

    /// Classifies a `pdf-inspector` failure (see the type docs).
    pub(crate) fn from_pdf_error(error: pdf_inspector::PdfError) -> Self {
        use pdf_inspector::PdfError;

        let classify: fn(Option<ErrorCause>) -> Self = match &error {
            PdfError::NotAPdf(_) => |cause| Self::NotAPdf { cause },
            PdfError::Encrypted => |cause| Self::Encrypted { cause },
            PdfError::Io(_) | PdfError::Parse(_) | PdfError::InvalidStructure => |cause| Self::Corrupt { cause },
        };
        classify(Some(std::sync::Arc::new(error)))
    }

    /// Classifies a boxed cause that may be a `pdf-inspector` failure.
    fn from_cause(cause: Box<dyn std::error::Error + Send + Sync>) -> Self {
        match cause.downcast::<pdf_inspector::PdfError>() {
            Ok(pdf_error) => Self::from_pdf_error(*pdf_error),
            Err(other) => Self::Corrupt {
                cause: Some(other.into()),
            },
        }
    }
}

impl serde::Serialize for ImportError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("ImportError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<PdfInspectionError> for ImportError {
    fn from(error: PdfInspectionError) -> Self {
        Self::from_cause(error.cause)
    }
}

impl From<PdfExtractionError> for ImportError {
    fn from(error: PdfExtractionError) -> Self {
        match error {
            PdfExtractionError::ExtractionFailed(cause) => Self::from_cause(cause),
            PdfExtractionError::ItemTranslationFailed(reason) => Self::Corrupt {
                cause: Some(std::sync::Arc::from(Box::<dyn std::error::Error + Send + Sync>::from(
                    reason,
                ))),
            },
        }
    }
}

/// Page-routing errors cannot come from [`import_screenplay`](crate::import_screenplay),
/// which routes pages itself; they map to [`ImportError::Internal`].
impl From<ScreenplayConversionError> for ImportError {
    fn from(error: ScreenplayConversionError) -> Self {
        match error {
            ScreenplayConversionError::PdfProcessingFailed => Self::Corrupt {
                cause: Some(std::sync::Arc::new(error)),
            },
            _ => Self::Internal {
                cause: Some(std::sync::Arc::new(error)),
            },
        }
    }
}
