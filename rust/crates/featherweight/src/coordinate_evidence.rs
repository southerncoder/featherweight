use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct CoordinateEvidence {
    pub x: Option<f64>,
    pub conflicting: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PageCoordinateObservation {
    pub x: f64,
    pub page_index: usize,
}

#[derive(Debug, Clone)]
pub struct CoordinateCluster {
    pub values: Vec<f64>,
}

pub fn cluster_coordinates(values: &[f64], tolerance: f64) -> Vec<CoordinateCluster> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut clusters: Vec<CoordinateCluster> = Vec::new();
    for v in sorted {
        let current = clusters.last();
        if current.is_none() || v - current.unwrap().values.last().copied().unwrap() > tolerance {
            clusters.push(CoordinateCluster { values: vec![v] });
        } else {
            clusters.last_mut().unwrap().values.push(v);
        }
    }
    clusters
}

pub fn cluster_page_coordinates(observations: &[PageCoordinateObservation], tolerance: f64) -> Vec<Vec<PageCoordinateObservation>> {
    let mut sorted = observations.to_vec();
    sorted.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    let mut clusters: Vec<Vec<PageCoordinateObservation>> = Vec::new();
    for obs in sorted {
        if clusters.is_empty() || obs.x - clusters.last().unwrap().last().unwrap().x > tolerance {
            clusters.push(vec![obs]);
        } else {
            clusters.last_mut().unwrap().push(obs);
        }
    }
    clusters
}

pub fn count_pages(observations: &[PageCoordinateObservation]) -> usize {
    observations.iter().map(|o| o.page_index).collect::<std::collections::BTreeSet<_>>().len()
}

pub fn average(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.iter().sum::<f64>() / values.len() as f64
}

pub fn median(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    }
}

pub fn resolve_coordinate_evidence(values: &[f64], tolerance: f64) -> CoordinateEvidence {
    let mut clusters = cluster_coordinates(values, tolerance);
    clusters.sort_by(|left, right| {
        right.values.len().cmp(&left.values.len())
            .then_with(|| average(&left.values).partial_cmp(&average(&right.values)).unwrap_or(std::cmp::Ordering::Equal))
    });
    let dominant = match clusters.first() {
        Some(c) => c,
        None => return CoordinateEvidence { x: None, conflicting: false },
    };
    let competing = clusters.get(1);
    if let Some(comp) = competing {
        if comp.values.len() == dominant.values.len() {
            return CoordinateEvidence { x: None, conflicting: true };
        }
    }
    let conflicting = if let Some(comp) = competing {
        comp.values.len() >= 3 && (comp.values.len() as f64 / dominant.values.len() as f64) >= 0.4
    } else {
        false
    };
    CoordinateEvidence { x: Some(average(&dominant.values)), conflicting }
}

pub fn resolve_page_supported_coordinate_evidence(observations: &[PageCoordinateObservation], tolerance: f64) -> CoordinateEvidence {
    let mut clusters = cluster_page_coordinates(observations, tolerance);
    clusters.sort_by(|left, right| {
        count_pages(right).cmp(&count_pages(left))
            .then_with(|| right.len().cmp(&left.len()))
            .then_with(|| {
                let ax = average(&right.iter().map(|o| o.x).collect::<Vec<_>>());
                let bx = average(&left.iter().map(|o| o.x).collect::<Vec<_>>());
                ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let dominant = match clusters.first() {
        Some(c) => c,
        None => return CoordinateEvidence { x: None, conflicting: false },
    };
    let dominant_x = average(&dominant.iter().map(|o| o.x).collect::<Vec<_>>());
    let competing = clusters.get(1);
    match competing {
        None => CoordinateEvidence { x: Some(dominant_x), conflicting: false },
        Some(comp) => {
            let tied = count_pages(comp) == count_pages(dominant) && comp.len() == dominant.len();
            if tied {
                return CoordinateEvidence { x: None, conflicting: true };
            }
            let conflicting = comp.len() >= 3 && (comp.len() as f64 / dominant.len() as f64) >= 0.4;
            CoordinateEvidence { x: Some(dominant_x), conflicting }
        }
    }
}

pub fn infer_coordinate_tolerance(lines: &[PhysicalTextLine]) -> f64 {
    if lines.is_empty() { return 1.0; }
    median(&lines.iter().map(|l| l.height).collect::<Vec<_>>()) / 4.0
}