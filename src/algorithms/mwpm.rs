//! Minimum-Weight Perfect Matching (MWPM) via Primal-Dual Blossom
//!
//! This module solves minimum-weight perfect matching on general undirected graphs
//! in polynomial time using a reduction to maximum-weight matching via a
//! cardinality-dominating transformation.
//!
//! # Mathematical Background
//!
//! Given an undirected graph $G = (V, E)$ with $|V| = 2k$ and edge weights $c_e$,
//! we wish to find a perfect matching $M$ minimizing:
//!
//! $$\sum_{e \in M} c_e$$
//!
//! The underlying solver (`integer-blossom`) computes a **maximum-weight matching**
//! with non-negative edge weights. To adapt it to minimum-weight perfect matching:
//!
//! 1. **Exact Scaling**: Floating-point weights are converted into exact integers
//!    by determining the common binary exponent and shifting mantissas without precision loss.
//! 2. **Cardinality Dominance**:
//!    Let $[c_{\min}, c_{\max}]$ be the range of edge costs. Any perfect matching has
//!    exactly $k = |V| / 2$ edges. We define transformed weights:
//!
//!    $$w_e = C_{\text{bonus}} - (c_e - c_{\min})$$
//!
//!    where $C_{\text{bonus}} > k \cdot (c_{\max} - c_{\min})$.
//!    Under this transformation:
//!    - Any matching with $k$ edges has total weight at least
//!      $k \cdot C_{\text{bonus}} - k \cdot (c_{\max} - c_{\min}) > (k - 1) \cdot C_{\text{bonus}}$.
//!    - Thus, any perfect matching strictly outweighs any non-perfect matching.
//!    - Among all perfect matchings (each having exactly $k$ edges), maximizing
//!      $\sum_{e \in M} w_e = k \cdot C_{\text{bonus}} + k \cdot c_{\min} - \sum_{e \in M} c_e$
//!      is strictly equivalent to minimizing $\sum_{e \in M} c_e$.

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::time::Instant;

/// Scales floating-point weights into exact 128-bit integers without precision loss.
#[derive(Debug, Clone)]
pub struct WeightScaler {
    common_exponent: i32,
}

impl WeightScaler {
    /// Extracts exact IEEE-754 binary floating point representation: `value = mantissa * 2^exponent`.
    #[inline]
    pub fn exact_parts(value: f64) -> (i128, i32) {
        if value == 0.0 {
            return (0, 0);
        }
        let bits = value.to_bits();
        let sign = if bits >> 63 == 0 { 1_i128 } else { -1_i128 };
        let biased_exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        if biased_exponent == 0 {
            // Subnormal number
            (sign * i128::from(fraction), -1074)
        } else {
            // Normal number
            (
                sign * i128::from(fraction | (1_u64 << 52)),
                biased_exponent - 1023 - 52,
            )
        }
    }

    /// Creates a new `WeightScaler` from a slice of candidate edge weights.
    pub fn new(weights: &[f64]) -> Self {
        let common_exponent = weights
            .iter()
            .map(|&w| Self::exact_parts(w))
            .filter(|&(mantissa, _)| mantissa != 0)
            .map(|(_, exponent)| exponent)
            .min()
            .unwrap_or(0);

        Self { common_exponent }
    }

    /// Converts a floating point weight to scaled `i128`.
    pub fn to_integer(&self, value: f64) -> Result<i128, String> {
        let (mantissa, exponent) = Self::exact_parts(value);
        if mantissa == 0 {
            return Ok(0);
        }

        let shift = u32::try_from(exponent - self.common_exponent)
            .map_err(|_| "invalid exponent shift during weight scaling".to_string())?;

        1_i128
            .checked_shl(shift)
            .and_then(|factor| mantissa.checked_mul(factor))
            .ok_or_else(|| "edge-weight precision range exceeds i128 capacity".to_string())
    }
}

/// A validated candidate edge for MWPM.
#[derive(Debug, Clone, Copy)]
pub struct FilteredEdge {
    pub u: usize,
    pub v: usize,
    pub original_index: usize,
    pub weight: f64,
}

