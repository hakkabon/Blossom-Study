//! Common types for matching algorithms

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Result of a matching computation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchingResult {
    /// Indices of edges in the matching
    pub edges: Vec<usize>,
    /// Total weight of the matching
    pub total_weight: f64,
    /// Whether the matching is perfect (covers all vertices)
    pub is_perfect: bool,
    /// Algorithm-specific metadata
    pub metadata: MatchingMetadata,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MatchingMetadata {
    pub iterations: usize,
    pub blossoms_created: usize,
    pub blossoms_expanded: usize,
    pub dual_updates: usize,
    pub runtime_ms: f64,
}

/// Trait for matching algorithms
pub trait MatchingAlgorithm {
    fn name(&self) -> &'static str;
    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult;
    fn min_weight_perfect(&mut self, graph: &crate::graph::Graph)
    -> Result<MatchingResult, String>;
}

/// Benchmark configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchConfig {
    pub graph_sizes: Vec<usize>,
    pub densities: Vec<f64>,
    pub weight_ranges: Vec<(f64, f64)>,
    pub runs_per_config: usize,
    pub algorithms: Vec<String>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            graph_sizes: vec![10, 20, 30, 40, 50, 60, 80, 100],
            densities: vec![0.1, 0.3, 0.5, 0.8, 1.0],
            weight_ranges: vec![(1.0, 100.0)],
            runs_per_config: 5,
            algorithms: vec!["edmonds".to_string(), "korte_vygen".to_string()],
        }
    }
}

pub mod edmonds;
pub mod fast_blossom;
pub mod korte_vygen;
pub mod union_find;

pub use edmonds::EdmondsAlgorithm;
pub use fast_blossom::FastBlossomMatching;
pub use korte_vygen::KorteVygenEdmonds;

/// O(V³) minimum-weight perfect matching via a primal-dual blossom solver.
///
/// A cardinality-dominating transformation lets the maximum-weight solver
/// handle sparse graphs and then minimize cost among all perfect matchings.
pub fn minimum_weight_perfect_matching(
    graph: &crate::graph::Graph,
) -> Result<MatchingResult, String> {
    if !graph.n.is_multiple_of(2) {
        return Err("a perfect matching requires an even number of vertices".into());
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
    let mut edge_for: Vec<Vec<Option<usize>>> = vec![vec![None; graph.n]; graph.n];
    for (index, edge) in graph.edges.iter().enumerate() {
        let cost = edge.weight.unwrap_or(1.0);
        if !cost.is_finite() {
            return Err(format!("edge {index} has a non-finite weight"));
        }
        if edge.u == edge.v {
            continue;
        }
        match edge_for[edge.u][edge.v] {
            Some(previous) if graph.edges[previous].weight.unwrap_or(1.0) <= cost => {}
            _ => {
                edge_for[edge.u][edge.v] = Some(index);
                edge_for[edge.v][edge.u] = Some(index);
            }
        }
    }
    if edge_for.iter().all(|row| row.iter().all(Option::is_none)) {
        return Err("graph does not have a perfect matching".into());
    }

    fn exact_parts(value: f64) -> (i128, i32) {
        if value == 0.0 {
            return (0, 0);
        }
        let bits = value.to_bits();
        let sign = if bits >> 63 == 0 { 1_i128 } else { -1_i128 };
        let biased_exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        if biased_exponent == 0 {
            (sign * i128::from(fraction), -1074)
        } else {
            (
                sign * i128::from(fraction | (1_u64 << 52)),
                biased_exponent - 1023 - 52,
            )
        }
    }

    let selected: Vec<(usize, usize, usize, i128, i32)> = (0..graph.n)
        .flat_map(|u| ((u + 1)..graph.n).map(move |v| (u, v)))
        .filter_map(|(u, v)| {
            edge_for[u][v].map(|index| {
                let (mantissa, exponent) = exact_parts(graph.edges[index].weight.unwrap_or(1.0));
                (u, v, index, mantissa, exponent)
            })
        })
        .collect();
    let common_exponent = selected
        .iter()
        .filter(|(_, _, _, mantissa, _)| *mantissa != 0)
        .map(|(_, _, _, _, exponent)| *exponent)
        .min()
        .unwrap_or(0);
    let scaled: Vec<(usize, usize, usize, i128)> = selected
        .into_iter()
        .map(|(u, v, index, mantissa, exponent)| {
            let shift = if mantissa == 0 {
                0
            } else {
                u32::try_from(exponent - common_exponent).expect("minimum exponent selected")
            };
            1_i128
                .checked_shl(shift)
                .and_then(|factor| mantissa.checked_mul(factor))
                .map(|cost| (u, v, index, cost))
                .ok_or_else(|| "edge-weight precision range exceeds i128 capacity".to_string())
        })
        .collect::<Result<_, _>>()?;
    let min_cost = scaled.iter().map(|(_, _, _, cost)| *cost).min().unwrap();
    let max_cost = scaled.iter().map(|(_, _, _, cost)| *cost).max().unwrap();
    let cardinality_bonus = max_cost
        .checked_sub(min_cost)
        .and_then(|range| range.checked_mul(i128::try_from(graph.n / 2).unwrap()))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "edge-weight range is too large".to_string())?;

    let mut transformed = vec![0_i128; graph.n * graph.n];
    for (u, v, _, cost) in scaled {
        let weight = cardinality_bonus - (cost - min_cost);
        transformed[u * graph.n + v] = weight;
        transformed[v * graph.n + u] = weight;
    }

    let mate = integer_blossom::max_weight_matching(&transformed, graph.n);
    if mate.iter().any(Option::is_none) {
        return Err("graph does not have a perfect matching".into());
    }
    let mut edges = Vec::with_capacity(graph.n / 2);
    for (u, partner) in mate.into_iter().enumerate() {
        let v = partner.expect("all partners checked above");
        if u < v {
            edges.push(edge_for[u][v].expect("solver only matches present edges"));
        }
    }
    edges.sort_unstable();
    let total_weight = edges
        .iter()
        .map(|&index| graph.edges[index].weight.unwrap_or(1.0))
        .sum();

    Ok(MatchingResult {
        edges,
        total_weight,
        is_perfect: true,
        metadata: MatchingMetadata {
            runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
            ..MatchingMetadata::default()
        },
    })
}
