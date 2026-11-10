mod common;

use blossom_study::{
    DdfsResult, Edge, Graph, MicaliVazirani, MicaliVaziraniError, MicaliVaziraniPhase,
};
use common::assert_valid_matching;

#[test]
fn bridge_processing_milestone_exposes_augmentation_phase() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
            Edge::new(3, 0, None),
        ],
    );
    let mut solver = MicaliVazirani::new(&graph);
    assert_eq!(solver.phase(), MicaliVaziraniPhase::Ready);

    // Milestone verification: bridge processing phase exposes Augmentation phase
    let bottleneck = solver.run_bridge_processing_phase(&graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::Augmentation);
    assert!(solver.mate().iter().all(Option::is_none));
    assert_eq!(solver.even_level(), &[Some(0); 4]);
    assert_eq!(solver.odd_level(), &[Some(1); 4]);
    assert_eq!(solver.min_level(), &[Some(0); 4]);
    assert_eq!(solver.max_level(), &[Some(1); 4]);
    assert_eq!(solver.bridge_candidates(), &[0, 1, 2, 3]);
    assert!(solver.level_search_scans() > 0);
    assert_eq!(bottleneck, Some(1));

    // Full algorithm execution: finds maximum matching of size 2
    let mut solver2 = MicaliVazirani::new(&graph);
    let result = solver2.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 2);
    assert!(result.is_perfect);
    assert_eq!(solver2.phase(), MicaliVaziraniPhase::Finished);
}

#[test]
fn level_search_phase_transitions_to_bridge_processing() {
    let graph = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    let mut solver = MicaliVazirani::new(&graph);
    solver.run_level_search_phase(&graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::BridgeProcessing);
    assert_eq!(solver.bridge_candidates(), &[0]);
    assert!(solver.mate().iter().all(Option::is_none));
}

#[test]
fn level_search_can_be_benchmarked_without_claiming_a_matching() {
    let graph = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    let mut solver = MicaliVazirani::new(&graph);
    solver.run_level_search_phase(&graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::BridgeProcessing);
    assert_eq!(solver.bridge_candidates(), &[0]);
    assert!(solver.mate().iter().all(Option::is_none));
}

#[test]
fn graph_identity_check_is_stronger_than_vertex_and_edge_counts() {
    let original = Graph::from_edges(4, vec![Edge::new(0, 1, None)]);
    let changed = Graph::from_edges(4, vec![Edge::new(2, 3, None)]);
    let mut solver = MicaliVazirani::new(&original);
    assert_eq!(
        solver.run_level_search_phase(&changed),
        Err(MicaliVaziraniError::GraphChanged)
    );
}

#[test]
fn isolated_vertices_and_empty_graphs_produce_valid_level_state() {
    let empty_graph = Graph::from_edges(0, vec![]);
    let mut solver = MicaliVazirani::new(&empty_graph);
    solver.run_level_search_phase(&empty_graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::BridgeProcessing);
    assert!(solver.bridge_candidates().is_empty());

    let isolated = Graph::from_edges(3, vec![]);
    let mut solver = MicaliVazirani::new(&isolated);
    solver.run_level_search_phase(&isolated).unwrap();
    assert_eq!(solver.even_level(), &[Some(0), Some(0), Some(0)]);
    assert_eq!(solver.odd_level(), &[None, None, None]);
    assert_eq!(solver.min_level(), &[Some(0), Some(0), Some(0)]);
    assert_eq!(solver.max_level(), &[None, None, None]);
    assert!(solver.bridge_candidates().is_empty());
}

#[test]
fn single_edge_empty_vs_matched() {
    let graph = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);

    // Case 1: Empty matching -> both exposed -> bridge of tenacity 1
    let mut solver = MicaliVazirani::new(&graph);
    solver.run_level_search_phase(&graph).unwrap();
    assert_eq!(solver.even_level(), &[Some(0), Some(0)]);
    assert_eq!(solver.odd_level(), &[Some(1), Some(1)]);
    assert!(solver.is_bridge(0));
    assert_eq!(solver.bridge_tenacity(&graph, 0), Some(1));

    // Case 2: Already matched -> no exposed roots -> no search progression
    let mate = vec![Some(1), Some(0)];
    let mut matched_solver = MicaliVazirani::with_matching(&graph, mate).unwrap();
    matched_solver.run_level_search_phase(&graph).unwrap();
    assert_eq!(matched_solver.even_level(), &[None, None]);
    assert_eq!(matched_solver.odd_level(), &[None, None]);
    assert!(matched_solver.bridge_candidates().is_empty());
}

