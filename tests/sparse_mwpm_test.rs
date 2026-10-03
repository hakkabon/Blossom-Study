use blossom_study::graph::{Edge, Graph};
use blossom_study::{minimum_weight_perfect_matching, sparse_minimum_weight_perfect_matching};

fn graph(n: usize, edges: &[(usize, usize, f64)]) -> Graph {
    Graph::from_edges(
        n,
        edges.iter().map(|&(u, v, w)| Edge::new(u, v, Some(w))).collect(),
    )
}

fn assert_valid_matching(graph: &Graph, result: &blossom_study::algorithms::MatchingResult) {
    let mut matched = vec![false; graph.n];
    for &idx in &result.edges {
        let edge = &graph.edges[idx];
        assert!(!matched[edge.u], "vertex {} matched multiple times", edge.u);
        assert!(!matched[edge.v], "vertex {} matched multiple times", edge.v);
        matched[edge.u] = true;
        matched[edge.v] = true;
    }
    assert!(matched.iter().all(|&m| m), "not all vertices were matched");
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
        best = best.min(cost[u][v] + brute_force(cost, remaining & !(1 << v)));
    }
    best
}

#[test]
fn test_sparse_mwpm_matches_brute_force_oracle() {
    let n = 8;
    let mut state = 0x5eed_u64;
    for trial in 0..40 {
        let mut costs = vec![vec![0.0; n]; n];
        let mut edges = Vec::new();
        for u in 0..n {
            for v in (u + 1)..n {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let weight = ((state >> 32) % 401) as f64 / 10.0 - 20.0;
                edges.push((u, v, weight));
                costs[u][v] = weight;
                costs[v][u] = weight;
            }
        }
        let g = graph(n, &edges);
        let sparse_result = sparse_minimum_weight_perfect_matching(&g).unwrap();
        assert_valid_matching(&g, &sparse_result);

        let dense_result = minimum_weight_perfect_matching(&g).unwrap();
        let expected = brute_force(&costs, (1 << n) - 1);

        assert!(
            (sparse_result.total_weight - expected).abs() < 1e-9,
            "trial {}: sparse ({}) != oracle ({})",
            trial,
            sparse_result.total_weight,
            expected
        );
        assert!(
            (sparse_result.total_weight - dense_result.total_weight).abs() < 1e-9,
            "trial {}: sparse ({}) != dense ({})",
            trial,
            sparse_result.total_weight,
            dense_result.total_weight
        );
    }
}

#[test]
fn test_sparse_mwpm_matches_dense_on_complete_graphs() {
    for n in [6, 10, 16, 20] {
        let g = blossom_study::graph::generator::complete_graph(n, (1.0, 50.0));
        let sparse_res = sparse_minimum_weight_perfect_matching(&g).unwrap();
        let dense_res = minimum_weight_perfect_matching(&g).unwrap();

        assert_valid_matching(&g, &sparse_res);
        assert!(
            (sparse_res.total_weight - dense_res.total_weight).abs() < 1e-9,
            "n={}: sparse ({}) != dense ({})",
            n,
            sparse_res.total_weight,
            dense_res.total_weight
        );
    }
}

#[test]
fn test_sparse_mwpm_on_sparse_grid() {
    let g = blossom_study::graph::generator::grid_graph(4, 4, (1.0, 20.0));
    let sparse_res = sparse_minimum_weight_perfect_matching(&g).unwrap();
    let dense_res = minimum_weight_perfect_matching(&g).unwrap();

    assert_valid_matching(&g, &sparse_res);
    assert!(
        (sparse_res.total_weight - dense_res.total_weight).abs() < 1e-9,
        "grid 4x4: sparse ({}) != dense ({})",
        sparse_res.total_weight,
        dense_res.total_weight
    );
}

#[test]
fn test_sparse_mwpm_edge_cases() {
    // Odd vertices
    let g_odd = graph(3, &[(0, 1, 1.0), (1, 2, 2.0)]);
    assert!(sparse_minimum_weight_perfect_matching(&g_odd).is_err());

    // Disconnected / impossible
    let g_disc = graph(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
    assert!(sparse_minimum_weight_perfect_matching(&g_disc).is_err());

    // Isolated vertex
    let g_iso = graph(4, &[(0, 1, 1.0)]);
    assert!(sparse_minimum_weight_perfect_matching(&g_iso).is_err());

    // Negative and parallel edges
    let g_neg = graph(
        4,
        &[
            (0, 1, 5.0),
            (0, 1, -4.0),
            (2, 3, -3.0),
            (0, 2, 10.0),
            (1, 3, 10.0),
        ],
    );
    let res_neg = sparse_minimum_weight_perfect_matching(&g_neg).unwrap();
    assert_valid_matching(&g_neg, &res_neg);
    assert_eq!(res_neg.total_weight, -7.0);

    // Empty graph
    let g_empty = graph(0, &[]);
    let res_empty = sparse_minimum_weight_perfect_matching(&g_empty).unwrap();
    assert_eq!(res_empty.edges.len(), 0);
    assert_eq!(res_empty.total_weight, 0.0);
}
