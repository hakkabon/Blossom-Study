//! Correctness tests for the bipartite Hungarian benchmark baseline.

use blossom_study::{
    Edge, Graph, HungarianSolver, hungarian_minimum_weight_perfect_matching,
    minimum_weight_perfect_matching,
};

fn bipartite_graph(costs: &[Vec<Option<f64>>]) -> Graph {
    let size = costs.len();
    assert!(costs.iter().all(|row| row.len() == size));
    let mut edges = Vec::new();
    for (left, row) in costs.iter().enumerate() {
        for (right, cost) in row.iter().copied().enumerate() {
            if let Some(cost) = cost {
                edges.push(Edge::new(left, size + right, Some(cost)));
            }
        }
    }
    Graph::from_edges(2 * size, edges)
}

fn assert_bipartite_perfect(graph: &Graph, left_size: usize, edges: &[usize]) {
    assert_eq!(edges.len(), left_size);
    let mut covered = vec![false; graph.n];
    for &index in edges {
        let edge = &graph.edges[index];
        assert_ne!(edge.u < left_size, edge.v < left_size);
        assert!(!covered[edge.u]);
        assert!(!covered[edge.v]);
        covered[edge.u] = true;
        covered[edge.v] = true;
    }
    assert!(covered.into_iter().all(|value| value));
}

#[test]
fn solves_the_classic_three_by_three_assignment() {
    let graph = bipartite_graph(&[
        vec![Some(4.0), Some(1.0), Some(3.0)],
        vec![Some(2.0), Some(0.0), Some(5.0)],
        vec![Some(3.0), Some(2.0), Some(2.0)],
    ]);
    let result = HungarianSolver::new(3).solve(&graph).unwrap();
    assert_bipartite_perfect(&graph, 3, &result.edges);
    assert_eq!(result.total_weight, 5.0);
    assert_eq!(result.metadata.blossoms_created, 0);
    assert_eq!(result.metadata.augmentations, 3);
}

#[test]
fn supports_sparse_negative_fractional_and_parallel_edges() {
    let mut graph = bipartite_graph(&[
        vec![Some(8.0), Some(-1.25), None],
        vec![Some(2.5), None, Some(-4.5)],
        vec![None, Some(1.0), Some(9.0)],
    ]);
    // A more expensive parallel copy must not displace the cheaper 0--4 edge.
    graph.edges.push(Edge::new(0, 4, Some(12.0)));
    graph = Graph::from_edges(graph.n, graph.edges);

    let result = hungarian_minimum_weight_perfect_matching(&graph, 3).unwrap();
    assert_bipartite_perfect(&graph, 3, &result.edges);
    assert_eq!(result.total_weight, 4.5);
}

#[test]
fn reports_partition_and_hall_violations() {
    let wrong_size = Graph::from_edges(3, vec![Edge::new(0, 2, Some(1.0))]);
    assert!(HungarianSolver::new(1).solve(&wrong_size).is_err());

    let inside_partition = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, Some(1.0)),
            Edge::new(0, 2, Some(1.0)),
            Edge::new(1, 3, Some(1.0)),
        ],
    );
    assert!(HungarianSolver::new(2).solve(&inside_partition).is_err());

    // Both left vertices can reach right vertex 2, but neither can reach 3.
    let hall_violation = Graph::from_edges(
        4,
        vec![Edge::new(0, 2, Some(1.0)), Edge::new(1, 2, Some(2.0))],
    );
    assert!(HungarianSolver::new(2).solve(&hall_violation).is_err());
}

#[test]
fn sparse_frontier_keeps_slack_discovered_by_an_earlier_tree_row() {
    // During the second augmentation, row 1 first reaches the occupied column
    // 0 and records column 1 as an alternative. Row 0 has no edge to column 1,
    // so the solver must retain row 1's earlier slack across the tree growth.
    let graph = bipartite_graph(&[vec![Some(0.0), None], vec![Some(0.0), Some(5.0)]]);
    let result = HungarianSolver::new(2).solve(&graph).unwrap();
    assert_bipartite_perfect(&graph, 2, &result.edges);
    assert_eq!(result.total_weight, 5.0);
}

#[test]
fn agrees_with_the_general_graph_solver_on_random_assignments() {
    let mut state = 0xa551_6e5d_5eed_u64;
    for size in 1..=7 {
        for case in 0..150 {
            let mut costs = vec![vec![None; size]; size];
            for row in &mut costs {
                for cost in row {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    // Binary fractions cover negative and non-integral values
                    // while remaining exactly representable by f64.
                    *cost = Some(((state >> 32) % 1601) as f64 / 8.0 - 100.0);
                }
            }
            let graph = bipartite_graph(&costs);
            let expected = minimum_weight_perfect_matching(&graph).unwrap();
            let actual = HungarianSolver::new(size)
                .solve(&graph)
                .unwrap_or_else(|error| panic!("size={size}, case={case}: {error}"));
            assert_bipartite_perfect(&graph, size, &actual.edges);
            assert_eq!(actual.total_weight, expected.total_weight, "case={case}");
        }
    }
}
