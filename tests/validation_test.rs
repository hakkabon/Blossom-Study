use blossom_study::{
    MatchingMetadata, MatchingResult,
    algorithms::SparseEdge,
    graph::{Edge, Graph},
};

fn result(edges: Vec<usize>, total_weight: f64, is_perfect: bool) -> MatchingResult {
    MatchingResult {
        edges,
        total_weight,
        is_perfect,
        metadata: MatchingMetadata::default(),
    }
}

#[test]
fn fallible_graph_constructors_reject_out_of_range_endpoints() {
    let edges = vec![Edge::new(0, 3, None)];
    let error = Graph::try_from_edges(3, edges).unwrap_err();
    assert!(error.contains("edge 0"));
    assert!(error.contains("outside 0..3"));

    let edges = vec![Edge::new(2, 0, None)];
    assert!(Graph::try_from_edges_sparse(2, edges).is_err());
    assert!(Graph::try_from_edges(usize::MAX, vec![]).is_err());
}

#[test]
fn json_deserialization_rejects_out_of_range_endpoints() {
    let error = serde_json::from_str::<Graph>(r#"{"n":2,"edges":[{"u":0,"v":2,"weight":null}]}"#)
        .unwrap_err();
    assert!(error.to_string().contains("outside 0..2"));
}

#[test]
fn dimacs_parser_rejects_zero_and_out_of_range_vertices() {
    for input in ["p edge 2 1\ne 0 2\n", "p edge 2 1\ne 1 3\n"] {
        assert!(Graph::from_dimacs_str(input).is_err());
    }
    let error = Graph::from_dimacs_str("c sample\np edge 2 1\ne 1 3\n")
        .unwrap_err()
        .to_string();
    assert!(error.contains("line 3"));
    assert!(error.contains("outside 1..=2"));
}

#[test]
fn dimacs_parser_reports_line_numbers_for_malformed_records() {
    let cases = [
        ("c sample\np edge 2 1\ne 1\n", "line 3"),
        ("p edge 2 x\n", "line 1"),
        ("p edge 2 1\ne 1 2 NaN\n", "line 2"),
        ("p edge 2 1\ne 1 2 3 extra\n", "line 2"),
        ("p edge 2 1\nx 1 2\n", "line 2"),
        ("p edge 2 0\np edge 2 0\n", "line 2"),
    ];
    for (input, expected_line) in cases {
        let error = Graph::from_dimacs_str(input).unwrap_err();
        assert!(
            error.to_string().contains(expected_line),
            "expected {expected_line} in {error}"
        );
    }
}

#[test]
fn dimacs_parser_checks_declared_counts_and_accepts_headerless_edge_lists() {
    let error = Graph::from_dimacs_str("c sample\np edge 3 2\ne 1 2\n")
        .unwrap_err()
        .to_string();
    assert!(error.contains("line 2"));
    assert!(error.contains("declares 2 edges, but found 1"));

    let graph = Graph::from_dimacs_str("e 1 4 2.5\n").unwrap();
    assert_eq!(graph.n, 4);
    assert_eq!(graph.edges, vec![Edge::new(0, 3, Some(2.5))]);
}

#[test]
fn dimacs_parser_accepts_the_repository_fixture() {
    let graph = Graph::from_dimacs_str(include_str!("data/test_dimacs.dat")).unwrap();
    assert_eq!(graph.n, 6);
    assert_eq!(graph.edges.len(), 9);
}

#[test]
fn sparse_edge_constructor_rejects_endpoints_that_do_not_fit_u32() {
    assert_eq!(SparseEdge::new(1, 2, 0, 3).unwrap().u, 1);
    assert!(SparseEdge::new(0, 2, 0, 3).is_err());
    if let Ok(out_of_range) = usize::try_from(u64::from(u32::MAX) + 1) {
        assert!(SparseEdge::new(out_of_range, 2, 0, 3).is_err());
    }
}

#[test]
fn matching_result_validator_accepts_valid_partial_and_perfect_matchings() {
    let graph = Graph::from_edges(4, vec![Edge::new(0, 1, Some(-2.5)), Edge::new(2, 3, None)]);
    assert!(result(vec![0], -2.5, false).validate(&graph).is_ok());
    assert!(result(vec![0, 1], -1.5, true).validate(&graph).is_ok());
    assert!(
        result(vec![], 0.0, false)
            .validate(&Graph::from_edges(0, vec![]))
            .is_err()
    );
}

#[test]
fn matching_result_validator_rejects_invalid_edges_and_duplicate_vertices() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, Some(1.0)),
            Edge::new(1, 2, Some(2.0)),
            Edge::new(2, 3, Some(3.0)),
        ],
    );
    assert!(result(vec![3], 0.0, false).validate(&graph).is_err());
    assert!(result(vec![0, 1], 3.0, false).validate(&graph).is_err());
    assert!(result(vec![0], 1.0, true).validate(&graph).is_err());
    assert!(result(vec![0], 2.0, false).validate(&graph).is_err());
}

#[test]
fn matching_result_validator_rejects_self_loops_and_mutated_graphs() {
    let self_loop = Graph::from_edges(2, vec![Edge::new(0, 0, Some(1.0))]);
    assert!(result(vec![0], 1.0, false).validate(&self_loop).is_err());

    let mut invalid = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    invalid.edges[0].v = 2;
    assert!(result(vec![], 0.0, false).validate(&invalid).is_err());
}
