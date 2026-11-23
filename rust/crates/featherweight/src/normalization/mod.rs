//! Port of `src/pdf/normalization`.
//!
//! The TypeScript keys its decision maps by `PhysicalTextLine` object identity.
//! Here a line is identified by its index in `PhysicalText::lines`, and the maps
//! are dense vectors indexed the same way (see [`page_artifact_decisions`]).
pub mod page_artifact_decisions;
pub mod page_artifact_text;
pub mod page_artifacts;
pub mod physical_text;
pub mod recurring_artifacts;
pub mod screenplay_pagination;
pub mod source_page_numbers;
