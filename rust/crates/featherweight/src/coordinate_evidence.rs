use crate::model::PhysicalTextLine;

#[derive(Debug, Clone)]
pub struct CoordinateEvidence {
    pub x: Option<f64>,
    pub conflicting: bool,
}

pub fn infer_coordinate_tolerance(lines: &[PhysicalTextLine]) -> f64 {
    if lines.is_empty() { return 4.0; }
    let heights: Vec<f64> = lines.iter().map(|l| l.height).collect();
    heights.iter().copied().fold(0.0_f64, f64::max) / 4.0
}

pub fn cluster_coordinates(values: &[f64], tolerance: f64) -> Vec<Vec<f64>> {
    let mut clusters: Vec<Vec<f64>> = Vec::new();
    for v in values {
        if let Some(c) = clusters.iter_mut().find(|c| (c[0] - v).abs() <= tolerance) {
            c.push(*v);
        } else {
            clusters.push(vec![*v]);
        }
    }
    clusters
}

pub fn resolve_coordinate_evidence(values: impl IntoIterator<Item=f64>, tolerance: f64) -> CoordinateEvidence {
    let vals: Vec<f64> = values.into_iter().collect();
    if vals.is_empty() { return CoordinateEvidence { x: None, conflicting: false }; }
    let clusters = cluster_coordinates(&vals, tolerance);
    if clusters.len() > 1 {
        // conflicting if no dominant cluster
        let max = clusters.iter().map(|c| c.len()).max().unwrap_or(0);
        if max * 2 < vals.len() {
            return CoordinateEvidence { x: None, conflicting: true };
        }
    }
    let largest = clusters.into_iter().max_by_key(|c| c.len()).unwrap();
    let x = largest.iter().sum::<f64>() / largest.len() as f64;
    CoordinateEvidence { x: Some(x), conflicting: false }
}

pub fn resolve_page_supported_coordinate_evidence(values: impl IntoIterator<Item=(f64, usize)>, tolerance: f64) -> CoordinateEvidence {
    resolve_coordinate_evidence(values.into_iter().map(|(x, _)| x), tolerance)
}