#[test]
fn alternating_path_chain_tracks_parity_and_props() {
    // Path: 0 - 1 = 2
    // 0 is exposed; edge (1, 2) is matched.
    let graph = Graph::from_edges(
        3,
        vec![
            Edge::new(0, 1, None), // index 0 (unmatched)
            Edge::new(1, 2, None), // index 1 (matched)
        ],
    );
    let mate = vec![None, Some(2), Some(1)];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();
    solver.run_level_search_phase(&graph).unwrap();

    // 0 is exposed (even 0)
    // 1 reached via unmatched edge (odd 1)
    // 2 reached via matched edge (even 2)
    assert_eq!(solver.even_level(), &[Some(0), None, Some(2)]);
    assert_eq!(solver.odd_level(), &[None, Some(1), None]);
    assert_eq!(solver.min_level(), &[Some(0), Some(1), Some(2)]);
    assert_eq!(solver.max_level(), &[None, None, None]);

    // Props:
    // edge 0 is a prop into 1
    // edge 1 is a prop into 2
    assert!(solver.is_predecessor_edge(1, 0));
    assert!(solver.is_predecessor_edge(2, 1));
    assert!(solver.bridge_candidates().is_empty());
}

#[test]
fn pentagon_odd_cycle_detects_blossom_bridge_with_tenacity_five() {
    // Pentagon cycle C5:
    // 0 - 1 = 2 - 3 = 4 - 0
    // Root 0 is exposed.
    // Matched edges: (1, 2) and (3, 4).
    // Bridge edge: (2, 3) connects two even vertices at level 2!
    let graph = Graph::from_edges(
        5,
        vec![
            Edge::new(0, 1, None), // 0: unmatched
            Edge::new(1, 2, None), // 1: matched
            Edge::new(2, 3, None), // 2: unmatched bridge
            Edge::new(3, 4, None), // 3: matched
            Edge::new(4, 0, None), // 4: unmatched
        ],
    );
    let mate = vec![None, Some(2), Some(1), Some(4), Some(3)];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();
    solver.run_level_search_phase(&graph).unwrap();

    // Check parity levels:
    // Root 0: even 0
    // 1: odd 1 (from 0 via unmatched edge 0)
    // 2: even 2 (from 1 via matched edge 1)
    // 4: odd 1 (from 0 via unmatched edge 4)
    // 3: even 2 (from 4 via matched edge 3)
    assert_eq!(solver.even_level()[0], Some(0));
    assert_eq!(solver.odd_level()[1], Some(1));
    assert_eq!(solver.even_level()[2], Some(2));
    assert_eq!(solver.odd_level()[4], Some(1));
    assert_eq!(solver.even_level()[3], Some(2));

    // Both 2 and 3 are at even level 2.
    // Edge (2, 3) (index 2) must be classified as a bridge!
    assert!(
        solver.is_bridge(2),
        "Edge (2, 3) must be a bridge candidate"
    );
    assert_eq!(
        solver.bridge_tenacity(&graph, 2),
        Some(5),
        "Tenacity of pentagon bridge (2, 3) must be 2 + 2 + 1 = 5"
    );

    // Props verify alternating tree structure:
    assert!(solver.is_predecessor_edge(1, 0));
    assert!(solver.is_predecessor_edge(2, 1));
    assert!(solver.is_predecessor_edge(4, 4));
    assert!(solver.is_predecessor_edge(3, 3));
    assert!(
        !solver.is_predecessor_edge(2, 2) && !solver.is_predecessor_edge(3, 2),
        "Bridge (2, 3) should not be a predecessor prop"
    );
}

