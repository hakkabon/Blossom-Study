//! Historical trace-to-root regressions for the Gabow-inspired heuristic.

use blossom_study::{Edge, GabowHeuristic, Graph};

/// Former defect 1: trace_to_root could not trace non-root outer vertices.
///
/// Graph: 0 - 1 = 2 - 3 (path of 4 vertices, {1,2} matched, 0 and 3 exposed)
/// Augmenting path: 0 - 1 = 2 - 3 (length 3)
///
/// Phase 1 finds the bridge (2,3) with path length 3.
/// Phase 2 must now trace the length-three alternating path.
#[test]
fn gabow_phase2_augments_length3_path() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
        ],
    );
    let mate = vec![None, Some(2), Some(1), None];
    let mut solver = GabowHeuristic::with_matching(&graph, mate).unwrap();

    let (shortest_len, bridges) = solver.phase1_search(&graph);
    assert_eq!(shortest_len, Some(3), "Phase 1 finds length-3 path");
    assert!(!bridges.is_empty(), "Phase 1 finds candidate bridge");

    let aug_count = solver.phase2_augment(&graph, 3, &bridges);
    assert_eq!(
        aug_count, 1,
        "Phase 2 should trace the non-root outer vertex"
    );
    assert_eq!(
        solver.mate().iter().filter(|mate| mate.is_some()).count(),
        4
    );
}

/// Phase 2 works for length-1 paths (bridge between two exposed roots).
#[test]
fn gabow_phase2_works_for_length1() {
    let graph = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    let mut solver = GabowHeuristic::new(&graph);
    let (len, bridges) = solver.phase1_search(&graph);
    assert_eq!(len, Some(1));
    let aug = solver.phase2_augment(&graph, 1, &bridges);
    assert_eq!(
        aug, 1,
        "Phase 2 works for length-1 (both endpoints are roots)"
    );
}

/// Former defect 2: trace_to_root produced invalid alternating paths through blossoms.
///
/// Triangle 0-1-2-0 plus edge 0-3, matching {0,1}, exposed {2,3}.
/// Augmenting path: 3-0=1-2 (length 3: unmatched, matched, unmatched).
///
/// Phase 2 must include the blossom interior when it augments.
#[test]
fn gabow_trace_to_root_lifts_blossom_interior() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 0, None),
            Edge::new(0, 3, None),
        ],
    );
    let mate = vec![Some(1), Some(0), None, None];
    let mut solver = GabowHeuristic::with_matching(&graph, mate).unwrap();

    let (shortest_len, bridges) = solver.phase1_search(&graph);

    if let Some(target_len) = shortest_len {
        let aug_count = solver.phase2_augment(&graph, target_len, &bridges);
        assert_eq!(
            aug_count, 1,
            "Phase 2 should lift a valid alternating path through the blossom"
        );
        assert_eq!(
            solver.mate().iter().filter(|mate| mate.is_some()).count(),
            4
        );
    }
}
