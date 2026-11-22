//! Port of `src/pdf/normalization/recurring-artifacts.ts`.
use std::collections::{HashMap, HashSet};

use crate::core::normalized_text::SuppressionReason;
use crate::core::page_extents::{PageExtentMeasurements, is_line_in_page_margin_band};
use crate::core::physical_lines::PhysicalTextLine;
use crate::normalization::page_artifact_decisions::{
    ArtifactDiagnostics, SuppressionReasons, add_artifact_diagnostic, add_suppression_reason,
};
use crate::normalization::page_artifact_text::{TextFragment, create_text_fragments, is_explicit_pagination_text};
use crate::core::text::js_len;

/// Port of `detectRecurringArtifacts`.
pub fn detect_recurring_artifacts(
    lines: &[PhysicalTextLine],
    geometry: &PageExtentMeasurements,
    suppression_reasons: &mut SuppressionReasons,
    diagnostics: &mut ArtifactDiagnostics,
) {
    let document_page_count = lines.iter().map(|line| line.page_index).collect::<HashSet<_>>().len();
    let fragments_by_text = group_by(
        lines
            .iter()
            .enumerate()
            .flat_map(|(line_index, line)| create_text_fragments(line_index, line))
            .filter(is_recurring_artifact_fragment),
        |fragment| fragment.text.clone(),
    );

    for fragments in &fragments_by_text {
        let mut clusters: Vec<Vec<&TextFragment<'_>>> = Vec::new();

        for fragment in fragments {
            match clusters.iter_mut().find(|candidate| have_stable_presentation(candidate[0], fragment)) {
                Some(cluster) => cluster.push(fragment),
                None => clusters.push(vec![fragment]),
            }
        }

        for cluster in &clusters {
            let fragments_by_page = group_by(cluster.iter().copied(), |fragment| fragment.line.page_index);
            let unambiguous_occurrences: Vec<&TextFragment<'_>> = fragments_by_page
                .iter()
                .map(|page_fragments| distinct_by_line(page_fragments))
                .filter(|page_fragments| page_fragments.len() == 1)
                .map(|page_fragments| page_fragments[0])
                .collect();
            let covers_document_majority =
                unambiguous_occurrences.len() as f64 / document_page_count as f64 > 0.5;

            if unambiguous_occurrences.len() < 3 || !covers_document_majority {
                continue;
            }

            for fragment in unambiguous_occurrences {
                if !is_line_in_page_margin_band(fragment.line, geometry) {
                    add_artifact_diagnostic(
                        diagnostics,
                        fragment.line_index,
                        fragment.line,
                        SuppressionReason::RecurringPageArtifact,
                    );
                    continue;
                }

                add_suppression_reason(
                    suppression_reasons,
                    fragment.line_index,
                    SuppressionReason::RecurringPageArtifact,
                );
            }
        }
    }
}

/// `Map.groupBy(items, key).values()`: groups in first-seen key order.
fn group_by<T, K: std::hash::Hash + Eq>(items: impl IntoIterator<Item = T>, key: impl Fn(&T) -> K) -> Vec<Vec<T>> {
    let mut group_positions: HashMap<K, usize> = HashMap::new();
    let mut groups: Vec<Vec<T>> = Vec::new();

    for item in items {
        match group_positions.get(&key(&item)) {
            Some(&position) => groups[position].push(item),
            None => {
                group_positions.insert(key(&item), groups.len());
                groups.push(vec![item]);
            }
        }
    }

    groups
}

/// `[...new Map(fragments.map((fragment) => [fragment.line, fragment])).values()]`:
/// one entry per line in first-seen order, holding that line's last fragment.
fn distinct_by_line<'f, 'a>(fragments: &[&'f TextFragment<'a>]) -> Vec<&'f TextFragment<'a>> {
    let mut distinct: Vec<&TextFragment<'a>> = Vec::new();

    for &fragment in fragments {
        match distinct.iter_mut().find(|existing| existing.line_index == fragment.line_index) {
            Some(existing) => *existing = fragment,
            None => distinct.push(fragment),
        }
    }

    distinct
}

fn is_recurring_artifact_fragment(fragment: &TextFragment<'_>) -> bool {
    if js_len(&fragment.text) < 5 {
        return false;
    }

    !is_explicit_pagination_text(&fragment.text)
}

fn have_stable_presentation(left: &TextFragment<'_>, right: &TextFragment<'_>) -> bool {
    let position_tolerance = left.font_size.max(right.font_size) / 2.0;

    left.font_family == right.font_family
        && left.style == right.style
        && (left.font_size - right.font_size).abs() <= 0.25
        && (left.x - right.x).abs() <= position_tolerance
        && (left.y - right.y).abs() <= position_tolerance
}
