use blossom_study::{Edge, Graph, MicaliVazirani, MicaliVaziraniError, MicaliVaziraniPhase};

#[test]
fn scaffold_exposes_phase_state_without_mislabeling_a_fallback() {
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
            phase: MicaliVaziraniPhase::LevelSearch,
        }
    );
    assert_eq!(solver.phase(), MicaliVaziraniPhase::LevelSearch);
    assert!(solver.mate().iter().all(Option::is_none));
}
