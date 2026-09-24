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

/// Trait for matching algorithms
pub trait MatchingAlgorithm {
    /// Name identifier of the algorithm
    fn name(&self) -> &'static str;
    /// Maximum cardinality matching
    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult;
    /// Minimum weight perfect matching
    fn min_weight_perfect(&mut self, graph: &crate::graph::Graph)
    -> Result<MatchingResult, String>;
}

pub mod edmonds;
pub mod fast_blossom;
pub mod korte_vygen;
pub mod mwpm;
pub mod union_find;

pub use edmonds::EdmondsAlgorithm;
pub use fast_blossom::FastBlossomMatching;
pub use korte_vygen::KorteVygenEdmonds;
pub use mwpm::{FilteredEdge, WeightScaler, minimum_weight_perfect_matching};

impl MatchingAlgorithm for EdmondsAlgorithm {
    fn name(&self) -> &'static str {
        "edmonds"
    }

    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult {
        self.max_cardinality(graph)
    }

    fn min_weight_perfect(
        &mut self,
        graph: &crate::graph::Graph,
    ) -> Result<MatchingResult, String> {
        self.min_weight_perfect(graph)
    }
}

impl MatchingAlgorithm for KorteVygenEdmonds<'_> {
    fn name(&self) -> &'static str {
        "korte_vygen"
    }

    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult {
        assert_eq!(
            graph.n,
            self.graph().n,
            "solver called with a different graph size"
        );
        assert_eq!(
            graph.edges,
            self.graph().edges,
            "solver called with different edges"
        );
        self.max_cardinality()
    }

    fn min_weight_perfect(
        &mut self,
        graph: &crate::graph::Graph,
    ) -> Result<MatchingResult, String> {
        Self::min_weight_perfect_matching(graph)
    }
}

impl MatchingAlgorithm for FastBlossomMatching {
    fn name(&self) -> &'static str {
        "fast_blossom"
    }

    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult {
        Self::max_cardinality_matching(graph)
    }

    fn min_weight_perfect(
        &mut self,
        graph: &crate::graph::Graph,
    ) -> Result<MatchingResult, String> {
        Self::min_weight_perfect_matching(graph)
    }
}
