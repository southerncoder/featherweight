use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PdfInspection {
    pub page_count: usize,
    pub pages_needing_ocr: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionedTextItem {
    pub source_index: usize,
    pub page_index: usize,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_name: String,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalTextSpan {
    pub start: usize,
    pub end: usize,
    pub source_index: usize,
    pub page_index: usize,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalTextLine {
    pub page_index: usize,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub spans: Vec<PhysicalTextSpan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalText {
    pub lines: Vec<PhysicalTextLine>,
}

pub fn group_into_physical_lines(items: Vec<PositionedTextItem>) -> PhysicalText {
    let mut baselines: Vec<Vec<PositionedTextItem>> = Vec::new();
    for item in items {
        let mut found = None;
        for (i, baseline) in baselines.iter().enumerate() {
            if baseline.iter().any(|e| {
                e.page_index == item.page_index
                    && (e.y - item.y).abs()
                        <= (e.height.min(item.height).min(e.font_size.min(item.font_size)) / 4.0)
            }) {
                found = Some(i);
                break;
            }
        }
        if let Some(i) = found {
            baselines[i].push(item);
        } else {
            baselines.push(vec![item]);
        }
    }

    let mut connected: Vec<Vec<PositionedTextItem>> = Vec::new();
    for baseline in baselines {
        let mut sorted = baseline.clone();
        sorted.sort_by(|a, b| {
            a.x.partial_cmp(&b.x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.source_index.cmp(&b.source_index))
        });
        let mut lines: Vec<Vec<PositionedTextItem>> = Vec::new();
        for item in sorted {
            if let Some(last) = lines.last_mut() {
                let prev = last.last().unwrap();
                let gap = item.x - (prev.x + prev.width);
                let scale = prev.height.max(item.height).max(prev.font_size).max(item.font_size);
                if gap <= scale {
                    last.push(item);
                    continue;
                }
            }
            lines.push(vec![item]);
        }
        connected.extend(lines);
    }

    connected.sort_by_key(|line| line.iter().map(|i| i.source_index).min().unwrap_or(0));

    let mut out_lines = Vec::new();
    for line in connected {
        if line.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut spans = Vec::new();
        for (idx, item) in line.iter().enumerate() {
            if idx > 0 {
                let prev = &line[idx - 1];
                let mut need_space = !(prev.text.ends_with(' ') || item.text.starts_with(' '))
                    && !(item.text.starts_with(',')
                        || item.text.starts_with('.')
                        || item.text.starts_with(';')
                        || item.text.starts_with(':')
                        || item.text.starts_with('!')
                        || item.text.starts_with('?')
                        || prev.text.ends_with('-'));
                if need_space {
                    let gap = item.x - (prev.x + prev.width);
                    let left_cw = prev.width / (prev.text.chars().count().max(1) as f64);
                    let right_cw = item.width / (item.text.chars().count().max(1) as f64);
                    let local = left_cw.min(right_cw);
                    if gap >= local / 2.0 {
                        text.push(' ');
                    } else {
                        need_space = false;
                    }
                }
                let _ = need_space;
            }
            let start = text.chars().count();
            text.push_str(&item.text);
            let end = text.chars().count();
            spans.push(PhysicalTextSpan {
                start,
                end,
                source_index: item.source_index,
                page_index: item.page_index,
                text: item.text.clone(),
                x: item.x,
                y: item.y,
                width: item.width,
                height: item.height,
                bold: item.bold,
                italic: item.italic,
            });
        }
        let x = line.iter().map(|i| i.x).fold(f64::INFINITY, f64::min);
        let y = line.iter().map(|i| i.y).fold(f64::INFINITY, f64::min);
        let right = line.iter().map(|i| i.x + i.width).fold(f64::NEG_INFINITY, f64::max);
        let bottom = line.iter().map(|i| i.y + i.height).fold(f64::NEG_INFINITY, f64::max);
        out_lines.push(PhysicalTextLine {
            page_index: line[0].page_index,
            text,
            x,
            y,
            width: right - x,
            height: bottom - y,
            spans,
        });
    }
    PhysicalText { lines: out_lines }
}