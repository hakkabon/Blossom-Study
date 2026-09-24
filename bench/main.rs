//! Benchmark runner for matching algorithms

use blossom_study::{graph::{Graph, generator}, algorithms::{EdmondsAlgorithm, KorteVygenEdmonds, FastBlossomMatching}};
use std::time::Instant;

fn main() {
    println!("Blossom Algorithm Benchmark");
    println!("===========================\n");
    
    // Test graphs
    let test_cases = vec![
        ("K6 (complete)", generator::complete_graph(6, (1.0, 100.0))),
        ("K10 (complete)", generator::complete_graph(10, (1.0, 100.0))),
        ("K20 (complete)", generator::complete_graph(20, (1.0, 100.0))),
        ("Sparse 20, 0.3", generator::random_sparse(20, 60, (1.0, 100.0))),
        ("Grid 5x5", generator::grid_graph(5, 5, (1.0, 100.0))),
    ];
    
    println!("{:<20} | {:>6} | {:>6} | {:>10} | {:>8} | {:>8} | {:>10}", 
             "Graph", "Algo", "Vertices", "Edges", "Match Size", "Time(ms)", "Blossoms");
    println!("{:-<20}-|-{:-<6}-|-{:-<8}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<10}",
             "", "", "", "", "", "", "");
    
    for (name, graph) in test_cases {
        // Test Edmonds algorithm
        let mut algo1 = EdmondsAlgorithm::new(&graph);
        let start = Instant::now();
        let result1 = algo1.max_cardinality(&graph);
        let elapsed1 = start.elapsed().as_secs_f64() * 1000.0;
        
        println!("{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
                 name,
                 "edmonds",
                 graph.n,
                 graph.edges.len(),
                 result1.edges.len(),
                 elapsed1,
                 result1.metadata.blossoms_created);
        
        // Test Korte-Vygen algorithm
        let mut graph2 = graph.clone();
        let result2 = KorteVygenEdmonds::max_cardinality_matching(&graph2);
        let elapsed2 = 0.0; // Time measured internally
        
        println!("{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
                 "",
                 "kv",
                 graph.n,
                 graph.edges.len(),
                 result2.edges.len(),
                 result2.metadata.runtime_ms,
                 result2.metadata.blossoms_created);
        
        // Test Fast Blossom algorithm
        let result3 = FastBlossomMatching::max_cardinality_matching(&graph);
        
        println!("{:<20} | {:>6} | {:>8} | {:>8} | {:>10} | {:>8.2} | {:>10}",
                 "",
                 "fast",
                 graph.n,
                 graph.edges.len(),
                 result3.edges.len(),
                 result3.metadata.runtime_ms,
                 result3.metadata.blossoms_created);
    }
    
    println!("\nNote: All implementations find maximum cardinality matching.");
    println!("Edmonds: Original implementation with explicit blossom tracking");
    println!("Korte-Vygen: Union-Find based implementation from Korte & Vygen");
    println!("Fast Blossom: DSU with timestamp-based LCA and path compression");
}