use blossom_study::{
    benchmark::{GENERAL_CARDINALITY_SOLVERS, compare_general_cardinality},
    graph::{Edge, Graph},
};

#[test]
fn general_cardinality_registry_compares_named_solvers_on_the_same_graph() {
    let graph = Graph::from_edges(
        6,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 0, None),
            Edge::new(3, 4, None),
            Edge::new(4, 5, None),
            Edge::new(5, 3, None),
            Edge::new(0, 3, None),
        ],
    );

    let results = compare_general_cardinality(&graph).unwrap();
    assert_eq!(results.len(), GENERAL_CARDINALITY_SOLVERS.len());
    assert_eq!(
        results
            .iter()
            .map(|entry| entry.solver.name)
            .collect::<Vec<_>>(),
        ["fast-blossom", "gabow-heuristic"]
    );
    assert_eq!(results[0].result.edges.len(), 3);
    for entry in results {
        entry.result.validate(&graph).unwrap();
        assert_eq!(entry.result.edges.len(), 3);
    }
}

#[test]
fn general_cardinality_registry_handles_empty_and_nonperfect_graphs() {
    for (graph, is_perfect) in [
        (Graph::from_edges(0, vec![]), true),
        (Graph::from_edges(3, vec![Edge::new(0, 1, None)]), false),
    ] {
        let results = compare_general_cardinality(&graph).unwrap();
        assert_eq!(results.len(), GENERAL_CARDINALITY_SOLVERS.len());
        assert_eq!(results[0].result.edges.len(), results[1].result.edges.len());
        assert!(
            results
                .iter()
                .all(|entry| entry.result.is_perfect == is_perfect)
        );
    }
}
