use blossom_study::algorithms::{
    EdmondsAlgorithm, FastBlossomMatching, KorteVygenEdmonds, MatchingResult,
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

fn assert_valid(graph: &Graph, result: &MatchingResult) {
    let mut covered = vec![false; graph.n];
    for &edge_index in &result.edges {
        let edge = &graph.edges[edge_index];
        assert!(!covered[edge.u] && !covered[edge.v]);
        covered[edge.u] = true;
        covered[edge.v] = true;
    }
    assert_eq!(result.is_perfect, covered.iter().all(|&value| value));
}

#[test]
fn fast_blossom_handles_pentagon_with_tail() {
    let graph = graph(
        6,
        &[
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 3, 1.0),
            (3, 4, 1.0),
            (4, 0, 1.0),
            (0, 5, 1.0),
        ],
    );
    let result = FastBlossomMatching::max_cardinality_matching(&graph);
    assert_valid(&graph, &result);
    assert_eq!(result.edges.len(), 3);
}

#[test]
fn fast_blossom_handles_nested_blossoms() {
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
    let result = FastBlossomMatching::max_cardinality_matching(&graph);
    assert_valid(&graph, &result);
    assert_eq!(result.edges.len(), 4);
}

#[test]
fn all_algorithms_find_the_same_minimum_weight() {
    let graph = graph(
        6,
        &[
            (0, 1, 9.0),
            (0, 2, 2.0),
            (0, 3, 8.0),
            (0, 4, 7.0),
            (0, 5, 6.0),
            (1, 2, 7.0),
            (1, 3, 3.0),
            (1, 4, 8.0),
            (1, 5, 9.0),
            (2, 3, 8.0),
            (2, 4, 9.0),
            (2, 5, 7.0),
            (3, 4, 6.0),
            (3, 5, 8.0),
            (4, 5, 1.0),
        ],
    );

    let results = [
        EdmondsAlgorithm::new(&graph)
            .min_weight_perfect(&graph)
            .unwrap(),
        KorteVygenEdmonds::min_weight_perfect_matching(&graph).unwrap(),
        FastBlossomMatching::min_weight_perfect_matching(&graph).unwrap(),
    ];
    for result in &results {
        assert_valid(&graph, result);
        assert_eq!(result.total_weight, 6.0);
    }
}

#[test]
fn minimum_weight_reports_impossible_graphs() {
    let graph = graph(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
    assert!(FastBlossomMatching::min_weight_perfect_matching(&graph).is_err());
}

#[test]
fn minimum_weight_supports_negative_and_parallel_edges() {
    let graph = graph(
        4,
        &[
            (0, 1, 5.0),
            (0, 1, -4.0),
            (2, 3, -3.0),
            (0, 2, 10.0),
            (1, 3, 10.0),
        ],
    );
    let result = FastBlossomMatching::min_weight_perfect_matching(&graph).unwrap();
    assert_valid(&graph, &result);
    assert_eq!(result.total_weight, -7.0);
    assert!(result.edges.contains(&1));
}

#[test]
fn polynomial_solver_handles_more_than_twenty_four_vertices() {
    let n = 30;
    let mut edges = Vec::new();
    for u in 0..n {
        for v in (u + 1)..n {
            let weight = if u / 2 == v / 2 { 1.0 } else { 100.0 };
            edges.push((u, v, weight));
        }
    }
    let graph = graph(n, &edges);
    let result = FastBlossomMatching::min_weight_perfect_matching(&graph).unwrap();
    assert_valid(&graph, &result);
    assert_eq!(result.total_weight, 15.0);
}

#[test]
fn primal_dual_solver_matches_brute_force_oracle() {
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

    let n = 8;
    let mut state = 0x5eed_u64;
    for _ in 0..40 {
        let mut costs = vec![vec![0.0; n]; n];
        let mut edges = Vec::new();
        for u in 0..n {
            for v in (u + 1)..n {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let weight = ((state >> 32) % 401) as f64 / 10.0 - 20.0;
                edges.push((u, v, weight));
            }
        }
        for &(u, v, weight) in &edges {
            costs[u][v] = weight;
            costs[v][u] = weight;
        }
        let graph = graph(n, &edges);
        let result = FastBlossomMatching::min_weight_perfect_matching(&graph).unwrap();
        let expected = brute_force(&costs, (1 << n) - 1);
        assert!((result.total_weight - expected).abs() < 1e-9);
    }
}
