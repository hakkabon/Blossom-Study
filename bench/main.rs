//! Benchmark runner for matching algorithms

use blossom_study::{algorithms::FastBlossomMatching, graph::generator};

fn main() {
    println!("Blossom Algorithm Benchmark");
    println!("===========================\n");

    // Test graphs
    let test_cases = vec![
        ("K6 (complete)", generator::complete_graph(6, (1.0, 100.0))),
        (
            "K10 (complete)",
            generator::complete_graph(10, (1.0, 100.0)),
        ),
        (
            "K20 (complete)",
            generator::complete_graph(20, (1.0, 100.0)),
        ),
        (
            "Sparse 20, 0.3",
            generator::random_sparse(20, 60, (1.0, 100.0)),
        ),
        ("Grid 5x5", generator::grid_graph(5, 5, (1.0, 100.0))),
    ];

    println!(
        "{:<20} | {:>6} | {:>6} | {:>10} | {:>8} | {:>8} | {:>10}",
        "Graph", "Algo", "Vertices", "Edges", "Match Size", "Time(ms)", "Blossoms"
    );
    println!(
        "{:-<20}-|-{:-<6}-|-{:-<8}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<10}",
        "", "", "", "", "", "", ""
    );

    for (name, graph) in test_cases {
        let result3 = FastBlossomMatching::max_cardinality_matching(&graph);

        println!(
            "{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
            name,
            "fast",
            graph.n,
            graph.edges.len(),
            result3.edges.len(),
            result3.metadata.runtime_ms,
            result3.metadata.blossoms_created
        );

        if graph.n % 2 == 0 {
            let mut weighted_results = vec![
                (
                    "rust-mwpm",
                    blossom_study::minimum_weight_perfect_matching(&graph),
                ),
                (
                    "engine",
                    blossom_study::blossom_engine_minimum_weight_perfect_matching(&graph),
                ),
                (
                    "tree",
                    blossom_study::blossom_tree_minimum_weight_perfect_matching(&graph),
                ),
            ];

            // Cherry-Tree deliberately preserves its complete distance-matrix
            // interface, so benchmarking it on a sparse or grid graph would
            // measure input rejection rather than matching performance.
            let complete_edge_count = graph.n.saturating_mul(graph.n.saturating_sub(1)) / 2;
            if graph.edges.len() == complete_edge_count {
                weighted_results.push((
                    "cherry",
                    blossom_study::cherry_tree_minimum_weight_perfect_matching(&graph),
                ));
            }

            for (algorithm, result) in weighted_results {
                match result {
                    Ok(result) => println!(
                        "{:<20} | {:>10} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
                        "",
                        algorithm,
                        graph.n,
                        graph.edges.len(),
                        result.edges.len(),
                        result.metadata.runtime_ms,
                        result.metadata.blossoms_created
                    ),
                    Err(error) => println!("{:<20} | {:>10} | {error}", "", algorithm),
                }
            }
        }
    }

    println!("\nFast Blossom is the active maximum-cardinality baseline.");
    println!("Weighted rows compare the available O(V³) primal-dual representations.");
    println!("Cherry includes dense-matrix conversion and shares the engine search kernel.");
}