#[test]
fn disjoint_trees_augmenting_bridge_tenacity_equals_path_length() {
    // Tree A: 0 - 1 = 2 (0 exposed)
    // Tree B: 5 - 4 = 3 (5 exposed)
    // Bridge: (2, 3) connecting Tree A and Tree B.
    // Total augmenting path: 0 - 1 = 2 - 3 = 4 - 5 (length 5).
    let graph = Graph::from_edges(
        6,
        vec![
            Edge::new(0, 1, None), // 0
            Edge::new(1, 2, None), // 1 (matched)
            Edge::new(2, 3, None), // 2 (bridge)
            Edge::new(3, 4, None), // 3 (matched)
            Edge::new(4, 5, None), // 4
        ],
    );
    let mate = vec![None, Some(2), Some(1), Some(4), Some(3), None];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();
    solver.run_level_search_phase(&graph).unwrap();

    assert_eq!(solver.even_level()[2], Some(2));
    assert_eq!(solver.even_level()[3], Some(2));
    assert!(solver.is_bridge(2));
    assert_eq!(
        solver.bridge_tenacity(&graph, 2),
        Some(5),
        "Tenacity of cross-tree bridge must match augmenting path length 5"
    );
}

#[test]
fn bipartite_graph_single_tree_has_no_dual_parity_levels() {
    // In a bipartite alternating tree with a single exposed root,
    // every reachable vertex has strictly one parity:
    // partition L has only even levels, and partition R has only odd levels.
    // Therefore, no petals are formed and max_level remains None everywhere.
    // Path: 0 - 1 = 2 - 3 = 4
    let graph = Graph::from_edges(
        5,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
            Edge::new(3, 4, None),
        ],
    );
    let mate = vec![None, Some(2), Some(1), Some(4), Some(3)];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();
    solver.run_level_search_phase(&graph).unwrap();

    // 0: even 0
    // 1: odd 1
    // 2: even 2
    // 3: odd 3
    // 4: even 4
    assert_eq!(
        solver.even_level(),
        &[Some(0), None, Some(2), None, Some(4)]
    );
    assert_eq!(solver.odd_level(), &[None, Some(1), None, Some(3), None]);
    assert!(
        solver.max_level().iter().all(Option::is_none),
        "Single-root bipartite tree must have no vertices with dual parity"
    );
    assert!(solver.bridge_candidates().is_empty());
}

#[test]
fn matching_validation_rejects_inconsistent_input() {
    let graph = Graph::from_edges(4, vec![Edge::new(0, 1, None), Edge::new(2, 3, None)]);

    // Length mismatch
    assert_eq!(
        MicaliVazirani::with_matching(&graph, vec![None; 3]).unwrap_err(),
        MicaliVaziraniError::InvalidMatching
    );

    // Self-matching
    assert_eq!(
        MicaliVazirani::with_matching(&graph, vec![Some(0), None, None, None]).unwrap_err(),
        MicaliVaziraniError::InvalidMatching
    );

    // Asymmetric matching
    assert_eq!(
        MicaliVazirani::with_matching(&graph, vec![Some(1), None, None, None]).unwrap_err(),
        MicaliVaziraniError::InvalidMatching
    );

    // Matching edge that doesn't exist in the graph (e.g. edge (0, 3))
    assert_eq!(
        MicaliVazirani::with_matching(&graph, vec![Some(3), None, None, Some(0)]).unwrap_err(),
        MicaliVaziraniError::InvalidMatching
    );
}

