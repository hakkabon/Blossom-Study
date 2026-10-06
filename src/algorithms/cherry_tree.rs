//! Adapter for the corrected Cherry-Tree dense-matrix solver.
//!
//! The original prototype's DSU/bridge state was not rich enough to represent
//! an alternating forest or route paths through nested blossoms correctly.  Its
//! public CLI name is retained for compatibility, while the repaired core uses
//! the repository's endpoint-aware hierarchical primal-dual kernel.  See
//! `cherry_tree_prototype` for the representation rationale.

use std::time::Instant;

use super::cherry_tree_prototype::CherryTreeSolver as PrototypeCore;
use super::mwpm::filter_edges;
use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

/// Corrected Cherry-Tree matrix representation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CherryTreeMatrixSolver;

impl CherryTreeMatrixSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        cherry_tree_minimum_weight_perfect_matching(graph)
    }
}

/// Historical public name retained for source compatibility.
pub type CherryTreePrototype = CherryTreeMatrixSolver;

/// Run the corrected Cherry-Tree representation and return an exact MWPM.
pub fn cherry_tree_minimum_weight_perfect_matching(
    graph: &Graph,
) -> Result<MatchingResult, String> {
    let started = Instant::now();
    if !graph.n.is_multiple_of(2) {
        return Err(format!(
            "Cherry-Tree requires an even vertex count (n = {})",
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

    let filtered = filter_edges(graph)?;
    let expected_edges = graph
        .n
        .checked_mul(graph.n.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| "vertex count is too large".to_string())?;
    if filtered.len() != expected_edges {
        return Err(format!(
            "Cherry-Tree is a complete-distance-matrix prototype: expected {expected_edges} undirected pairs, found {}",
            filtered.len()
        ));
    }

    let mut matrix = vec![vec![0.0; graph.n]; graph.n];
    for edge in &filtered {
        matrix[edge.u][edge.v] = edge.weight;
        matrix[edge.v][edge.u] = edge.weight;
    }

    let mut core = PrototypeCore::new(graph.n, matrix);
    let mates = core.solve()?.to_vec();

    let mut matched_edges = Vec::with_capacity(graph.n / 2);
    for (u, &v) in mates.iter().enumerate() {
        if v >= graph.n || v == u || mates.get(v) != Some(&u) {
            return Err(format!(
                "Cherry-Tree did not produce a perfect reciprocal matching (first failure at vertex {u})"
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

    let mut metadata = core.metadata().clone();
    metadata.runtime_ms = started.elapsed().as_secs_f64() * 1000.0;

    Ok(MatchingResult {
        edges: matched_edges,
        total_weight,
        is_perfect: true,
        metadata,
    })
}

/// Historical function name retained for source compatibility.
pub fn cherry_tree_prototype_matching(graph: &Graph) -> Result<MatchingResult, String> {
    cherry_tree_minimum_weight_perfect_matching(graph)
}
