//! Common types and traits for matching algorithms

use serde::{Deserialize, Serialize};

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
    /// Name identifier of the algorithm
    fn name(&self) -> &'static str;
    /// Maximum cardinality matching
    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult;
    /// Minimum weight perfect matching
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
            algorithms: vec![
                "edmonds".to_string(),
                "korte_vygen".to_string(),
                "fast_blossom".to_string(),
            ],
        }
    }
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

impl MatchingAlgorithm for KorteVygenEdmonds {
    fn name(&self) -> &'static str {
        "korte_vygen"
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
