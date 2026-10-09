//! Cross-checks for the newly collected educational weighted solvers.
//!
//! Blossom-Tree and the reconstructed Cherry-Tree solver are expected to agree
//! with the established exact solver.  The Cherry cases cover the defects from
//! the retired prototypes as well as sparse and nested-blossom searches.

use blossom_study::graph::{Edge, Graph};
use blossom_study::{
    BlossomTreeSolver, CherryTreeSolver, minimum_weight_perfect_matching,
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
    let cherry = CherryTreeSolver::new().solve(&graph).unwrap();
    assert_eq!(flat.total_weight, baseline.total_weight);
    assert_eq!(sparse.total_weight, baseline.total_weight);
    assert_eq!(cherry.total_weight, baseline.total_weight);
}

#[test]
fn cherry_tree_accepts_sparse_input() {
    let sparse = graph(4, &[(0, 1, 1.0), (2, 3, 1.0)]);
    let result = CherryTreeSolver::new().solve(&sparse).unwrap();
    assert_perfect(&sparse, &result);
    assert_eq!(result.total_weight, 2.0);
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
    let result = CherryTreeSolver::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result);
    assert_eq!(result.total_weight, 2.0);
}

#[test]
fn cherry_tree_handles_the_equal_weight_k4_regression() {
    let graph = graph(
        4,
        &[
            (0, 1, 1.0),
            (0, 2, 1.0),
            (0, 3, 1.0),
            (1, 2, 1.0),
            (1, 3, 1.0),
            (2, 3, 1.0),
        ],
    );
    let result = CherryTreeSolver::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result);
    assert_eq!(result.total_weight, 2.0);
}

#[test]
fn cherry_tree_handles_the_unique_k4_regression() {
    // Lexicographic edge weights [01, 02, 03, 12, 13, 23].  The former
    // implementation misclassified a same-tree even/even collision here and
    // produced a non-reciprocal mate table.
    let graph = graph(
        4,
        &[
            (0, 1, 1.0),
            (0, 2, 2.0),
            (0, 3, 4.0),
            (1, 2, 3.0),
            (1, 3, 6.0),
            (2, 3, 5.0),
        ],
    );
    let result = CherryTreeSolver::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result);
    assert_eq!(result.total_weight, 6.0);
}

#[test]
fn cherry_tree_matches_exact_baseline_on_complete_graphs() {
    let mut state = 0xc4e2_7719_5eed_u64;
    for n in [4, 6, 8] {
        for case in 0..100 {
            let mut edges = Vec::new();
            for u in 0..n {
                for v in (u + 1)..n {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let weight = ((state >> 32) % 401) as f64 / 4.0 - 50.0;
                    edges.push((u, v, weight));
                }
            }
            let graph = graph(n, &edges);
            let expected = minimum_weight_perfect_matching(&graph).unwrap();
            let actual = CherryTreeSolver::new().solve(&graph).unwrap();
            assert_perfect(&graph, &actual);
            assert_eq!(
                actual.total_weight, expected.total_weight,
                "n={n}, case={case}"
            );
        }
    }
}

#[test]
fn cherry_tree_does_not_accept_a_greedy_but_suboptimal_perfect_matching() {
    // The retired solver stopped as soon as its greedy initializer happened to
    // cover every vertex.  It returned (0,3)+(1,2) with cost 948, although the
    // dual-optimal matching below has cost 537.
    let graph = graph(
        4,
        &[
            (0, 1, 963.0),
            (0, 2, 281.0),
            (0, 3, 15.0),
            (1, 2, 933.0),
            (1, 3, 256.0),
            (2, 3, 465.0),
        ],
    );
    let result = CherryTreeSolver::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result);
    assert_eq!(result.total_weight, 537.0);
}

#[test]
fn cherry_tree_matches_the_oracle_on_random_sparse_graphs_with_planted_matchings() {
    let mut state = 0x51a7_5e5d_0dd5_eed5_u64;
    for n in [4_usize, 6, 8, 10] {
        for case in 0..200 {
            let mut edges = Vec::new();

            // Plant a perfect matching so an error cannot be dismissed as an
            // unmatchable random instance.
            for u in (0..n).step_by(2) {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                edges.push((u, u + 1, ((state >> 32) % 401) as f64 - 200.0));
            }
            for u in 0..n {
                for v in (u + 1)..n {
                    if u / 2 == v / 2 && u.is_multiple_of(2) {
                        continue;
                    }
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if (state >> 61) != 0 {
                        continue;
                    }
                    edges.push((u, v, ((state >> 32) % 401) as f64 - 200.0));
                }
            }

            let graph = graph(n, &edges);
            let expected = minimum_weight_perfect_matching(&graph).unwrap();
            let actual = CherryTreeSolver::new().solve(&graph).unwrap();
            assert_perfect(&graph, &actual);
            assert_eq!(
                actual.total_weight, expected.total_weight,
                "n={n}, case={case}"
            );
        }
    }
}

#[test]
fn cherry_tree_contracts_nested_odd_cycles_and_rejects_infeasible_graphs() {
    let nested = graph(
        8,
        &[
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            (2, 3, 1.0),
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 6, 1.0),
            (6, 4, 1.0),
            (0, 7, 1.0),
            (6, 7, 1.0),
        ],
    );
    let result = CherryTreeSolver::new().solve(&nested).unwrap();
    assert_perfect(&nested, &result);
    assert_eq!(result.total_weight, 4.0);
    assert!(result.metadata.blossoms_created > 0);

    // Both triangles have positive degree everywhere, but neither component
    // can contribute a perfect matching.
    let infeasible = graph(
        6,
        &[
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 3, 1.0),
        ],
    );
    assert!(CherryTreeSolver::new().solve(&infeasible).is_err());
}
