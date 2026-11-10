//! Benchmark runner for completed matching algorithms.

use blossom_study::{
    FastBlossomMatching,
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
    println!("Experimental phased cardinality tracks are excluded.");

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
        "{:<20} | {:>8} | {:>8} | {:>10} | {:>8}",
        "Graph", "Vertices", "Edges", "Cardinality", "Time(ms)"
    );
    for (name, graph) in &test_cases {
        let result = FastBlossomMatching::max_cardinality_matching(graph);
        println!(
            "{:<20} | {:>8} | {:>8} | {:>10} | {:>8.2}",
            name,
            graph.n,
            graph.edges.len(),
            result.edges.len(),
            result.metadata.runtime_ms
        );
    }

    println!("\nGENERAL-GRAPH MINIMUM-WEIGHT PERFECT MATCHING");
    for (name, graph) in test_cases
        .iter()
        .filter(|(_, graph)| graph.n.is_multiple_of(2))
    {
        let results = [
            (
                "primal-dual",
                blossom_study::minimum_weight_perfect_matching(graph),
            ),
            (
                "blossom-engine",
                blossom_study::blossom_engine_minimum_weight_perfect_matching(graph),
            ),
            (
                "blossom-tree",
                blossom_study::blossom_tree_minimum_weight_perfect_matching(graph),
            ),
            (
                "cherry-tree",
                blossom_study::cherry_tree_minimum_weight_perfect_matching(graph),
            ),
            (
                "sparse-blossom",
                blossom_study::sparse_minimum_weight_perfect_matching(graph),
            ),
        ];
        for (algorithm, result) in results {
            match result {
                Ok(result) => println!(
                    "  {:<20} {:>12}: weight={:>10.3}, time={:>8.3} ms",
                    name, algorithm, result.total_weight, result.metadata.runtime_ms
                ),
                Err(error) => println!("  {:<20} {:>12}: {error}", name, algorithm),
            }
        }
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
                "primal-dual",
                blossom_study::minimum_weight_perfect_matching(&graph),
            ),
            (
                "blossom-engine",
                blossom_study::blossom_engine_minimum_weight_perfect_matching(&graph),
            ),
            (
                "blossom-tree",
                blossom_study::blossom_tree_minimum_weight_perfect_matching(&graph),
            ),
            (
                "cherry-tree",
                blossom_study::cherry_tree_minimum_weight_perfect_matching(&graph),
            ),
            (
                "sparse-blossom",
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

    println!("\nFast Blossom is the general-graph maximum-cardinality baseline.");
    println!("MV development timing is available only in the isolated Criterion phase group.");
}
