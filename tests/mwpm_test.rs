use blossom_study::algorithms::minimum_weight_perfect_matching;
use blossom_study::algorithms::mwpm::{WeightScaler, filter_edges};
use blossom_study::graph::{Edge, Graph};

fn make_graph(n: usize, edges: &[(usize, usize, f64)]) -> Graph {
    Graph::from_edges(
        n,
        edges
            .iter()
            .map(|&(u, v, w)| Edge::new(u, v, Some(w)))
            .collect(),
    )
}

#[test]
fn test_weight_scaler_exactness() {
    let weights = vec![0.0, 1.5, -2.25, 0.125, 1000.0];
    let scaler = WeightScaler::new(&weights);

    for &w in &weights {
        let int_val = scaler.to_integer(w).expect("scaling should succeed");
        // Verify relative proportion matches exactly
        if w != 0.0 {
            let int_one = scaler.to_integer(1.0).expect("1.0 scale");
            let ratio = (int_val as f64) / (int_one as f64);
            assert!((ratio - w).abs() < 1e-12);
        } else {
            assert_eq!(int_val, 0);
        }
    }
}

#[test]
fn test_filter_edges_removes_self_loops_and_parallel() {
    let graph = make_graph(
        4,
        &[
            (0, 0, 1.0),  // self-loop -> dropped
            (0, 1, 10.0), // parallel edge 1
            (0, 1, 2.0),  // parallel edge 2 (cheaper -> kept)
            (1, 0, 5.0),  // reverse parallel -> dropped (more expensive)
            (2, 3, 3.0),
        ],
    );

    let filtered = filter_edges(&graph).expect("filter edges");
    assert_eq!(filtered.len(), 2);
    let edge_01 = filtered
        .iter()
        .find(|e| (e.u == 0 && e.v == 1) || (e.u == 1 && e.v == 0))
        .expect("edge between 0 and 1");
    assert_eq!(edge_01.weight, 2.0);
    assert_eq!(edge_01.original_index, 2);
}

#[test]
fn test_mwpm_diagnostics_on_isolated_vertex() {
    // 4 vertices, but vertex 3 has no edges
    let graph = make_graph(4, &[(0, 1, 1.0), (1, 2, 2.0)]);
    let err = minimum_weight_perfect_matching(&graph).unwrap_err();
    assert!(
        err.contains("isolated") || err.contains("degree 0"),
        "Expected error message about isolated vertex, got: {}",
        err
    );
}

#[test]
fn test_mwpm_diagnostics_on_odd_vertices() {
    let graph = make_graph(5, &[(0, 1, 1.0), (2, 3, 1.0)]);
    let err = minimum_weight_perfect_matching(&graph).unwrap_err();
    assert!(
        err.contains("even number of vertices"),
        "Expected error about even vertices, got: {}",
        err
    );
}

#[test]
fn test_mwpm_diagnostics_on_non_finite_weight() {
    let graph = make_graph(4, &[(0, 1, f64::NAN), (2, 3, 1.0)]);
    let err = minimum_weight_perfect_matching(&graph).unwrap_err();
    assert!(
        err.contains("non-finite"),
        "Expected error about non-finite weight, got: {}",
        err
    );
}

#[test]
fn test_weighted_solver_public_entry_point() {
    let graph = make_graph(
        4,
        &[
            (0, 1, 3.0),
            (1, 2, 1.0),
            (2, 3, 4.0),
            (3, 0, 2.0),
            (0, 2, 10.0),
            (1, 3, 10.0),
        ],
    );

    let result = minimum_weight_perfect_matching(&graph).expect("MWPM should succeed");
    assert_eq!(result.edges.len(), 2);
    assert!(result.is_perfect);
    assert_eq!(result.total_weight, 3.0);
}
