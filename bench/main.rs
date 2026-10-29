//! Benchmark runner for matching algorithms

use blossom_study::{
    algorithms::{FastBlossomMatching, GabowCardinality, MicaliVazirani},
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
        "{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8} | {:>10}",
        "Graph", "Algo", "Vertices", "Edges", "Match Size", "Time(ms)", "Blossoms"
    );
    println!(
        "{:-<20}-|-{:-<6}-|-{:-<8}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<10}",
        "", "", "", "", "", "", ""
    );

    for (name, graph) in test_cases {
        let result_fast = FastBlossomMatching::max_cardinality_matching(&graph);
        let mut mv_solver = MicaliVazirani::new(&graph);
        let result_mv = mv_solver.max_cardinality_matching(&graph).unwrap();
        let result_gabow = GabowCardinality::compute_matching(&graph);

        println!(
            "{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
            name,
            "fast",
            graph.n,
            graph.edges.len(),
            result_fast.edges.len(),
            result_fast.metadata.runtime_ms,
            result_fast.metadata.blossoms_created
        );
        println!(
            "{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
            "",
            "mv",
            graph.n,
            graph.edges.len(),
            result_mv.edges.len(),
            result_mv.metadata.runtime_ms,
            result_mv.metadata.blossoms_created
        );
        println!(
            "{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
            "",
            "gabow",
            graph.n,
            graph.edges.len(),
            result_gabow.edges.len(),
            result_gabow.metadata.runtime_ms,
            result_gabow.metadata.blossoms_created
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

    println!("\nCARDINALITY MATCHING: MICALI-VAZIRANI VS GABOW (MEHLHORN ET AL. 2024)");
    println!("Comparing end-to-end phased maximum cardinality algorithms with O(m*sqrt(n)) complexity.");
    println!(
        "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8} | {:>8}",
        "Graph Family", "Vertices", "Edges", "Algo", "Match Size", "Time(ms)", "Phases"
    );
    println!(
        "{:-<20}-|-{:-<8}-|-{:-<8}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<8}",
        "", "", "", "", "", "", ""
    );

    // 1. Dense Complete Graphs
    for n in [20, 50, 100] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        let mut mv_solver = MicaliVazirani::new(&graph);
        let res_mv = mv_solver.max_cardinality_matching(&graph).unwrap();
        let res_gabow = GabowCardinality::compute_matching(&graph);

        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            format!("Complete K{n}"),
            graph.n,
            graph.edges.len(),
            "mv",
            res_mv.edges.len(),
            res_mv.metadata.runtime_ms,
            res_mv.metadata.iterations
        );
        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            "",
            "",
            "",
            "gabow",
            res_gabow.edges.len(),
            res_gabow.metadata.runtime_ms,
            res_gabow.metadata.iterations
        );
    }

    // 2. Grid Graphs
    for side in [10, 20, 30] {
        let n = side * side;
        let graph = generator::grid_graph(side, side, (1.0, 100.0));
        let mut mv_solver = MicaliVazirani::new(&graph);
        let res_mv = mv_solver.max_cardinality_matching(&graph).unwrap();
        let res_gabow = GabowCardinality::compute_matching(&graph);

        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            format!("Grid {side}x{side}"),
            n,
            graph.edges.len(),
            "mv",
            res_mv.edges.len(),
            res_mv.metadata.runtime_ms,
            res_mv.metadata.iterations
        );
        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            "",
            "",
            "",
            "gabow",
            res_gabow.edges.len(),
            res_gabow.metadata.runtime_ms,
            res_gabow.metadata.iterations
        );
    }

    // 3. Planted Matching Graphs
    for n in [20, 50, 100] {
        let mut edges = Vec::new();
        let matched_pairs = (n / 4) * 2;
        let mut mate = vec![None; n];
        for u in (0..matched_pairs).step_by(2) {
            edges.push(Edge::new(u, u + 1, Some(1.0)));
            mate[u] = Some(u + 1);
            mate[u + 1] = Some(u);
        }
        for u in 0..n {
            for offset in [2, 3, 5] {
                let v = (u + offset) % n;
                if u < v {
                    edges.push(Edge::new(u, v, Some(1.0)));
                }
            }
        }
        let graph = Graph::from_edges(n, edges);
        let mut mv_solver = MicaliVazirani::new(&graph);
        let res_mv = mv_solver.max_cardinality_matching(&graph).unwrap();
        let res_gabow = GabowCardinality::compute_matching(&graph);

        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            format!("Planted n={n}"),
            graph.n,
            graph.edges.len(),
            "mv",
            res_mv.edges.len(),
            res_mv.metadata.runtime_ms,
            res_mv.metadata.iterations
        );
        println!(
            "{:<20} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>8}",
            "",
            "",
            "",
            "gabow",
            res_gabow.edges.len(),
            res_gabow.metadata.runtime_ms,
            res_gabow.metadata.iterations
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
