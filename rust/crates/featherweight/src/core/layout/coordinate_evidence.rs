//! Port of `src/core/layout/coordinate-evidence.ts`.
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::core::physical_lines::PhysicalTextLine;

/// `CoordinateEvidence`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateEvidence {
    pub x: Option<f64>,
    pub conflicting: bool,
}

/// `PageCoordinateObservation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageCoordinateObservation {
    pub x: f64,
    pub page_index: usize,
}

/// `CoordinateCluster` (module-private in TypeScript, but returned by the
/// exported `clusterCoordinates`).
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateCluster {
    pub values: Vec<f64>,
}

/// `resolveCoordinateEvidence`.
pub fn resolve_coordinate_evidence(values: &[f64], coordinate_tolerance: f64) -> CoordinateEvidence {
    let mut clusters = cluster_coordinates(values, coordinate_tolerance);
    clusters.sort_by(|left, right| {
        right
            .values
            .len()
            .cmp(&left.values.len())
            .then_with(|| js_numeric_order(average(&left.values) - average(&right.values)))
    });

    let Some(dominant_cluster) = clusters.first() else {
        return CoordinateEvidence {
            x: None,
            conflicting: false,
        };
    };

    let competing_cluster = clusters.get(1);

    if let Some(competing) = competing_cluster
        && competing.values.len() == dominant_cluster.values.len()
    {
        return CoordinateEvidence {
            x: None,
            conflicting: true,
        };
    }

    let conflicting = competing_cluster.is_some_and(|competing| {
        competing.values.len() >= 3 && competing.values.len() as f64 / dominant_cluster.values.len() as f64 >= 0.4
    });

    CoordinateEvidence {
        x: Some(average(&dominant_cluster.values)),
        conflicting,
    }
}

/// `resolvePageSupportedCoordinateEvidence`.
pub fn resolve_page_supported_coordinate_evidence(
    observations: &[PageCoordinateObservation],
    coordinate_tolerance: f64,
) -> CoordinateEvidence {
    let mut clusters = cluster_page_coordinates(observations, coordinate_tolerance);
    clusters.sort_by(|left, right| {
        count_pages(right)
            .cmp(&count_pages(left))
            .then_with(|| right.len().cmp(&left.len()))
            .then_with(|| js_numeric_order(observation_average(left) - observation_average(right)))
    });

    let Some(dominant_cluster) = clusters.first() else {
        return CoordinateEvidence {
            x: None,
            conflicting: false,
        };
    };

    let dominant_x = observation_average(dominant_cluster);

    let Some(competing_cluster) = clusters.get(1) else {
        return CoordinateEvidence {
            x: Some(dominant_x),
            conflicting: false,
        };
    };

    let tied = count_pages(competing_cluster) == count_pages(dominant_cluster)
        && competing_cluster.len() == dominant_cluster.len();

    if tied {
        return CoordinateEvidence {
            x: None,
            conflicting: true,
        };
    }

    let conflicting =
        competing_cluster.len() >= 3 && competing_cluster.len() as f64 / dominant_cluster.len() as f64 >= 0.4;

    CoordinateEvidence {
        x: Some(dominant_x),
        conflicting,
    }
}

/// `clusterCoordinates`: sorts ascending, then starts a new cluster whenever a
/// value is more than `coordinate_tolerance` past the previous cluster's last
/// value.
pub fn cluster_coordinates(values: &[f64], coordinate_tolerance: f64) -> Vec<CoordinateCluster> {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(|left, right| js_numeric_order(left - right));
    let mut clusters: Vec<CoordinateCluster> = Vec::new();

    for value in sorted_values {
        let starts_new_cluster = clusters.last().is_none_or(|current_cluster| {
            value - current_cluster.values.last().expect("clusters are never empty") > coordinate_tolerance
        });

        if starts_new_cluster {
            clusters.push(CoordinateCluster { values: vec![value] });
        } else if let Some(current_cluster) = clusters.last_mut() {
            current_cluster.values.push(value);
        }
    }

    clusters
}

