//! Focused tests demonstrating specific defects in Gabow's trace_to_root.

use blossom_study::{Edge, GabowCardinality, Graph};

/// Defect 1: trace_to_root cannot trace non-root outer vertices.
///
/// Graph: 0 - 1 = 2 - 3 (path of 4 vertices, {1,2} matched, 0 and 3 exposed)
/// Augmenting path: 0 - 1 = 2 - 3 (length 3)
///
/// Phase 1 finds the bridge (2,3) with path length 3.
/// Phase 2 FAILS because trace_to_root(2) returns [2] (p[2] = MAX,
/// since p is only set for inner vertices, not outer vertices).
/// The "root" is 2, which is matched (mate[2]=1), so the bridge is skipped.
#[test]
fn gabow_phase2_fails_on_length3_path() {
    let graph = Graph::from_edges(
        4,
        vec![
            Edge::new(0, 1, None),
            Edge::new(1, 2, None),
            Edge::new(2, 3, None),
        ],
    );
    let mate = vec![None, Some(2), Some(1), None];
    let mut solver = GabowCardinality::with_matching(&graph, mate).unwrap();

    let (shortest_len, bridges) = solver.phase1_search(&graph);
    assert_eq!(shortest_len, Some(3), "Phase 1 finds length-3 path");
    assert!(!bridges.is_empty(), "Phase 1 finds candidate bridge");

    let aug_count = solver.phase2_augment(&graph, 3, &bridges);
    assert_eq!(
        aug_count, 0,
        "Phase 2 fails: trace_to_root(2) = [2] because p[2]=MAX"
    );
}

/// Phase 2 works for length-1 paths (bridge between two exposed roots).
#[test]
fn gabow_phase2_works_for_length1() {
    let graph = Graph::from_edges(2, vec![Edge::new(0, 1, None)]);
    let mut solver = GabowCardinality::new(&graph);
    let (len, bridges) = solver.phase1_search(&graph);
    assert_eq!(len, Some(1));
    let aug = solver.phase2_augment(&graph, 1, &bridges);
    assert_eq!(aug, 1, "Phase 2 works for length-1 (both endpoints are roots)");
}

/// Defect 2: trace_to_root produces invalid alternating paths through blossoms.
///
/// Triangle 0-1-2-0 plus edge 0-3, matching {0,1}, exposed {2,3}.
/// Augmenting path: 3-0=1-2 (length 3: unmatched, matched, unmatched).
///
/// After blossom on {0,1,2} with base 2:
/// p[0] = 2 (set by mark_blossom to bridge endpoint)
/// trace_to_root(0) = [0, 2] using direct unmatched edge (0,2)
/// Full path [2, 0, 3] has (2,0) unmatched, (0,3) unmatched — NOT alternating.
#[test]
fn gabow_trace_to_root_skips_blossom_interior() {
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
    let mut solver = GabowCardinality::with_matching(&graph, mate).unwrap();

    let (shortest_len, bridges) = solver.phase1_search(&graph);

    if let Some(target_len) = shortest_len {
        let aug_count = solver.phase2_augment(&graph, target_len, &bridges);
        // Phase 2 fails because the traced path through the blossom
        // has two consecutive unmatched edges (non-alternating).
        assert_eq!(
            aug_count, 0,
            "Phase 2 fails: trace_to_root skips blossom interior"
        );
    }
}