#[test]
fn tenacity_queue_orders_odd_tenacities_in_o1() {
    use blossom_study::TenacityQueue;

    let mut queue = TenacityQueue::new(20);
    assert!(queue.is_empty());

    // Insert edges with mixed tenacities: 7, 3, 5, 1, 5
    queue.push(10, 7);
    queue.push(20, 3);
    queue.push(30, 5);
    queue.push(40, 1);
    queue.push(50, 5);

    assert_eq!(queue.len(), 5);

    // Must pop in non-decreasing order: 1, 3, 5, 5, 7
    let (edge, tenacity) = queue.pop_min().unwrap();
    assert_eq!((edge, tenacity), (40, 1));

    let (edge, tenacity) = queue.pop_min().unwrap();
    assert_eq!((edge, tenacity), (20, 3));

    let (_, tenacity1) = queue.pop_min().unwrap();
    let (_, tenacity2) = queue.pop_min().unwrap();
    assert_eq!(tenacity1, 5);
    assert_eq!(tenacity2, 5);

    let (edge, tenacity) = queue.pop_min().unwrap();
    assert_eq!((edge, tenacity), (10, 7));

    assert!(queue.pop_min().is_none());
    assert!(queue.is_empty());
}

#[test]
fn ddfs_contracts_pentagon_petal_and_assigns_max_levels() {
    // Pentagon cycle C5:
    // 0 - 1 = 2 - 3 = 4 - 0
    // 0 exposed root, (1, 2) matched, (3, 4) matched.
    // Bridge (2, 3) (edge 2) has tenacity 5.
    let graph = Graph::from_edges(
        5,
        vec![
            Edge::new(0, 1, None), // 0: unmatched prop
            Edge::new(1, 2, None), // 1: matched prop
            Edge::new(2, 3, None), // 2: bridge (tenacity 5)
            Edge::new(3, 4, None), // 3: matched prop
            Edge::new(4, 0, None), // 4: unmatched prop
        ],
    );
    let mate = vec![None, Some(2), Some(1), Some(4), Some(3)];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();

    // Run level search to discover levels and bridge (2, 3)
    solver.run_level_search_phase(&graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::BridgeProcessing);

    // Run bridge processing phase: DDFS should discover petal with base 0
    let bottleneck = solver.run_bridge_processing_phase(&graph).unwrap();
    assert_eq!(solver.phase(), MicaliVaziraniPhase::Augmentation);
    // On C5 with matched edges, there are no exposed roots other than 0,
    // so no augmenting path exists (bottleneck is None)
    assert_eq!(bottleneck, None);

    // Verify petal base unification: all vertices on C5 now have base 0
    assert_eq!(solver.petal_base_of(0), 0);
    assert_eq!(solver.petal_base_of(1), 0);
    assert_eq!(solver.petal_base_of(2), 0);
    assert_eq!(solver.petal_base_of(3), 0);
    assert_eq!(solver.petal_base_of(4), 0);

    // Verify MAX levels assigned via petal formula: max_level(v) = tenacity - min_level(v)
    // tenacity = 5
    // min_level: 0 -> 0, 1 -> 1, 2 -> 2, 3 -> 2, 4 -> 1
    // max_level: 0 -> 5, 1 -> 4, 2 -> 3, 3 -> 3, 4 -> 4
    assert_eq!(solver.max_level()[2], Some(3)); // 5 - 2 = 3
    assert_eq!(solver.max_level()[3], Some(3)); // 5 - 2 = 3
    assert_eq!(solver.max_level()[1], Some(4)); // 5 - 1 = 4
    assert_eq!(solver.max_level()[4], Some(4)); // 5 - 1 = 4
    assert_eq!(solver.max_level()[0], Some(5)); // 5 - 0 = 5
}

#[test]
fn ddfs_identifies_bottleneck_augmenting_path() {
    // Path: 0 - 1 = 2 - 3 = 4 - 5
    // 0 and 5 exposed roots.
    // Matched: (1, 2) and (3, 4).
    // Bridge (2, 3) (edge 2) has tenacity 5.
    let graph = Graph::from_edges(
        6,
        vec![
            Edge::new(0, 1, None), // 0
            Edge::new(1, 2, None), // 1 (matched)
            Edge::new(2, 3, None), // 2 (bridge)
            Edge::new(3, 4, None), // 3 (matched)
            Edge::new(4, 5, None), // 4
        ],
    );
    let mate = vec![None, Some(2), Some(1), Some(4), Some(3), None];
    let mut solver = MicaliVazirani::with_matching(&graph, mate).unwrap();

    let bottleneck = solver.run_bridge_processing_phase(&graph).unwrap();
    assert_eq!(bottleneck, Some(5));
    assert_eq!(solver.bottleneck_tenacity(), Some(5));
    assert_eq!(solver.augmenting_bridges(), &[2]);
    assert_eq!(solver.phase(), MicaliVaziraniPhase::Augmentation);
}

