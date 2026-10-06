//! Cross-checks for the newly collected educational weighted solvers.
//!
//! Blossom-Tree is expected to agree with the established exact solver.
//! Cherry-Tree is deliberately tested only for safe execution and matching
//! feasibility because its nested-blossom optimality logic is unfinished.

use blossom_study::graph::{Edge, Graph};
use blossom_study::{
    BlossomTreeSolver, CherryTreePrototype, minimum_weight_perfect_matching,
    sparse_minimum_weight_perfect_matching,
};

fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Graph {
    Graph::from_edges(
        n,
        edges
            .iter()
            .map(|&(u, v, weight)| Edge::new(u, v, Some(weight)))
            .collect(),
    )
}

fn assert_perfect(graph: &Graph, result: &blossom_study::MatchingResult) {
    let mut covered = vec![false; graph.n];
    for &edge_index in &result.edges {
        let edge = &graph.edges[edge_index];
        assert!(!covered[edge.u]);
        assert!(!covered[edge.v]);
        covered[edge.u] = true;
        covered[edge.v] = true;
    }
    assert!(covered.into_iter().all(|value| value));
}

#[test]
fn blossom_tree_matches_exact_baseline_on_varied_complete_graphs() {
    let n = 8;
    let mut state = 0xb105_50aa_u64;
    for case in 0..200 {
        let mut edges = Vec::new();
        for u in 0..n {
            for v in (u + 1)..n {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                // Binary fractions exercise exact f64 scaling without relying
                // solely on the integer-weight tests imported with the engine.
                let weight = ((state >> 32) % 1601) as f64 / 8.0 - 100.0;
                edges.push((u, v, weight));
            }
        }
        let graph = graph(n, &edges);
        let expected = minimum_weight_perfect_matching(&graph).unwrap();
        let actual = BlossomTreeSolver::new().solve(&graph).unwrap();
        assert_perfect(&graph, &actual);
        assert_eq!(actual.total_weight, expected.total_weight, "case {case}");
    }
}

#[test]
fn all_verified_weighted_solvers_agree_on_an_odd_cycle_instance() {
    let graph = graph(
        6,
        &[
            (0, 1, 2.0),
            (1, 2, 2.0),
            (2, 0, 2.0),
            (3, 4, 2.0),
            (4, 5, 2.0),
            (5, 3, 2.0),
            (0, 3, -5.0),
        ],
    );
    let baseline = minimum_weight_perfect_matching(&graph).unwrap();
    let flat = BlossomTreeSolver::new().solve(&graph).unwrap();
    let sparse = sparse_minimum_weight_perfect_matching(&graph).unwrap();
    assert_eq!(flat.total_weight, baseline.total_weight);
    assert_eq!(sparse.total_weight, baseline.total_weight);
}

#[test]
fn cherry_tree_is_guarded_and_rejects_sparse_input() {
    let sparse = graph(4, &[(0, 1, 1.0), (2, 3, 1.0)]);
    let error = CherryTreePrototype::new().solve(&sparse).unwrap_err();
    assert!(error.contains("complete-distance-matrix prototype"));
}

#[test]
fn cherry_tree_can_return_a_valid_result_on_a_simple_complete_graph() {
    let graph = graph(
        4,
        &[
            (0, 1, 1.0),
            (0, 2, 4.0),
            (0, 3, 5.0),
            (1, 2, 5.0),
            (1, 3, 4.0),
            (2, 3, 1.0),
        ],
    );
    let result = CherryTreePrototype::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result);
    // This assertion documents this easy case, not a general optimality claim.
    assert_eq!(result.total_weight, 2.0);
}
