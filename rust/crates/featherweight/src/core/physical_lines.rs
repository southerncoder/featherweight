//! Port of `src/core/physical-lines.ts`.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::positioned_text::{
    PositionedText, PositionedTextBounds, PositionedTextFont, PositionedTextItem, PositionedTextStyle, SourceMethod,
};
use crate::core::text::{is_js_whitespace, js_len};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalText {
    pub lines: Vec<PhysicalTextLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalTextLine {
    pub page_index: usize,
    pub text: String,
    pub bounds: PositionedTextBounds,
    pub spans: Vec<PhysicalTextSpan>,
    /// Leading whitespace characters moved from the text into `bounds.x`, when
    /// the PDF indents with spaces instead of positioning text. `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indentation: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalTextSpan {
    /// UTF-8 byte offset into [`PhysicalTextLine::text`] (TypeScript: UTF-16 offset).
    pub start: usize,
    /// UTF-8 byte offset into [`PhysicalTextLine::text`] (TypeScript: UTF-16 offset).
    pub end: usize,
    pub source_index: usize,
    pub source_method: SourceMethod,
    pub bounds: PositionedTextBounds,
    pub font: PositionedTextFont,
    pub style: PositionedTextStyle,
}

pub fn group_positioned_text_into_physical_lines(positioned_text: &PositionedText) -> PhysicalText {
    let mut connected_lines: Vec<(Vec<&PositionedTextItem>, usize)> = group_items_by_baseline(&positioned_text.items)
        .into_iter()
        .flat_map(split_baseline_by_horizontal_connectivity)
        .collect();
    // `Array.prototype.sort` is stable, as is `sort_by_key`.
    connected_lines.sort_by_key(|(_, first_source_index)| *first_source_index);
    let space_widths = estimate_space_widths(&positioned_text.items);

    PhysicalText {
        lines: connected_lines
            .iter()
            .map(|(items, _)| create_physical_text_line(items, &space_widths))
            .collect(),
    }
}

fn group_items_by_baseline(items: &[PositionedTextItem]) -> Vec<Vec<&PositionedTextItem>> {
    let mut baselines: Vec<Vec<&PositionedTextItem>> = Vec::new();

    for item in items {
        let compatible = baselines.iter_mut().find(|baseline| {
            baseline
                .iter()
                .any(|existing| have_compatible_baselines(existing, item))
        });

        match compatible {
            Some(baseline) => baseline.push(item),
            None => baselines.push(vec![item]),
        }
    }

    baselines
}

fn have_compatible_baselines(left: &PositionedTextItem, right: &PositionedTextItem) -> bool {
    if left.page_index != right.page_index {
        return false;
    }

    let local_vertical_scale = left
        .bounds
        .height
        .min(right.bounds.height)
        .min(left.font.size)
        .min(right.font.size);
    let baseline_tolerance = local_vertical_scale / 4.0;

    (left.bounds.y - right.bounds.y).abs() <= baseline_tolerance
}

fn split_baseline_by_horizontal_connectivity(
    items: Vec<&PositionedTextItem>,
) -> Vec<(Vec<&PositionedTextItem>, usize)> {
    let mut sorted_items = items;
    sorted_items.sort_by(|left, right| {
        left.bounds
            .x
            .partial_cmp(&right.bounds.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.source_index.cmp(&right.source_index))
    });
    let mut connected_lines: Vec<Vec<&PositionedTextItem>> = Vec::new();

    for item in sorted_items {
        if let Some(current_line) = connected_lines.last_mut() {
            // invariant: every line is created with one item and only grows.
            let previous_item = *current_line.last().expect("lines are never empty");
            if are_horizontally_connected(previous_item, item) {
                current_line.push(item);
                continue;
            }
        }
        connected_lines.push(vec![item]);
    }

    connected_lines
        .into_iter()
        .map(|line_items| {
            let first_source_index = line_items.iter().map(|item| item.source_index).min().unwrap_or(0);
            (line_items, first_source_index)
        })
        .collect()
}

fn are_horizontally_connected(left: &PositionedTextItem, right: &PositionedTextItem) -> bool {
    let horizontal_gap = right.bounds.x - (left.bounds.x + left.bounds.width);
    let local_run_scale = left
        .bounds
        .height
        .max(right.bounds.height)
        .max(left.font.size)
        .max(right.font.size);

    horizontal_gap <= local_run_scale
}

fn create_physical_text_line(items: &[&PositionedTextItem], space_widths: &SpaceWidths) -> PhysicalTextLine {
    let mut text = String::new();
    let mut spans = Vec::with_capacity(items.len());

    for (item_index, item) in items.iter().enumerate() {
        if item_index > 0 && has_geometric_word_boundary(items[item_index - 1], item) {
            text.push(' ');
        }

        // The line's leading indentation is position, not text.
        let item_text = if item_index == 0 {
            &item.text[leading_indentation(item).1..]
        } else {
            item.text.as_str()
        };
        let start = text.len();
        text.push_str(item_text);

        spans.push(PhysicalTextSpan {
            start,
            end: text.len(),
            source_index: item.source_index,
            source_method: item.source_method,
            bounds: item.bounds,
            font: item.font.clone(),
            style: item.style,
        });
    }

    let (indentation, _) = leading_indentation(items[0]);

    PhysicalTextLine {
        page_index: items[0].page_index,
        text,
        bounds: create_aggregate_bounds(items, space_widths),
        spans,
        indentation: (indentation > 0).then_some(indentation),
    }
}

fn has_geometric_word_boundary(left: &PositionedTextItem, right: &PositionedTextItem) -> bool {
    if left.text.ends_with(is_js_whitespace) || right.text.starts_with(is_js_whitespace) {
        return false;
    }

    if right.text.starts_with([',', '.', ';', ':', '!', '?', '\u{2026}', ')']) || left.text.ends_with('-') {
        return false;
    }

    let horizontal_gap = right.bounds.x - (left.bounds.x + left.bounds.width);
    let left_character_width = left.bounds.width / js_len(&left.text).max(1) as f64;
    let right_character_width = right.bounds.width / js_len(&right.text).max(1) as f64;
    let local_character_width = left_character_width.min(right_character_width);

    horizontal_gap >= local_character_width / 2.0
}

fn create_aggregate_bounds(items: &[&PositionedTextItem], space_widths: &SpaceWidths) -> PositionedTextBounds {
    let first = items[0].bounds;
    let mut minimum_x = visible_start_x(items[0], space_widths);
    let mut minimum_y = first.y;
    let mut maximum_x = visible_end_x(items[0], space_widths);
    let mut maximum_y = first.y + first.height;

    for item in &items[1..] {
        minimum_x = minimum_x.min(item.bounds.x);
        minimum_y = minimum_y.min(item.bounds.y);
        maximum_x = maximum_x.max(visible_end_x(item, space_widths));
        maximum_y = maximum_y.max(item.bounds.y + item.bounds.height);
    }

    PositionedTextBounds {
        x: minimum_x,
        y: minimum_y,
        width: maximum_x - minimum_x,
        height: maximum_y - minimum_y,
    }
}

/// Space width per font, keyed like the TypeScript `fontKey` (name and size).
type SpaceWidths = HashMap<(String, u64), f64>;

fn font_key(item: &PositionedTextItem) -> (String, u64) {
    (item.font.name.clone(), item.font.size.to_bits())
}

/// Port of `leadingIndentation`: some PDFs indent with literal spaces instead
/// of positioning text, so every line starts at the same x and only the leading
/// spaces show where it sits. Returns the indentation as (UTF-16 units, UTF-8
/// bytes): all leading whitespace, unless the item is whitespace only.
fn leading_indentation(item: &PositionedTextItem) -> (usize, usize) {
    let indentation = &item.text[..item.text.len() - item.text.trim_start_matches(is_js_whitespace).len()];
    if indentation.len() == item.text.len() {
        (0, 0)
    } else {
        (js_len(indentation), indentation.len())
    }
}

/// Port of `visibleStartX`: where an item's visible text starts, past its
/// leading indentation, measured in its font's space width.
fn visible_start_x(item: &PositionedTextItem, space_widths: &SpaceWidths) -> f64 {
    let (indentation, _) = leading_indentation(item);

    if indentation == 0 {
        return item.bounds.x;
    }

    item.bounds.x + indentation as f64 * space_width(item, space_widths)
}

/// Port of `visibleEndX`: where an item's visible text ends, before its
/// trailing whitespace (line-end padding in text-formatted PDFs), measured in
/// its font's space width.
fn visible_end_x(item: &PositionedTextItem, space_widths: &SpaceWidths) -> f64 {
    let end = item.bounds.x + item.bounds.width;
    // Every JavaScript whitespace character is one UTF-16 code unit.
    let trailing = item.text.chars().rev().take_while(|&c| is_js_whitespace(c)).count();

    if trailing == 0 || trailing == js_len(&item.text) {
        return end;
    }

    end - trailing as f64 * space_width(item, space_widths)
}

/// Port of `spaceWidth`: the font's estimated space width, else the item's
/// average character width.
fn space_width(item: &PositionedTextItem, space_widths: &SpaceWidths) -> f64 {
    space_widths
        .get(&font_key(item))
        .copied()
        .unwrap_or_else(|| item.bounds.width / js_len(&item.text) as f64)
}

/// Port of `estimateSpaceWidths`: PDFs report only whole-item widths, so fit
/// `width = space * whitespace + glyph * other` by least squares over every item
/// in a font (exact for monospace fonts). Fonts without a usable fit are left
/// out; callers fall back to the item's average character width.
fn estimate_space_widths(items: &[PositionedTextItem]) -> SpaceWidths {
    #[derive(Default)]
    struct Sums {
        aa: f64,
        ab: f64,
        bb: f64,
        aw: f64,
        bw: f64,
    }

    let mut sums: HashMap<(String, u64), Sums> = HashMap::new();

    for item in items {
        let whitespace = item.text.chars().filter(|&c| is_js_whitespace(c)).count() as f64;
        let other = js_len(&item.text) as f64 - whitespace;
        let width = item.bounds.width;
        let sum = sums.entry(font_key(item)).or_default();

        sum.aa += whitespace * whitespace;
        sum.ab += whitespace * other;
        sum.bb += other * other;
        sum.aw += whitespace * width;
        sum.bw += other * width;
    }

    sums.into_iter()
        .filter_map(|(key, sum)| {
            let determinant = sum.aa * sum.bb - sum.ab * sum.ab;
            if determinant <= 0.0 {
                return None;
            }
            let space_width = (sum.aw * sum.bb - sum.ab * sum.bw) / determinant;
            (space_width.is_finite() && space_width > 0.0).then_some((key, space_width))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A proportional font whose space is 3 wide and other glyphs 6 wide.
    fn space_indented(source_index: usize, text: &str, y: f64) -> PositionedTextItem {
        let spaces = text.chars().filter(|&c| is_js_whitespace(c)).count() as f64;
        PositionedTextItem {
            source_index,
            source_method: SourceMethod::EmbeddedText,
            page_index: 0,
            text: text.to_string(),
            bounds: PositionedTextBounds {
                x: 36.0,
                y,
                width: spaces * 3.0 + (js_len(text) as f64 - spaces) * 6.0,
                height: 12.0,
            },
            font: PositionedTextFont {
                name: "Proportional".to_string(),
                size: 12.0,
            },
            style: PositionedTextStyle::default(),
        }
    }

    #[test]
    fn moves_leading_indentation_into_position_using_the_fonts_space_width() {
        let physical = group_positioned_text_into_physical_lines(&PositionedText {
            items: vec![
                space_indented(0, "INT. HOUSE - DAY", 700.0),
                space_indented(1, "                                  MR. BROWN ", 680.0),
                space_indented(2, "                      Like a Virgin.  ", 668.0),
            ],
        });
        let lines: Vec<(&str, f64)> = physical.lines.iter().map(|l| (l.text.as_str(), l.bounds.x)).collect();

        assert_eq!(
            lines,
            [
                ("INT. HOUSE - DAY", 36.0),
                ("MR. BROWN ", 36.0 + 34.0 * 3.0),
                ("Like a Virgin.  ", 36.0 + 22.0 * 3.0)
            ]
        );
        assert_eq!(
            (physical.lines[1].spans[0].start, physical.lines[1].spans[0].end),
            (0, "MR. BROWN ".len())
        );
    }

    #[test]
    fn leaves_whitespace_only_items_unchanged() {
        let physical = group_positioned_text_into_physical_lines(&PositionedText {
            items: vec![space_indented(0, "   ", 700.0)],
        });
        assert_eq!(
            (physical.lines[0].text.as_str(), physical.lines[0].bounds.x),
            ("   ", 36.0)
        );
    }
}
