//! Adapter for the educational Cherry-Tree prototype.
//!
//! Cherry-Tree models primal/dual growth explicitly with floating-point vertex
//! duals, a DSU for virtual contraction, flat cycle records, and one stored
//! bridge per blossom.  Its source documents two unresolved algorithmic gaps:
//! routing augmenting paths through nested blossoms and restoring alternating
//! labels during expansion.  Consequently this module is deliberately named a
//! *prototype* and never claims that a returned perfect matching is optimal.
//!
//! The adapter accepts only complete graphs, catches prototype panics, enforces
//! a bounded iteration count in the imported engine, and validates the returned
//! matching before exposing it as a `MatchingResult`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use super::cherry_tree_prototype::CherryTreeSolver as PrototypeCore;
use super::mwpm::filter_edges;
use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

/// Explicitly experimental wrapper around the incomplete Cherry-Tree design.
#[derive(Clone, Copy, Debug, Default)]
pub struct CherryTreePrototype;

impl CherryTreePrototype {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        cherry_tree_prototype_matching(graph)
    }
}

/// Run Cherry-Tree and return a validated perfect matching when it finds one.
///
/// Success certifies only matching feasibility.  Until the missing nested
/// blossom operations are completed, callers must not interpret success as an
/// optimality certificate.
pub fn cherry_tree_prototype_matching(graph: &Graph) -> Result<MatchingResult, String> {
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
    let mates =
        catch_unwind(AssertUnwindSafe(|| core.solve()[..graph.n].to_vec())).map_err(|_| {
            "Cherry-Tree prototype panicked while processing its incomplete blossom state"
                .to_string()
        })?;

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

    Ok(MatchingResult {
        edges: matched_edges,
        total_weight,
        is_perfect: true,
        metadata: MatchingMetadata {
            iterations: 0,
            augmentations: graph.n / 2,
            blossoms_created: core.blossoms.len(),
            blossoms_expanded: core
                .blossoms
                .iter()
                .filter(|blossom| !blossom.is_active)
                .count(),
            dual_updates: 0,
            runtime_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    })
}
