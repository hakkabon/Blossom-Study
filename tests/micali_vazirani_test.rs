use blossom_study::{Edge, Graph, MicaliVazirani, MicaliVaziraniError, MicaliVaziraniPhase};

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

    let error = solver.max_cardinality_matching(&graph).unwrap_err();
    assert_eq!(
        error,
        MicaliVaziraniError::NotImplemented {
            phase: MicaliVaziraniPhase::Augmentation,
        }
    );
    assert_eq!(solver.phase(), MicaliVaziraniPhase::Augmentation);
    assert!(solver.mate().iter().all(Option::is_none));
    assert_eq!(solver.even_level(), &[Some(0); 4]);
    assert_eq!(solver.odd_level(), &[Some(1); 4]);
    assert_eq!(solver.min_level(), &[Some(0); 4]);
    assert_eq!(solver.max_level(), &[Some(1); 4]);
    assert_eq!(solver.bridge_candidates(), &[0, 1, 2, 3]);
    assert!(solver.level_search_scans() > 0);
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
    assert_eq!(solver.even_level(), &[Some(0), None, Some(2), None, Some(4)]);
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
