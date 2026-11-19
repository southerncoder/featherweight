use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct NormalizedTextPage {
    pub page_index: usize,
    pub lines: Vec<PhysicalTextLine>,
}

#[derive(Debug, Clone)]
pub struct NormalizedText {
    pub pages: Vec<NormalizedTextPage>,
}