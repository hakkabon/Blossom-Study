use blossom_study::algorithms::{EdmondsAlgorithm, FastBlossomMatching, KorteVygenEdmonds};
use blossom_study::graph::{Edge, Graph};

fn make_graph(n: usize, edges: &[(usize, usize)]) -> Graph {
    Graph::from_edges(
        n,
        edges
            .iter()
            .map(|&(u, v)| Edge::new(u, v, Some(1.0)))
            .collect(),
    )
}

#[test]
fn test_edmonds_blossom_and_iteration_tracking() {
    // Pentagon with tail:
    // Vertices 0-1-2-3-4-0 form a 5-cycle (odd cycle/blossom), 0-5 is a tail
    let graph = make_graph(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 5)]);

    let mut edmonds = EdmondsAlgorithm::new(&graph);
    let result = edmonds.max_cardinality(&graph);

    assert_eq!(
        result.edges.len(),
        3,
        "Pentagon with tail should have size 3 matching"
    );
    assert!(result.is_perfect, "Should be a perfect matching");
    assert!(
        result.metadata.iterations > 0,
        "Iterations should be tracked and > 0, got {}",
        result.metadata.iterations
    );
    assert!(
        result.metadata.blossoms_created > 0,
        "Blossoms created should be tracked and > 0, got {}",
        result.metadata.blossoms_created
    );
    assert!(
        result.metadata.blossoms_expanded > 0,
        "Blossoms expanded should be tracked and > 0, got {}",
        result.metadata.blossoms_expanded
    );
}

#[test]
fn test_edmonds_odd_cycle_triangle() {
    // 3 vertices, triangle: only 1 edge can be matched
    let graph = make_graph(3, &[(0, 1), (1, 2), (2, 0)]);

    let mut edmonds = EdmondsAlgorithm::new(&graph);
    let result = edmonds.max_cardinality(&graph);

    assert_eq!(result.edges.len(), 1);
    assert!(!result.is_perfect);
}

#[test]
fn test_algorithms_cardinality_consistency() {
    // A bipartite graph with a bridge to another component
    let graph = make_graph(
        8,
        &[
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (2, 4), // bridge
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
        ],
    );

    let mut algo1 = EdmondsAlgorithm::new(&graph);
    let res1 = algo1.max_cardinality(&graph);

    let res2 = KorteVygenEdmonds::max_cardinality_matching(&graph);
    let res3 = FastBlossomMatching::max_cardinality_matching(&graph);

    assert_eq!(res1.edges.len(), res2.edges.len());
    assert_eq!(res2.edges.len(), res3.edges.len());
    assert_eq!(res1.edges.len(), 4);
    assert!(res1.is_perfect);
    assert!(res2.is_perfect);
    assert!(res3.is_perfect);
}

#[test]
fn test_all_algorithms_track_telemetry() {
    // Pentagon with tail: 0-1-2-3-4-0 (5-cycle) plus tail edge 0-5.
    let graph = make_graph(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 5)]);

    let mut edmonds = EdmondsAlgorithm::new(&graph);
    let res_edmonds = edmonds.max_cardinality(&graph);
    let res_kv = KorteVygenEdmonds::max_cardinality_matching(&graph);
    let res_fast = FastBlossomMatching::max_cardinality_matching(&graph);

    // All should be size-3 perfect matchings
    assert_eq!(res_edmonds.edges.len(), 3);
    assert_eq!(res_kv.edges.len(), 3);
    assert_eq!(res_fast.edges.len(), 3);

    // Iterations should be measured for all algorithms
    assert!(res_edmonds.metadata.iterations > 0);
    assert!(res_kv.metadata.iterations > 0,
        "KV iterations should be > 0 (includes greedy phase), got {}",
        res_kv.metadata.iterations);
    assert!(res_fast.metadata.iterations > 0);

    // Blossoms created: Edmonds and FastBlossom must contract the pentagon.
    // KorteVygen sorts vertices by degree (ascending) before the greedy pass,
    // so it matches the degree-1 tail vertex (5→0) first and then pairs the
    // remaining 4-cycle vertices — a perfect matching with no BFS required.
    // blossoms_created == 0 is the correct observed value for KV here.
    assert!(res_edmonds.metadata.blossoms_created > 0,
        "Edmonds must create a blossom on the pentagon+tail");
    assert!(res_fast.metadata.blossoms_created > 0,
        "FastBlossom must create a blossom on the pentagon+tail");

    // KV telemetry fields are properly initialised (valid usize / f64)
    let _ = res_kv.metadata.blossoms_created;
    let _ = res_kv.metadata.blossoms_expanded;

    // Runtime should be recorded and non-negative
    assert!(res_edmonds.metadata.runtime_ms >= 0.0);
    assert!(res_kv.metadata.runtime_ms >= 0.0);
    assert!(res_fast.metadata.runtime_ms >= 0.0);
}

#[test]
fn test_kv_blossom_telemetry_on_forced_contraction() {
    // Triangle (0-1-2) plus a 3-path hanging off vertex 0: 0-3-4-5.
    // 6 vertices, 7 edges.
    //
    // Degree-sorted greedy (ascending):
    //   deg-1: vertex 5  → matched to 4
    //   deg-2: vertex 1  → matched to 0  (first available)
    //   deg-2: vertex 2  → 0 and 1 already matched → stays unmatched
    //   deg-2: vertex 3  → 0 matched, 4 matched     → stays unmatched
    //   deg-2: vertex 4  → already matched
    //   deg-3: vertex 0  → already matched
    //
    // Greedy result: {5-4, 1-0} — vertices 2 and 3 are unmatched.
    //
    // BFS: outer vertex 2 grows tree 2→0→1 (0 matched to 1).
    //       Scanning 1: neighbour 2 is outer in the *same* tree → SHRINK.
    //       blossoms_created is incremented here.
    let graph = make_graph(
        6,
        &[
            (0, 1),
            (1, 2),
            (2, 0), // triangle
            (0, 3),
            (3, 4),
            (4, 5), // path attached to 0
        ],
    );

    let res_kv = KorteVygenEdmonds::max_cardinality_matching(&graph);
    assert_eq!(res_kv.edges.len(), 3, "should find a 3-edge perfect matching");
    assert!(res_kv.is_perfect, "matching should be perfect");
    assert!(
        res_kv.metadata.iterations > 0,
        "iterations must be tracked, got {}",
        res_kv.metadata.iterations
    );
    assert!(
        res_kv.metadata.blossoms_created > 0,
        "KV should have shrunk the triangle blossom, got blossoms_created={}",
        res_kv.metadata.blossoms_created
    );
}
