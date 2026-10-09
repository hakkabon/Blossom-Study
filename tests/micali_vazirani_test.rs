use blossom_study::{Edge, Graph, MicaliVazirani, MicaliVaziraniError, MicaliVaziraniPhase};

#[test]
fn level_search_exposes_phase_state_without_mislabeling_a_fallback() {
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
            phase: MicaliVaziraniPhase::BridgeProcessing,
        }
    );
    assert_eq!(solver.phase(), MicaliVaziraniPhase::BridgeProcessing);
    assert!(solver.mate().iter().all(Option::is_none));
    assert_eq!(solver.even_level(), &[Some(0); 4]);
    assert_eq!(solver.odd_level(), &[Some(1); 4]);
    assert_eq!(solver.min_level(), &[Some(0); 4]);
    assert_eq!(solver.max_level(), &[Some(1); 4]);
    assert!((0..4).all(|vertex| solver.predecessor_edges(vertex).unwrap().is_empty()));
    assert_eq!(solver.bridge_candidates(), &[0, 1, 2, 3]);
    assert!(solver.level_search_scans() > 0);
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
