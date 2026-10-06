//! Blossom-Tree: a flat-containment weighted blossom variant.
//!
//! The engine behind this adapter is the implementation from the sibling
//! `Blossom-Tree` project.  It began as an MIT-licensed adaptation of
//! `mwmatching` and replaces the direct blossom-parent array with immutable
//! contraction records plus a hash-based active-containment index.  The full
//! license notice is retained in the repository's `LICENSE-MWMATCHING` file.
//!
//! Keeping this adapter separate is useful educationally: it lets benchmarks
//! compare two representations of the same Edmonds/Galil primal-dual ideas—
//! hierarchical blossoms in `BlossomEngine` and flat contraction history here.

use std::time::Instant;

use super::blossom_tree_flat::{Matching, SENTINEL};
use super::mwpm::{WeightScaler, filter_edges};
use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

/// Public marker type for the flat-containment solver.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlossomTreeSolver;

impl BlossomTreeSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        blossom_tree_minimum_weight_perfect_matching(graph)
    }
}

pub fn blossom_tree_minimum_weight_perfect_matching(
    graph: &Graph,
) -> Result<MatchingResult, String> {
    let started = Instant::now();
    if !graph.n.is_multiple_of(2) {
        return Err(format!(
            "a perfect matching requires an even number of vertices (graph has n = {})",
            graph.n
        ));
    }
    if graph.n == 0 {
        return Ok(MatchingResult {
            edges: Vec::new(),
            total_weight: 0.0,
            is_perfect: true,
            metadata: MatchingMetadata::default(),
        });
    }

    // Reuse the study's canonical filtering policy: discard self-loops and
    // retain the cheapest edge for each undirected parallel pair.
    let filtered = filter_edges(graph)?;
    let mut degree = vec![0_usize; graph.n];
    for edge in &filtered {
        degree[edge.u] += 1;
        degree[edge.v] += 1;
    }
    if let Some((vertex, _)) = degree.iter().enumerate().find(|(_, value)| **value == 0) {
        return Err(format!(
            "graph has no perfect matching: vertex {vertex} has degree 0 (isolated)"
        ));
    }

    // Blossom-Tree maximizes weight.  Exact binary scaling followed by the
    // same cardinality-dominating transformation used by the other adapters
    // turns MWPM into maximum-weight matching without rounding f64 costs.
    let costs: Vec<f64> = filtered.iter().map(|edge| edge.weight).collect();
    let scaler = WeightScaler::new(&costs);
    let scaled: Vec<i128> = costs
        .into_iter()
        .map(|cost| scaler.to_integer(cost))
        .collect::<Result<_, _>>()?;
    let (minimum, maximum) = scaled
        .iter()
        .copied()
        .fold((i128::MAX, i128::MIN), |(minimum, maximum), cost| {
            (minimum.min(cost), maximum.max(cost))
        });
    let pairs = i128::try_from(graph.n / 2)
        .map_err(|_| "vertex count exceeds i128 capacity".to_string())?;
    let bonus = maximum
        .checked_sub(minimum)
        .and_then(|range| range.checked_mul(pairs))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "edge-weight range is too large to represent in i128".to_string())?;
    if bonus > i128::MAX / 4 {
        return Err(
            "edge-weight range leaves insufficient i128 headroom for Blossom-Tree dual arithmetic"
                .to_string(),
        );
    }

    let weighted_edges = filtered
        .iter()
        .zip(scaled)
        .map(|(edge, cost)| {
            let weight = bonus - (cost - minimum);
            (edge.u, edge.v, weight)
        })
        .collect();
    let mut engine = Matching::new(weighted_edges);
    engine.max_cardinality();
    let mates = engine.solve();

    if mates.len() != graph.n || mates.contains(&SENTINEL) {
        return Err("graph does not have a perfect matching".to_string());
    }

    let mut matched_edges = Vec::with_capacity(graph.n / 2);
    for (u, &v) in mates.iter().enumerate() {
        if v >= graph.n || v == u || mates[v] != u {
            return Err(format!(
                "Blossom-Tree returned a non-reciprocal mate relation at vertex {u}"
            ));
        }
        if u < v {
            let position = filtered
                .binary_search_by_key(&(u, v), |edge| (edge.u, edge.v))
                .map_err(|_| format!("internal error: matched edge ({u}, {v}) not found"))?;
            matched_edges.push(filtered[position].original_index);
        }
    }
    matched_edges.sort_unstable();
    let total_weight = matched_edges
        .iter()
        .map(|&index| graph.edges[index].weight_or_one())
        .sum();

    Ok(MatchingResult {
        edges: matched_edges,
        total_weight,
        is_perfect: true,
        metadata: MatchingMetadata {
            iterations: 0,
            augmentations: graph.n / 2,
            blossoms_created: 0,
            blossoms_expanded: 0,
            dual_updates: 0,
            runtime_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    })
}
