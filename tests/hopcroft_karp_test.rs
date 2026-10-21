//! Independent correctness checks for the bipartite cardinality baseline.

use blossom_study::{Edge, Graph, HopcroftKarpSolver, hopcroft_karp_maximum_cardinality_matching};

fn assert_valid_matching(graph: &Graph, left_size: usize, indices: &[usize]) {
    let mut covered = vec![false; graph.n];
    for &index in indices {
        let edge = &graph.edges[index];
        assert_ne!(edge.u < left_size, edge.v < left_size);
        assert!(!covered[edge.u], "left or right endpoint used twice");
        assert!(!covered[edge.v], "left or right endpoint used twice");
        covered[edge.u] = true;
        covered[edge.v] = true;
    }
}

#[test]
fn finds_an_augmenting_path_in_a_later_bfs_phase() {
    // Initially 0--2 is chosen. The second root can reach the remaining free
    // right vertex only by the alternating path 1--2==0--3, so a one-pass
    // greedy matcher would stop too early.
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 2, None),
            Edge::new(0, 3, None),
            Edge::new(1, 2, None),
        ],
    );
    let result = HopcroftKarpSolver::new(2).solve(&graph).unwrap();
    assert_valid_matching(&graph, 2, &result.edges);
    assert_eq!(result.edges, vec![1, 2]);
    assert_eq!(result.metadata.augmentations, 2);
    assert_eq!(result.total_weight, 2.0);
    assert!(result.is_perfect);
}

#[test]
fn handles_unbalanced_and_nonperfect_bipartite_graphs() {
    let graph = Graph::from_edges(
        5,
        vec![
            Edge::new(0, 3, None),
            Edge::new(1, 3, None),
            Edge::new(2, 4, None),
        ],
    );
    let result = hopcroft_karp_maximum_cardinality_matching(&graph, 3).unwrap();
    assert_valid_matching(&graph, 3, &result.edges);
    assert_eq!(result.edges.len(), 2);
    assert!(!result.is_perfect);

    let empty = Graph::from_edges(0, vec![]);
    let result = HopcroftKarpSolver::new(0).solve(&empty).unwrap();
    assert!(result.edges.is_empty());
    assert!(result.is_perfect);

    let isolated = Graph::from_edges(3, vec![]);
    let result = HopcroftKarpSolver::new(1).solve(&isolated).unwrap();
    assert!(result.edges.is_empty());
    assert!(!result.is_perfect);
}

#[test]
fn ignores_costs_for_selection_and_preserves_original_parallel_edge_index() {
    let graph = Graph::from_edges(
        2,
        vec![Edge::new(1, 0, Some(100.0)), Edge::new(0, 1, Some(-50.0))],
    );
    let result = HopcroftKarpSolver::new(1).solve(&graph).unwrap();
    assert_eq!(result.edges, vec![0]);
    assert_eq!(result.total_weight, 100.0);
    assert!(result.is_perfect);
}

#[test]
fn rejects_edges_inside_a_partition() {
    let invalid = Graph::from_edges(4, vec![Edge::new(0, 1, None)]);
    assert!(HopcroftKarpSolver::new(2).solve(&invalid).is_err());
    assert!(HopcroftKarpSolver::new(5).solve(&invalid).is_err());

    let invalid_right = Graph::from_edges(4, vec![Edge::new(2, 3, None)]);
    assert!(HopcroftKarpSolver::new(2).solve(&invalid_right).is_err());
}

/// A deliberately simple exponential oracle, independent of BFS and DFS.
/// It may leave a left vertex unmatched, then tries each still-free neighbor.
fn brute_force_cardinality(neighbors: &[u64], left: usize, used_right: u64) -> usize {
    if left == neighbors.len() {
        return 0;
    }
    let mut best = brute_force_cardinality(neighbors, left + 1, used_right);
    let mut available = neighbors[left] & !used_right;
    while available != 0 {
        let right_bit = available & available.wrapping_neg();
        available ^= right_bit;
        best = best.max(1 + brute_force_cardinality(neighbors, left + 1, used_right | right_bit));
    }
    best
}

#[test]
fn agrees_with_an_exhaustive_oracle_on_all_small_bipartite_graphs() {
    for left_size in 1..=4 {
        for right_size in 1..=3 {
            let possible_edges = left_size * right_size;
            for edge_mask in 0..(1_usize << possible_edges) {
                let mut edges = Vec::new();
                let mut neighbors = vec![0_u64; left_size];
                for left in 0..left_size {
                    for right in 0..right_size {
                        if edge_mask & (1 << (left * right_size + right)) != 0 {
                            edges.push(Edge::new(left, left_size + right, None));
                            neighbors[left] |= 1 << right;
                        }
                    }
                }
                let graph = Graph::from_edges(left_size + right_size, edges);
                let result = HopcroftKarpSolver::new(left_size)
                    .solve(&graph)
                    .unwrap_or_else(|error| panic!("mask={edge_mask}: {error}"));
                assert_valid_matching(&graph, left_size, &result.edges);
                assert_eq!(
                    result.edges.len(),
                    brute_force_cardinality(&neighbors, 0, 0),
                    "left={left_size}, right={right_size}, mask={edge_mask}"
                );
                assert_eq!(result.is_perfect, result.edges.len() * 2 == graph.n);
            }
        }
    }
}
