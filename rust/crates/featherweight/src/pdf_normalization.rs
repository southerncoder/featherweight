use crate::model::PhysicalTextLine;
use crate::normalized_text::{NormalizedText, NormalizedTextPage};

pub fn normalize_physical_text(lines: &[PhysicalTextLine], represented: &[usize]) -> NormalizedText {
    let mut pages_map = std::collections::BTreeMap::new();
    for &p in represented { pages_map.insert(p, Vec::new()); }
    for l in lines {
        pages_map.entry(l.page_index).or_insert_with(Vec::new).push(l.clone());
    }
    let pages = pages_map.into_iter().map(|(pi, ls)| NormalizedTextPage {
        page_index: pi,
        lines: ls,
    }).collect();
    NormalizedText { pages }
}