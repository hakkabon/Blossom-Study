//! Common types and traits for matching algorithms

use serde::{Deserialize, Serialize};
use std::fmt;

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

impl fmt::Display for MatchingResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "matching_size={}", self.edges.len())?;
        writeln!(formatter, "total_weight={:.6}", self.total_weight)?;
        writeln!(formatter, "is_perfect={}", self.is_perfect)?;
        writeln!(formatter, "iterations={}", self.metadata.iterations)?;
        writeln!(formatter, "augmentations={}", self.metadata.augmentations)?;
        writeln!(
            formatter,
            "blossoms_created={}",
            self.metadata.blossoms_created
        )?;
        writeln!(
            formatter,
            "blossoms_expanded={}",
            self.metadata.blossoms_expanded
        )?;
        writeln!(formatter, "dual_updates={}", self.metadata.dual_updates)?;
        writeln!(formatter, "runtime_ms={:.3}", self.metadata.runtime_ms)?;
        if !self.edges.is_empty() {
            write!(formatter, "edges=")?;
            for (position, edge) in self.edges.iter().enumerate() {
                if position > 0 {
                    write!(formatter, " ")?;
                }
                write!(formatter, "{edge}")?;
            }
            writeln!(formatter)?;
        }
        Ok(())
    }
}

impl MatchingResult {
    /// Check that this result is a valid matching of `graph` and that its
    /// reported weight and perfectness agree with the selected edges.
    pub fn validate(&self, graph: &crate::graph::Graph) -> Result<(), String> {
        graph.validate()?;

        let mut covered = vec![false; graph.n];
        let mut expected_weight = 0.0;
        let mut compensation = 0.0;
        let mut absolute_weight_sum = 0.0;
        for &edge_index in &self.edges {
            let edge = graph.edges.get(edge_index).ok_or_else(|| {
                format!(
                    "matching references edge {edge_index}, but graph has {} edges",
                    graph.edges.len()
                )
            })?;
            if edge.u == edge.v {
                return Err(format!("matching contains self-loop edge {edge_index}"));
            }
            if covered[edge.u] || covered[edge.v] {
                return Err(format!(
                    "matching contains multiple edges incident to a vertex (edge {edge_index})"
                ));
            }
            covered[edge.u] = true;
            covered[edge.v] = true;
            let weight = edge.weight_or_one();
            if !weight.is_finite() {
                return Err(format!("matching edge {edge_index} has non-finite weight"));
            }
            absolute_weight_sum += weight.abs();

            let next_sum = expected_weight + weight;
            if expected_weight.abs() >= weight.abs() {
                compensation += (expected_weight - next_sum) + weight;
            } else {
                compensation += (weight - next_sum) + expected_weight;
            }
            expected_weight = next_sum;
        }
        expected_weight += compensation;

        if !self.total_weight.is_finite() || !expected_weight.is_finite() {
            return Err("matching or reported total weight is not finite".to_string());
        }
        if !absolute_weight_sum.is_finite() {
            return Err(
                "selected-edge weight magnitudes exceed the finite validation range".to_string(),
            );
        }
        let tolerance = 8.0
            * f64::EPSILON
            * (self.edges.len().max(1) as f64 + 1.0)
            * absolute_weight_sum
                .max(expected_weight.abs())
                .max(self.total_weight.abs())
                .max(1.0);
        if (self.total_weight - expected_weight).abs() > tolerance {
            return Err(format!(
                "reported total weight {} differs from selected-edge total {expected_weight}",
                self.total_weight
            ));
        }

        let is_perfect = covered.iter().all(|&is_covered| is_covered);
        if self.is_perfect != is_perfect {
            return Err(format!(
                "reported is_perfect={} but matching perfectness is {is_perfect}",
                self.is_perfect
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MatchingMetadata {
    /// Algorithm-specific work counter; its unit is documented per solver and
    /// is not comparable across implementations. Zero may mean uninstrumented.
    pub iterations: usize,
    /// Number of augmentations, or the known perfect-matching cardinality for
    /// adapters that do not expose the solver's internal count.
    #[serde(default)]
    pub augmentations: usize,
    /// Number of blossom contractions when instrumented; otherwise zero.
    pub blossoms_created: usize,
    /// Number of blossom expansions when instrumented; otherwise zero.
    pub blossoms_expanded: usize,
    /// Number of dual-adjustment steps when instrumented; otherwise zero.
    pub dual_updates: usize,
    /// Solver-reported wall time, in milliseconds. This is distinct from
    /// Criterion's externally measured benchmark duration.
    pub runtime_ms: f64,
}

pub mod blossom_engine;
pub mod duan_mwpm;
pub mod fast_blossom;
pub mod gabow_heuristic;
pub mod general_cardinality;
pub mod hopcroft_karp;
pub mod hungarian;
pub mod imported_cardinality;
mod korte_vygen;
pub mod micali_vazirani;
pub mod mwpm;
pub mod sparse_mwpm;

pub use blossom_engine::{BlossomEngine, blossom_engine_minimum_weight_perfect_matching};
pub use duan_mwpm::duan_minimum_weight_perfect_matching;
pub use fast_blossom::FastBlossomMatching;
pub use gabow_heuristic::{GabowHeuristic, gabow_heuristic_matching};
pub use hopcroft_karp::{HopcroftKarpSolver, hopcroft_karp_maximum_cardinality_matching};
pub use hungarian::{HungarianSolver, hungarian_minimum_weight_perfect_matching};
pub use imported_cardinality::{
    gabow_maximum_cardinality_matching, mn_maximum_cardinality_matching,
};
pub use micali_vazirani::{
    DdfsMarker, DdfsResult, MicaliVazirani, MicaliVaziraniError, Phase as MicaliVaziraniPhase,
    TenacityQueue, VisitSource,
};
pub use mwpm::{FilteredEdge, WeightScaler, minimum_weight_perfect_matching};
pub use sparse_mwpm::{SparseBlossomSolver, SparseEdge, sparse_minimum_weight_perfect_matching};

/// Retained for study, but excluded from the primary API and benchmarks.
pub mod reference {
    pub use super::korte_vygen::KorteVygenEdmonds;
}
