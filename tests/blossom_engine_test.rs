use blossom_study::algorithms::{
    BlossomEngine, blossom_engine_minimum_weight_perfect_matching, minimum_weight_perfect_matching,
};
use blossom_study::graph::{Edge, Graph};

fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Graph {
    Graph::from_edges(
        n,
        edges
            .iter()
            .map(|&(u, v, weight)| Edge::new(u, v, Some(weight)))
            .collect(),
    )
}

fn assert_perfect(graph: &Graph, indices: &[usize]) {
    let mut covered = vec![false; graph.n];
    for &index in indices {
        let edge = &graph.edges[index];
        assert!(!covered[edge.u], "vertex {} appears twice", edge.u);
        assert!(!covered[edge.v], "vertex {} appears twice", edge.v);
        covered[edge.u] = true;
        covered[edge.v] = true;
    }
    assert!(covered.into_iter().all(|value| value));
}

fn brute_force(cost: &[Vec<f64>], mask: u32) -> f64 {
    if mask == 0 {
        return 0.0;
    }
    let u = mask.trailing_zeros() as usize;
    let remaining = mask & !(1 << u);
    let mut candidates = remaining;
    let mut best = f64::INFINITY;
    while candidates != 0 {
        let v = candidates.trailing_zeros() as usize;
        candidates &= candidates - 1;
        if cost[u][v].is_finite() {
            best = best.min(cost[u][v] + brute_force(cost, remaining & !(1 << v)));
        }
    }
    best
}

#[test]
fn solves_supplied_diamond_case() {
    let graph = graph(
        4,
        &[
            (0, 1, 10.0),
            (0, 2, 5.0),
            (1, 2, 2.0),
            (1, 3, 4.0),
            (2, 3, 10.0),
        ],
    );
    let result = BlossomEngine::new().solve(&graph).unwrap();
    assert_perfect(&graph, &result.edges);
    assert_eq!(result.total_weight, 9.0);
}

#[test]
fn supports_negative_fractional_and_parallel_edges() {
    let graph = graph(
        4,
        &[
            (0, 1, 5.5),
            (0, 1, -4.25),
            (2, 3, -3.5),
            (0, 2, 10.0),
            (1, 3, 10.0),
        ],
    );
    let result = blossom_engine_minimum_weight_perfect_matching(&graph).unwrap();
    assert_perfect(&graph, &result.edges);
    assert_eq!(result.total_weight, -7.75);
    assert!(result.edges.contains(&1));
}

#[test]
fn agrees_with_brute_force_on_dense_random_graphs() {
    let mut state = 0x5eed_cafe_u64;
    for n in [2, 4, 6, 8, 10] {
        let case_count = if n < 10 { 500 } else { 100 };
        for case in 0..case_count {
            let mut costs = vec![vec![f64::INFINITY; n]; n];
            let mut edges = Vec::new();
            let mut u = 0;
            while u < n {
                let mut v = u + 1;
                while v < n {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let weight = ((state >> 32) % 401) as f64 - 200.0;
                    costs[u][v] = weight;
                    costs[v][u] = weight;
                    edges.push((u, v, weight));
                    v += 1;
                }
                u += 1;
            }
            let graph = graph(n, &edges);
            let result = blossom_engine_minimum_weight_perfect_matching(&graph)
                .unwrap_or_else(|error| panic!("n={n}, case={case}, edges={edges:?}: {error}"));
            assert_perfect(&graph, &result.edges);
            let expected = brute_force(&costs, (1_u32 << n) - 1);
            assert_eq!(result.total_weight, expected, "n={n}, case={case}");
        }
    }
}

#[test]
fn agrees_with_existing_solver_on_sparse_planted_instances() {
    let n = 10;
    let mut state = 0x1234_5678_u64;
    for case in 0..500 {
        let mut edges = Vec::new();
        // Plant a perfect matching so every generated graph is feasible.
        for u in (0..n).step_by(2) {
            edges.push((u, u + 1, ((u + case) % 17) as f64 - 8.0));
        }
        for u in 0..n {
            for v in (u + 1)..n {
                state = state
                    .wrapping_mul(2862933555777941757)
                    .wrapping_add(3037000493);
                if state.is_multiple_of(4) && !edges.iter().any(|&(a, b, _)| a == u && b == v) {
                    edges.push((u, v, ((state >> 32) % 101) as f64 - 50.0));
                }
            }
        }
        let graph = graph(n, &edges);
        let expected = minimum_weight_perfect_matching(&graph).unwrap();
        let actual = blossom_engine_minimum_weight_perfect_matching(&graph)
            .unwrap_or_else(|error| panic!("case={case}: {error}"));
        assert_perfect(&graph, &actual.edges);
        assert_eq!(actual.total_weight, expected.total_weight, "case={case}");
    }
}

#[test]
fn rejects_graphs_without_a_perfect_matching() {
    let odd = graph(3, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
    assert!(blossom_engine_minimum_weight_perfect_matching(&odd).is_err());

    let infeasible = graph(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
    assert!(blossom_engine_minimum_weight_perfect_matching(&infeasible).is_err());

    // Every vertex has positive degree, but two disjoint triangles still have
    // no perfect matching.  This exercises solver-level infeasibility rather
    // than the adapter's isolated-vertex check.
    let two_triangles = graph(
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
    assert!(blossom_engine_minimum_weight_perfect_matching(&two_triangles).is_err());
}

#[test]
fn handles_equal_weights_and_nested_odd_cycles() {
    let graph = graph(
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
    let result = blossom_engine_minimum_weight_perfect_matching(&graph).unwrap();
    assert_perfect(&graph, &result.edges);
    assert_eq!(result.total_weight, 4.0);
    assert!(result.metadata.blossoms_created > 0);
}
