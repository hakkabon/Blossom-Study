//! Common types for matching algorithms

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
    fn name(&self) -> &'static str;
    fn max_cardinality(&mut self, graph: &crate::graph::Graph) -> MatchingResult;
    fn min_weight_perfect(&mut self, graph: &crate::graph::Graph) -> Result<MatchingResult, String>;
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

pub mod union_find;
pub mod edmonds;
pub mod korte_vygen;
pub mod fast_blossom;

pub use edmonds::EdmondsAlgorithm;
pub use korte_vygen::KorteVygenEdmonds;
pub use fast_blossom::FastBlossomMatching;

#[cfg(test)]
mod fast_blossom_tests {
    use crate::algorithms::FastBlossomMatching;
    use crate::graph::Graph;

    /// Helper to convert a list of edges into our Graph format
    fn build_graph(n: usize, edges: &[(usize, usize)]) -> Graph {
        let mut graph_edges = Vec::new();
        for &(u, v) in edges {
            graph_edges.push(crate::graph::Edge::new(u, v, None));
        }
        Graph::from_edges(n, graph_edges)
    }

    /// Helper to count total matched edges and verify consistency
    fn verify_and_count_matches(match_pair: &[usize]) -> usize {
        let mut count = 0;
        for (u, &v) in match_pair.iter().enumerate() {
            if v != usize::MAX {
                // Symmetrical integrity check: If u matches v, v MUST match u
                assert_eq!(match_pair[v], u, "Asymmetric match found between {} and {}", u, v);
                count += 1;
            }
        }
        // Each matched edge is counted twice (u->v and v->u)
        count / 2
    }

    #[test]
    fn test_pentagon_with_tail() {
        // Graph structure: 
        // 0-1-2-3-4 is a 5-cycle (Pentagon). Node 0 also connects to node 5 (the Tail).
        // Max cardinality matching should yield exactly 3 pairs.
        let n = 6;
        let edges = vec![
            (0, 1), (1, 2), (2, 3), (3, 4), (4, 0), // Pentagon
            (0, 5)                                  // Tail
        ];
        
        let graph = build_graph(n, &edges);
        let mut matcher = FastBlossomMatching::from_graph(&graph);
        let results = matcher.solve();

        let match_count = verify_and_count_matches(results);
        assert_eq!(match_count, 3, "Pentagon with tail failed to find 3 matches.");
        
        // Ensure the isolated tail node (5) successfully forced its way into a match
        assert_ne!(results[5], usize::MAX, "The tail node 5 was left unmatched.");
    }

    #[test]
    fn test_nested_blossoms() {
        // A complex general graph where identifying an augmenting path 
        // forces an inner blossom to contract, which immediately triggers 
        // an outer blossom contraction.
        let n = 8;
        let edges = vec![
            (0, 1), (1, 2), (2, 0), // Inner triangle blossom (0, 1, 2)
            (2, 3), (3, 4),         // Path leading out
            (4, 5), (5, 6), (6, 4), // Outer triangle blossom (4, 5, 6)
            (0, 7), (6, 7)          // Back-arc link wrapping both together
        ];

        let graph = build_graph(n, &edges);
        let mut matcher = FastBlossomMatching::from_graph(&graph);
        let results = matcher.solve();

        let match_count = verify_and_count_matches(results);
        // An 8 node graph with this configuration allows a perfect matching (4 pairs)
        assert_eq!(match_count, 4, "Nested blossoms failed to calculate perfect matching.");
    }

    #[test]
    fn test_unmatchable_triangle() {
        // A simple 3-node triangle. Only 1 pair can ever be made.
        // One node must reliably be left out without crashing the implementation.
        let n = 3;
        let edges = vec![(0, 1), (1, 2), (2, 0)];

        let graph = build_graph(n, &edges);
        let mut matcher = FastBlossomMatching::from_graph(&graph);
        let results = matcher.solve();

        let match_count = verify_and_count_matches(results);
        assert_eq!(match_count, 1, "Triangle should only find exactly 1 matching pair.");

        // Count how many vertices were correctly identified as unmatched
        let unmatched_count = results.iter().filter(|&&v| v == usize::MAX).count();
        assert_eq!(unmatched_count, 1, "Exactly one node should be left unmatched.");
    }
}