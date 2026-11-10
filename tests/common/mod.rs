use blossom_study::{MatchingResult, graph::Graph};

pub fn assert_valid_matching(graph: &Graph, result: &MatchingResult) {
    result
        .validate(graph)
        .unwrap_or_else(|error| panic!("invalid matching result: {error}"));
}

#[allow(dead_code)]
pub fn assert_perfect_matching(graph: &Graph, result: &MatchingResult) {
    assert_valid_matching(graph, result);
    assert!(result.is_perfect, "matching result should be perfect");
}