/// `clusterPageCoordinates`.
fn cluster_page_coordinates(
    observations: &[PageCoordinateObservation],
    coordinate_tolerance: f64,
) -> Vec<Vec<PageCoordinateObservation>> {
    let mut sorted_observations = observations.to_vec();
    sorted_observations.sort_by(|left, right| js_numeric_order(left.x - right.x));
    let mut clusters: Vec<Vec<PageCoordinateObservation>> = Vec::new();

    for observation in sorted_observations {
        let starts_new_cluster = clusters.last().is_none_or(|current_cluster| {
            observation.x - current_cluster.last().expect("clusters are never empty").x > coordinate_tolerance
        });

        if starts_new_cluster {
            clusters.push(vec![observation]);
        } else if let Some(current_cluster) = clusters.last_mut() {
            current_cluster.push(observation);
        }
    }

    clusters
}

/// `countPages`.
fn count_pages(observations: &[PageCoordinateObservation]) -> usize {
    observations
        .iter()
        .map(|observation| observation.page_index)
        .collect::<HashSet<_>>()
        .len()
}

/// `inferCoordinateTolerance`.
pub fn infer_coordinate_tolerance(lines: &[&PhysicalTextLine]) -> f64 {
    if lines.is_empty() {
        return 1.0;
    }

    median(&lines.iter().map(|line| line.bounds.height).collect::<Vec<_>>()) / 4.0
}

/// `median`. An empty input yields NaN, as in TypeScript
/// (`(undefined + undefined) / 2`).
pub fn median(values: &[f64]) -> f64 {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(|left, right| js_numeric_order(left - right));
    let middle_index = sorted_values.len() / 2;

    if sorted_values.len() % 2 == 1 {
        return sorted_values[middle_index];
    }

    if sorted_values.is_empty() {
        return f64::NAN;
    }

    (sorted_values[middle_index - 1] + sorted_values[middle_index]) / 2.0
}

/// `average`: `reduce((sum, value) => sum + value, 0) / length`.
fn average(values: &[f64]) -> f64 {
    values.iter().fold(0.0, |sum, value| sum + value) / values.len() as f64
}

fn observation_average(observations: &[PageCoordinateObservation]) -> f64 {
    average(&observations.iter().map(|observation| observation.x).collect::<Vec<_>>())
}

/// Interprets a JavaScript numeric comparator result: negative orders first,
/// positive orders second, and zero or NaN keeps the pair as is.
pub(crate) fn js_numeric_order(difference: f64) -> Ordering {
    if difference < 0.0 {
        Ordering::Less
    } else if difference > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_sorted_values_by_tolerance() {
        let clusters = cluster_coordinates(&[10.0, 1.0, 2.0, 11.5, 30.0], 1.5);
        let values: Vec<Vec<f64>> = clusters.into_iter().map(|cluster| cluster.values).collect();
        assert_eq!(values, vec![vec![1.0, 2.0], vec![10.0, 11.5], vec![30.0]]);
    }

    #[test]
    fn resolves_dominant_cluster_and_conflicts() {
        let evidence = resolve_coordinate_evidence(&[100.0, 100.0, 100.0, 200.0], 2.0);
        assert_eq!(
            evidence,
            CoordinateEvidence {
                x: Some(100.0),
                conflicting: false
            }
        );

        let tied = resolve_coordinate_evidence(&[100.0, 200.0], 2.0);
        assert_eq!(
            tied,
            CoordinateEvidence {
                x: None,
                conflicting: true
            }
        );

        let conflicting =
            resolve_coordinate_evidence(&[100.0; 5].iter().chain(&[200.0; 3]).copied().collect::<Vec<_>>(), 2.0);
        assert_eq!(
            conflicting,
            CoordinateEvidence {
                x: Some(100.0),
                conflicting: true
            }
        );

        assert_eq!(
            resolve_coordinate_evidence(&[], 2.0),
            CoordinateEvidence {
                x: None,
                conflicting: false
            }
        );
    }

    #[test]
    fn page_supported_evidence_prefers_page_count() {
        let observations = [
            PageCoordinateObservation { x: 50.0, page_index: 0 },
            PageCoordinateObservation { x: 50.0, page_index: 0 },
            PageCoordinateObservation { x: 50.0, page_index: 0 },
            PageCoordinateObservation { x: 90.0, page_index: 1 },
            PageCoordinateObservation { x: 90.0, page_index: 2 },
        ];
        let evidence = resolve_page_supported_coordinate_evidence(&observations, 2.0);
        assert_eq!(
            evidence,
            CoordinateEvidence {
                x: Some(90.0),
                conflicting: true
            }
        );
    }

    #[test]
    fn median_matches_javascript() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert!(median(&[]).is_nan());
    }
}
