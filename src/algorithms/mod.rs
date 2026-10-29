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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MatchingMetadata {
    pub iterations: usize,
    #[serde(default)]
    pub augmentations: usize,
    pub blossoms_created: usize,
    pub blossoms_expanded: usize,
    pub dual_updates: usize,
    pub runtime_ms: f64,
}

pub mod blossom_engine;
pub mod blossom_tree;
mod blossom_tree_flat;
pub mod cherry_tree_solver;
pub mod fast_blossom;
pub mod gabow_cardinality;
pub mod hopcroft_karp;
pub mod hungarian;
mod korte_vygen;
pub mod micali_vazirani;
pub mod mwpm;
pub mod sparse_mwpm;
mod union_find;

pub use blossom_engine::{BlossomEngine, blossom_engine_minimum_weight_perfect_matching};
pub use blossom_tree::{BlossomTreeSolver, blossom_tree_minimum_weight_perfect_matching};
pub use cherry_tree_solver::{CherryTreeSolver, cherry_tree_minimum_weight_perfect_matching};
pub use fast_blossom::FastBlossomMatching;
pub use gabow_cardinality::{GabowCardinality, gabow_maximum_cardinality_matching};
pub use hopcroft_karp::{HopcroftKarpSolver, hopcroft_karp_maximum_cardinality_matching};
pub use hungarian::{HungarianSolver, hungarian_minimum_weight_perfect_matching};
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
