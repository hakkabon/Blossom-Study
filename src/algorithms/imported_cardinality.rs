//! Adapters for the two phased Gabow-family cardinality implementations.
//!
//! The implementations keep their own graph and matching types in
//! `implementations/`. This boundary maps their vertex mates back to edge IDs
//! in the study graph, including graphs with parallel edges.

use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::{collections::BTreeMap, time::Instant};

fn adapt(graph: &Graph, mate: &[usize], elapsed_ms: f64, phases: usize) -> MatchingResult {
    let mut original = BTreeMap::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.u != edge.v {
            let pair = (edge.u.min(edge.v), edge.u.max(edge.v));
            original.entry(pair).or_insert(index);
        }
    }
    let edges: Vec<usize> = mate
        .iter()
        .enumerate()
        .filter_map(|(u, &v)| (u < v && v < graph.n).then(|| original[&(u, v)]))
        .collect();
    let cardinality = edges.len();
    let total_weight = edges.iter().map(|&i| graph.edges[i].weight_or_one()).sum();
    MatchingResult {
        is_perfect: 2 * cardinality == graph.n,
        edges,
        total_weight,
        metadata: MatchingMetadata {
            iterations: phases,
            augmentations: cardinality,
            runtime_ms: elapsed_ms,
            ..Default::default()
        },
    }
}

fn edge_pairs(graph: &Graph) -> Vec<(usize, usize)> {
    graph.edges.iter().map(|e| (e.u, e.v)).collect()
}

/// Gabow's weighted-matching approach to maximum cardinality matching.
pub fn gabow_maximum_cardinality_matching(graph: &Graph) -> MatchingResult {
    let start = Instant::now();
    let g = gabow_matching::Graph::new(graph.n, &edge_pairs(graph));
    let (m, phases) = gabow_matching::maximum_matching(&g);
    adapt(
        graph,
        &m.mate,
        start.elapsed().as_secs_f64() * 1000.0,
        phases.len(),
    )
}

/// Mehlhorn–Nobahari's revised shortest-path phase for Gabow matching.
pub fn mn_maximum_cardinality_matching(graph: &Graph) -> MatchingResult {
    let start = Instant::now();
    let g = mn_matching::Graph::new(graph.n, &edge_pairs(graph));
    let (m, phases) = mn_matching::maximum_matching(&g);
    adapt(
        graph,
        &m.mate,
        start.elapsed().as_secs_f64() * 1000.0,
        phases.len(),
    )
}
