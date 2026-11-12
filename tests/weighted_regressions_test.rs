//! Graphs that exposed defects in retired weighted implementations.
//!
//! Keep the inputs and expected optima independent of the solver that first
//! exposed them. Each active weighted implementation must return a valid
//! perfect matching with the same minimum cost.

mod common;

use blossom_study::graph::{Edge, Graph};
use blossom_study::{
    MatchingResult, blossom_engine_minimum_weight_perfect_matching,
    duan_minimum_weight_perfect_matching, minimum_weight_perfect_matching,
    sparse_minimum_weight_perfect_matching,
};
use common::assert_perfect_matching;

type Solver = fn(&Graph) -> Result<MatchingResult, String>;

const SOLVERS: [(&str, Solver); 4] = [
    ("primal-dual", minimum_weight_perfect_matching),
    (
        "blossom-engine",
        blossom_engine_minimum_weight_perfect_matching,
    ),
    ("sparse-blossom", sparse_minimum_weight_perfect_matching),
    ("duan-scaling", duan_minimum_weight_perfect_matching),
];

fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Graph {
    Graph::from_edges(
        n,
        edges
            .iter()
            .map(|&(u, v, weight)| Edge::new(u, v, Some(weight)))
            .collect(),
    )
}

fn expect_weight(graph: &Graph, expected: f64) {
    for (name, solve) in SOLVERS {
        let result = solve(graph).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_perfect_matching(graph, &result);
        assert_eq!(result.total_weight, expected, "{name}");
    }
}

#[test]
fn odd_cycle_with_negative_bridge() {
    expect_weight(
        &graph(
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
        ),
        -1.0,
    );
}

#[test]
fn sparse_and_equal_weight_k4() {
    expect_weight(&graph(4, &[(0, 1, 1.0), (2, 3, 1.0)]), 2.0);
    expect_weight(
        &graph(
            4,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (1, 2, 1.0),
                (1, 3, 1.0),
                (2, 3, 1.0),
            ],
        ),
        2.0,
    );
}

#[test]
fn unique_k4_and_greedy_trap() {
    // The unique K4 exposed a non-reciprocal mate relation in an earlier
    // implementation. The second graph has a greedy perfect matching costing
    // 948, while its optimum costs 537.
    expect_weight(
        &graph(
            4,
            &[
                (0, 1, 1.0),
                (0, 2, 2.0),
                (0, 3, 4.0),
                (1, 2, 3.0),
                (1, 3, 6.0),
                (2, 3, 5.0),
            ],
        ),
        6.0,
    );
    expect_weight(
        &graph(
            4,
            &[
                (0, 1, 963.0),
                (0, 2, 281.0),
                (0, 3, 15.0),
                (1, 2, 933.0),
                (1, 3, 256.0),
                (2, 3, 465.0),
            ],
        ),
        537.0,
    );
}

#[test]
fn nested_odd_cycles_and_infeasible_triangles() {
    expect_weight(
        &graph(
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
        ),
        4.0,
    );
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
    for (name, solve) in SOLVERS {
        assert!(
            solve(&infeasible).is_err(),
            "{name} accepted infeasible graph"
        );
    }
}

#[test]
fn fractional_dense_and_planted_sparse_cases() {
    // These deterministic families preserve the former solvers' randomized
    // differential checks, including exact binary fractions and negative costs.
    let mut state = 0xb105_50aa_u64;
    for n in [4, 6, 8] {
        for case in 0..40 {
            let mut edges = Vec::new();
            for u in 0..n {
                for v in u + 1..n {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let weight = ((state >> 32) % 1601) as f64 / 8.0 - 100.0;
                    if (u % 2 == 0 && v == u + 1) || state >> 61 == 0 {
                        edges.push((u, v, weight));
                    }
                }
            }
            let instance = graph(n, &edges);
            let expected = minimum_weight_perfect_matching(&instance)
                .unwrap()
                .total_weight;
            for (name, solve) in SOLVERS.into_iter().skip(1) {
                let result = solve(&instance)
                    .unwrap_or_else(|error| panic!("n={n}, case={case}, {name}: {error}"));
                assert_perfect_matching(&instance, &result);
                assert_eq!(result.total_weight, expected, "n={n}, case={case}, {name}");
            }
        }
    }
}