#[test]
fn mv_solves_pentagon_with_tail() {
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
    let mut solver = MicaliVazirani::new(&graph);
    let result = solver.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 3);
    assert!(result.is_perfect);
}

#[test]
fn mv_solves_nested_blossoms() {
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
    let mut solver = MicaliVazirani::new(&graph);
    let result = solver.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 4);
    assert!(result.is_perfect);
}

#[test]
fn mv_matches_fast_blossom_on_complete_and_odd_graphs() {
    use blossom_study::FastBlossomMatching;

    // Test on K4
    let k4 = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, None),
            Edge::new(0, 2, None),
            Edge::new(0, 3, None),
            Edge::new(1, 2, None),
            Edge::new(1, 3, None),
            Edge::new(2, 3, None),
        ],
    );
    let mut mv_k4 = MicaliVazirani::new(&k4);
    let res_mv_k4 = mv_k4.max_cardinality_matching(&k4).unwrap();
    let res_fb_k4 = FastBlossomMatching::max_cardinality_matching(&k4);
    assert_valid_matching(&k4, &res_mv_k4);
    assert_valid_matching(&k4, &res_fb_k4);
    assert_eq!(res_mv_k4.edges.len(), res_fb_k4.edges.len());
    assert_eq!(res_mv_k4.edges.len(), 2);

    // Test on C7 (odd cycle of 7 vertices -> maximum matching size 3)
    let c7 = Graph::from_edges(
        7,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
            Edge::new(3, 4, None),
            Edge::new(4, 5, None),
            Edge::new(5, 6, None),
            Edge::new(6, 0, None),
        ],
    );
    let mut mv_c7 = MicaliVazirani::new(&c7);
    let res_mv_c7 = mv_c7.max_cardinality_matching(&c7).unwrap();
    let res_fb_c7 = FastBlossomMatching::max_cardinality_matching(&c7);
    assert_valid_matching(&c7, &res_mv_c7);
    assert_valid_matching(&c7, &res_fb_c7);
    assert_eq!(res_mv_c7.edges.len(), res_fb_c7.edges.len());
    assert_eq!(res_mv_c7.edges.len(), 3);
}

#[test]
fn mv_agrees_with_fast_blossom_on_varied_graphs() {
    use blossom_study::FastBlossomMatching;

    // Deterministic pseudo-random LCG for reproducible graphs
    let mut rng_state = 123456789_u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_state >> 33) as u32
    };

    for n in [8, 10, 12, 14] {
        for seed in 0..10 {
            let mut edges = Vec::new();
            for u in 0..n {
                for v in (u + 1)..n {
                    // Density ~ 35%
                    if (next_rand() % 100) < 35 {
                        edges.push(Edge::new(u, v, None));
                    }
                }
            }
            let graph = Graph::from_edges(n, edges);
            let expected_cardinality = FastBlossomMatching::max_cardinality_matching(&graph)
                .edges
                .len();

            let mut mv = MicaliVazirani::new(&graph);
            let mv_result = mv.max_cardinality_matching(&graph).unwrap();
            assert_valid_matching(&graph, &mv_result);
            assert_eq!(
                mv_result.edges.len(),
                expected_cardinality,
                "Failed on n={n}, seed={seed}"
            );
        }
    }
}

