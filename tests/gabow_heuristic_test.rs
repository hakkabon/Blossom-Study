use blossom_study::{
    Edge, FastBlossomMatching, GabowHeuristic, Graph, MicaliVazirani, gabow_heuristic_matching,
};

#[test]
fn gabow_solves_empty_and_single_edge_graphs() {
    let empty_graph = Graph::from_edges(0, vec![]);
    let res = gabow_heuristic_matching(&empty_graph);
    assert_eq!(res.edges.len(), 0);
    assert!(res.is_perfect);

    let single_edge = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    let res = gabow_heuristic_matching(&single_edge);
    assert_eq!(res.edges.len(), 1);
    assert!(res.is_perfect);

    let triangle = Graph::from_edges(
        3,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 0, None),
        ],
    );
    let res = gabow_heuristic_matching(&triangle);
    assert_eq!(res.edges.len(), 1);
    assert!(!res.is_perfect);
}

#[test]
fn gabow_solves_pentagon_with_tail() {
    // 5-cycle (0-1-2-3-4-0) with a tail attached at 0 (edge 0-5)
    let graph = Graph::from_edges(
        6,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
            Edge::new(3, 4, None),
            Edge::new(4, 0, None),
            Edge::new(0, 5, None),
        ],
    );
    let mut solver = GabowHeuristic::new(&graph);
    let result = solver.max_cardinality_matching(&graph);
    assert_eq!(result.edges.len(), 3);
    assert!(result.is_perfect);
}

#[test]
fn gabow_solves_nested_blossoms() {
    let graph = Graph::from_edges(
        8,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 0, None),
            Edge::new(2, 3, None),
            Edge::new(3, 4, None),
            Edge::new(4, 5, None),
            Edge::new(5, 6, None),
            Edge::new(6, 4, None),
            Edge::new(0, 7, None),
            Edge::new(6, 7, None),
        ],
    );
    let mut solver = GabowHeuristic::new(&graph);
    let result = solver.max_cardinality_matching(&graph);
    assert_eq!(result.edges.len(), 4);
    assert!(result.is_perfect);
}

#[test]
fn gabow_solves_nested_petal_length_nine_graph() {
    let pairs = [
        (0, 1),
        (0, 4),
        (1, 2),
        (1, 4),
        (1, 5),
        (2, 5),
        (2, 6),
        (2, 8),
        (3, 7),
        (4, 5),
        (4, 9),
        (5, 8),
        (5, 9),
        (6, 7),
    ];
    let graph = Graph::from_edges(
        10,
        pairs.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
    );
    let initial = vec![
        None,
        Some(4),
        Some(8),
        None,
        Some(1),
        Some(9),
        Some(7),
        Some(6),
        Some(2),
        Some(5),
    ];
    let mut solver = GabowHeuristic::with_matching(&graph, initial).unwrap();
    let result = solver.max_cardinality_matching(&graph);
    assert_eq!(result.edges.len(), 5);
    assert!(result.is_perfect);
}

/// A missed bridge once made Phase 1 claim completion with exposed vertices
/// 6 and 7. The path 6--2==5--3==1--4==0--7 augments to four pairs.
#[test]
fn gabow_does_not_stop_at_false_optimum_on_eight_vertices() {
    let pairs = [
        (0, 2),
        (0, 4),
        (0, 5),
        (0, 6),
        (0, 7),
        (1, 3),
        (1, 4),
        (2, 5),
        (2, 6),
        (3, 4),
        (3, 5),
    ];
    let graph = Graph::from_edges(
        8,
        pairs.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
    );
    let mut solver = GabowHeuristic::new(&graph);
    let result = solver.max_cardinality_matching(&graph);
    assert_eq!(result.edges.len(), 4);
    assert!(result.is_perfect);
    for (u, mate) in solver.mate().iter().enumerate() {
        if let Some(v) = mate {
            assert_eq!(solver.mate()[*v], Some(u));
            assert!(graph.edge_index(u, *v).is_some());
        }
    }
}

#[test]
fn gabow_matches_mv_and_fast_blossom_on_diverse_graphs() {
    let mut rng_state = 987654321_u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_state >> 33) as u32
    };

    for n in [6, 8, 10, 12, 16] {
        for _seed in 0..8 {
            let mut edges = Vec::new();
            for u in 0..n {
                for v in (u + 1)..n {
                    if (next_rand() % 100) < 30 {
                        edges.push(Edge::new(u, v, None));
                    }
                }
            }
            let graph = Graph::from_edges(n, edges);

            let fb_result = FastBlossomMatching::max_cardinality_matching(&graph);
            let mut mv_solver = MicaliVazirani::new(&graph);
            let mv_result = mv_solver.max_cardinality_matching(&graph).unwrap();

            let mut gabow_solver = GabowHeuristic::new(&graph);
            let gabow_result = gabow_solver.max_cardinality_matching(&graph);

            assert_eq!(
                gabow_result.edges.len(),
                fb_result.edges.len(),
                "Gabow differs from FastBlossom on n={n}"
            );
            assert_eq!(
                gabow_result.edges.len(),
                mv_result.edges.len(),
                "Gabow differs from MV on n={n}"
            );
            assert_eq!(gabow_result.is_perfect, fb_result.is_perfect);

            // Verify matching validity
            for (u, mate) in gabow_solver.mate().iter().enumerate() {
                if let Some(v) = mate {
                    assert_ne!(u, *v);
                    assert_eq!(gabow_solver.mate()[*v], Some(u));
                    assert!(graph.edge_index(u, *v).is_some());
                }
            }
        }
    }
}

fn brute_maximum_cardinality(n: usize, edges: &[(usize, usize)], used: u64) -> usize {
    let Some(u) = (0..n).find(|&v| used & (1 << v) == 0) else {
        return 0;
    };
    let without_u = used | (1 << u);
    let mut best = brute_maximum_cardinality(n, edges, without_u);
    for &(a, b) in edges {
        let v = if a == u {
            b
        } else if b == u {
            a
        } else {
            continue;
        };
        if used & (1 << v) == 0 {
            best = best.max(1 + brute_maximum_cardinality(n, edges, without_u | (1 << v)));
        }
    }
    best
}

#[test]
fn gabow_agrees_with_exhaustive_small_graph_oracle() {
    for n in 0..=6 {
        let possible_edges: Vec<_> = (0..n)
            .flat_map(|u| (u + 1..n).map(move |v| (u, v)))
            .collect();
        for mask in 0..(1_u64 << possible_edges.len()) {
            let chosen: Vec<_> = possible_edges
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, &pair)| pair)
                .collect();
            let graph = Graph::from_edges(
                n,
                chosen.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
            );
            let expected = brute_maximum_cardinality(n, &chosen, 0);

            let mut solver = GabowHeuristic::new(&graph);
            let result = solver.max_cardinality_matching(&graph);
            assert_eq!(result.edges.len(), expected);
            assert_eq!(result.is_perfect, expected * 2 == n);
        }
    }
}
