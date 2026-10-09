//! Benchmark runner for matching algorithms

use blossom_study::{
    algorithms::{FastBlossomMatching, MicaliVazirani},
    graph::{Edge, Graph, generator},
};

fn complete_bipartite_graph(partition_size: usize) -> Graph {
    let mut edges = Vec::with_capacity(partition_size * partition_size);
    for left in 0..partition_size {
        for right in 0..partition_size {
            let weight = ((left * 131 + right * 67 + (left ^ right) * 17) % 997) as f64 / 4.0;
            edges.push(Edge::new(left, partition_size + right, Some(weight)));
        }
    }
    Graph::from_edges(2 * partition_size, edges)
}

fn main() {
    println!("Blossom Algorithm Benchmark");
    println!("===========================\n");
    println!("GENERAL-GRAPH CARDINALITY AND MWPM");

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
                (
                    "sparse",
                    blossom_study::sparse_minimum_weight_perfect_matching(&graph),
                ),
            ];

            weighted_results.push((
                "cherry",
                blossom_study::cherry_tree_minimum_weight_perfect_matching(&graph),
            ));

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

    println!("\nMICALI-VAZIRANI DEVELOPMENT PHASE");
    println!("Only alternating level search is timed; this is not an end-to-end matcher.");
    for n in [20, 50, 100] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        let started = std::time::Instant::now();
        let mut solver = MicaliVazirani::new(&graph);
        solver
            .run_level_search_phase(&graph)
            .expect("level search should accept its construction graph");
        println!(
            "  n={n:>4}, scans={:>8}, time={:>8.3} ms",
            solver.level_search_scans(),
            started.elapsed().as_secs_f64() * 1000.0
        );
    }

    println!("\nBIPARTITE WEIGHTED ASSIGNMENT");
    println!("Each size is shown as left+right; every solver receives the same graph.");
    for partition_size in [3, 5, 10, 20] {
        let graph = complete_bipartite_graph(partition_size);
        let results = [
            (
                "hungarian",
                blossom_study::hungarian_minimum_weight_perfect_matching(&graph, partition_size),
            ),
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
            (
                "cherry",
                blossom_study::cherry_tree_minimum_weight_perfect_matching(&graph),
            ),
            (
                "sparse",
                blossom_study::sparse_minimum_weight_perfect_matching(&graph),
            ),
        ];
        for (algorithm, result) in results {
            match result {
                Ok(result) => println!(
                    "  {:>3}+{:<3} {:>12}: weight={:>10.3}, time={:>8.3} ms",
                    partition_size,
                    partition_size,
                    algorithm,
                    result.total_weight,
                    result.metadata.runtime_ms
                ),
                Err(error) => println!(
                    "  {:>3}+{:<3} {:>12}: {error}",
                    partition_size, partition_size, algorithm
                ),
            }
        }
    }

    println!("\nFast Blossom is the active maximum-cardinality baseline.");
    println!("Weighted rows compare the available O(V³) primal-dual representations.");
    println!("Cherry is the repaired self-contained ordered-blossom implementation.");
    println!("Hungarian is the blossom-free bipartite primal-dual baseline.");
}