/// The greedy first phase may choose 1--3. The next phase must recognize the
/// matched middle edge as a bridge of tenacity three and augment on
/// 0--3==1--2, obtaining the perfect matching {0--3, 1--2}.
#[test]
fn matched_bridge_on_four_vertex_path_is_not_a_false_optimum() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 3, None),
            Edge::new(1, 2, None),
            Edge::new(1, 3, None),
        ],
    );

    let initial = vec![None, Some(3), None, Some(1)];
    let mut from_initial = MicaliVazirani::with_matching(&graph, initial).unwrap();
    assert_eq!(
        from_initial.run_interleaved_search_phase(&graph).unwrap(),
        Some(3)
    );
    assert_eq!(from_initial.bridge_tenacity(&graph, 2), Some(3));
    // DDFS must not accept a forged tenacity as a certificate.
    assert_eq!(from_initial.ddfs(&graph, 2, 5), DdfsResult::Redundant);
    let mut from_initial =
        MicaliVazirani::with_matching(&graph, vec![None, Some(3), None, Some(1)]).unwrap();
    assert_eq!(
        from_initial
            .max_cardinality_matching(&graph)
            .unwrap()
            .edges
            .len(),
        2
    );

    let mut from_empty = MicaliVazirani::new(&graph);
    let result = from_empty.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 2);
    assert!(result.is_perfect);
}

/// A first petal must activate its MAX-level vertices; the resulting second
/// petal must be lifted into actual graph edges before the length-seven path
/// can be used. This once terminated with only three matched pairs.
#[test]
fn nested_petals_are_unrolled_to_an_actual_augmenting_path() {
    let pairs = [
        (0, 7),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 7),
        (2, 3),
        (2, 4),
        (2, 5),
        (3, 4),
        (3, 5),
        (3, 7),
        (4, 5),
        (4, 6),
        (6, 7),
    ];
    let graph = Graph::from_edges(
        8,
        pairs.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
    );
    let initial = vec![
        None,
        None,
        Some(3),
        Some(2),
        Some(5),
        Some(4),
        Some(7),
        Some(6),
    ];
    let mut solver = MicaliVazirani::with_matching(&graph, initial).unwrap();
    let result = solver.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 4);
    assert!(result.is_perfect);
}

/// A 10-vertex graph with nested blossoms requiring a length-nine augmenting path.
/// The solver successfully resolves the petal hierarchy and finds the maximum matching.
#[test]
fn nested_petal_length_nine_augmenting_path_succeeds() {
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
    let mut solver = MicaliVazirani::with_matching(&graph, initial).unwrap();
    let result = solver.max_cardinality_matching(&graph).unwrap();
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 5);
    assert!(result.is_perfect);
}

/// Regression graph for nested-petal discovery. The current implementation
/// resolves this instance and returns a perfect matching.
#[test]
fn mv_solves_second_ten_vertex_nested_petal_regression() {
    let pairs = [
        (0, 1),
        (0, 5),
        (0, 7),
        (1, 2),
        (1, 8),
        (2, 3),
        (2, 6),
        (2, 8),
        (2, 9),
        (4, 6),
        (4, 9),
        (5, 7),
        (5, 9),
        (6, 7),
        (6, 9),
    ];
    let graph = Graph::from_edges(
        10,
        pairs.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
    );
    let mut solver = MicaliVazirani::new(&graph);
    let result = solver.max_cardinality_matching(&graph);
    let result = result.expect("nested-petal regression should be solved");
    assert_valid_matching(&graph, &result);
    assert_eq!(result.edges.len(), 5);
    assert!(result.is_perfect);
}

/// Exact, deliberately exponential oracle for the tiny exhaustive suite.
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

/// Covers every simple graph through six vertices, including disconnected,
/// bipartite, odd-cycle, and blossom-containing cases. In addition to size,
/// check mate symmetry and that every selected pair is an actual graph edge.
#[test]
fn mv_agrees_with_exhaustive_small_graph_oracle() {
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
            let mut solver = MicaliVazirani::new(&graph);
            let result = solver.max_cardinality_matching(&graph).unwrap();
            assert_valid_matching(&graph, &result);
            assert_eq!(
                result.edges.len(),
                expected,
                "n={n}, mask={mask}, edges={chosen:?}"
            );
            assert_eq!(result.is_perfect, expected * 2 == n);
            for (u, mate) in solver.mate().iter().enumerate() {
                if let Some(v) = mate {
                    assert_ne!(u, *v);
                    assert_eq!(solver.mate()[*v], Some(u));
                    assert!(chosen.contains(&(u.min(*v), u.max(*v))));
                }
            }
        }
    }
}