/// Pre-filters edges: verifies finiteness, eliminates self-loops, and keeps minimum-weight parallel edges.
pub fn filter_edges(graph: &Graph) -> Result<Vec<FilteredEdge>, String> {
    let mut candidates = Vec::with_capacity(graph.edges.len());

    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.u >= graph.n || edge.v >= graph.n {
            return Err(format!(
                "edge {index} references out-of-bounds vertex (u={}, v={}, n={})",
                edge.u, edge.v, graph.n
            ));
        }

        let cost = edge.weight_or_one();
        if !cost.is_finite() {
            return Err(format!("edge {index} has a non-finite weight ({cost})"));
        }

        // Self-loops cannot participate in any simple matching
        if edge.u == edge.v {
            continue;
        }

        let (u, v) = (edge.u.min(edge.v), edge.u.max(edge.v));
        candidates.push(FilteredEdge {
            u,
            v,
            original_index: index,
            weight: cost,
        });
    }

    candidates.sort_by(|a, b| {
        (a.u, a.v)
            .cmp(&(b.u, b.v))
            .then_with(|| a.weight.total_cmp(&b.weight))
            .then_with(|| a.original_index.cmp(&b.original_index))
    });
    candidates.dedup_by(|next, current| (next.u, next.v) == (current.u, current.v));
    Ok(candidates)
}

/// Solves Minimum-Weight Perfect Matching on `graph`.
pub fn minimum_weight_perfect_matching(graph: &Graph) -> Result<MatchingResult, String> {
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

    let start = Instant::now();

    // 1. Validate and filter edges
    let filtered_edges = filter_edges(graph)?;
    if filtered_edges.is_empty() {
        return Err(format!(
            "graph (n = {}, m = {}) has no edges available for matching",
            graph.n,
            graph.edges.len()
        ));
    }

    // Check vertex degrees: every vertex must have at least one incident edge
    let mut degree = vec![0usize; graph.n];
    for e in &filtered_edges {
        degree[e.u] += 1;
        degree[e.v] += 1;
    }
    if let Some((v, _)) = degree.iter().enumerate().find(|&(_, &d)| d == 0) {
        return Err(format!(
            "graph has no perfect matching: vertex {v} has degree 0 (isolated)"
        ));
    }

    // 2. Exact scaling to integers
    let weights: Vec<f64> = filtered_edges.iter().map(|e| e.weight).collect();
    let scaler = WeightScaler::new(&weights);

    let mut scaled_edges: Vec<(usize, usize, usize, i128)> =
        Vec::with_capacity(filtered_edges.len());
    for e in &filtered_edges {
        let scaled_cost = scaler.to_integer(e.weight)?;
        scaled_edges.push((e.u, e.v, e.original_index, scaled_cost));
    }

    let (min_cost, max_cost) = scaled_edges.iter().fold(
        (i128::MAX, i128::MIN),
        |(min_cost, max_cost), &(_, _, _, cost)| (min_cost.min(cost), max_cost.max(cost)),
    );

    // Cardinality bonus ensures any perfect matching strictly dominates any non-perfect matching
    let k =
        i128::try_from(graph.n / 2).map_err(|_| "vertex count exceeds i128 limit".to_string())?;

    let cardinality_bonus = max_cost
        .checked_sub(min_cost)
        .and_then(|range| range.checked_mul(k))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "edge-weight range is too large to represent in i128".to_string())?;

    // 3. Build transformed weight matrix for integer blossom solver
    let mut transformed = vec![0_i128; graph.n * graph.n];

    for (u, v, _, cost) in scaled_edges {
        let weight = cardinality_bonus - (cost - min_cost);
        transformed[u * graph.n + v] = weight;
        transformed[v * graph.n + u] = weight;
    }

    // 4. Solve maximum weight matching via integer_blossom
    let mate = integer_blossom::max_weight_matching(&transformed, graph.n);

    // Verify perfect matching
    if mate.iter().any(Option::is_none) {
        let unmatched_count = mate.iter().filter(|m| m.is_none()).count();
        return Err(format!(
            "graph does not have a perfect matching: {unmatched_count} of {} vertices could not be matched",
            graph.n
        ));
    }

    // 5. Reconstruct matched edges
    let mut matched_edges = Vec::with_capacity(graph.n / 2);
    for (u, partner) in mate.into_iter().enumerate() {
        let v = partner.expect("all partners verified");
        if u < v {
            let position = filtered_edges
                .binary_search_by_key(&(u, v), |edge| (edge.u, edge.v))
                .map_err(|_| format!("internal error: matched edge ({u}, {v}) not found"))?;
            matched_edges.push(filtered_edges[position].original_index);
        }
    }

    matched_edges.sort_unstable();

    let total_weight: f64 = matched_edges
        .iter()
        .map(|&idx| graph.edges[idx].weight_or_one())
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
            runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
        },
    })
}
