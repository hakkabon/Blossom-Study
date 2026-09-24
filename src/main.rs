//! Blossom Algorithm Study - CLI and REPL for matching algorithms

use clap::{Parser, Subcommand, ValueEnum};
use anyhow::Result;
use rustyline::{DefaultEditor, Result as RustylineResult};

mod graph;
mod algorithms;

#[derive(Clone, Debug, ValueEnum)]
enum Algorithm {
    /// Original Edmonds implementation
    Edmonds,
    /// Korte-Vygen implementation (Union-Find based)
    KorteVygen,
    /// Fast Blossom (DSU with timestamp LCA)
    FastBlossom,
}

#[derive(Parser)]
#[command(name = "blossom_study")]
#[command(about = "Benchmark and test Edmonds' Blossom Algorithm implementations")]
struct Cli {
    #[arg(short, long, value_enum, default_value = "edmonds")]
    algorithm: Algorithm,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run maximum cardinality matching on a graph
    MaxCardinality {
        /// Input graph file (JSON or DIMACS format)
        #[arg(short, long)]
        input: String,
        /// Output results to file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run minimum weight perfect matching on a graph
    MinWeightPerfect {
        /// Input graph file (JSON or DIMACS format)
        #[arg(short, long)]
        input: String,
        /// Output results to file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate random test graphs
    Generate {
        /// Number of vertices
        #[arg(short, long, default_value = "20")]
        n: usize,
        /// Number of edges (for sparse) or density (for dense)
        #[arg(short, long, default_value = "1.0")]
        m: f64,
        /// Output file
        #[arg(short, long)]
        output: String,
        /// Graph type: complete, sparse, grid
        #[arg(short, long, default_value = "complete")]
        type_: String,
    },
    /// Run benchmarks
    Bench {
        /// Config file
        #[arg(short, long)]
        config: Option<String>,
        /// Output directory for results
        #[arg(short, long, default_value = "bench_results")]
        output: String,
    },
    /// Interactive REPL for matching algorithms
    Repl {
        /// Initial graph file to load (JSON or DIMACS)
        #[arg(short, long)]
        load: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::MaxCardinality { input, output } => {
            let graph = load_graph(&input)?;
            let result = match cli.algorithm {
                Algorithm::Edmonds => {
                    let mut algo = algorithms::EdmondsAlgorithm::new(&graph);
                    algo.max_cardinality(&graph)
                }
                Algorithm::KorteVygen => {
                    algorithms::KorteVygenEdmonds::max_cardinality_matching(&graph)
                }
                Algorithm::FastBlossom => {
                    algorithms::FastBlossomMatching::max_cardinality_matching(&graph)
                }
            };
            print_result(&result, output)?;
        }
        Commands::MinWeightPerfect { input, output } => {
            let graph = load_graph(&input)?;
            let mut algo = algorithms::EdmondsAlgorithm::new(&graph);
            let result = algo.min_weight_perfect(&graph).map_err(anyhow::Error::msg)?;
            print_result(&result, output)?;
        }
        Commands::Generate { n, m, output, type_ } => {
            let graph = match type_.as_str() {
                "complete" => graph::generator::complete_graph(n, (1.0, 100.0)),
                "sparse" => graph::generator::random_sparse(n, m as usize, (1.0, 100.0)),
                "grid" => {
                    let rows = (n as f64).sqrt() as usize;
                    let cols = (n + rows - 1) / rows;
                    graph::generator::grid_graph(rows, cols, (1.0, 100.0))
                }
                _ => anyhow::bail!("Unknown graph type: {}", type_),
            };
            graph.to_json_file(&output)?;
            println!("Generated graph with {} vertices and {} edges", graph.n, graph.edges.len());
        }
        Commands::Bench { config, output } => {
            println!("Running benchmarks...");
            // TODO: Implement benchmark runner
            println!("Config: {:?}, Output: {}", config, output);
        }
        Commands::Repl { load } => {
            run_repl(load, cli.algorithm)?;
        }
    }
    Ok(())
}

fn load_graph(path: &str) -> Result<graph::Graph> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        "json" => graph::Graph::from_json_file(path).map_err(anyhow::Error::from),
        "dimacs" | "dat" | "txt" => graph::Graph::from_dimacs_file(path).map_err(anyhow::Error::from),
        _ => {
            // Try JSON first, then DIMACS
            graph::Graph::from_json_file(path)
                .map_err(anyhow::Error::from)
                .or_else(|_| graph::Graph::from_dimacs_file(path).map_err(anyhow::Error::from))
        }
    }
}

fn print_result(result: &algorithms::MatchingResult, output: Option<String>) -> Result<()> {
    // Compact output format
    println!("matching_size={}", result.edges.len());
    println!("total_weight={:.6}", result.total_weight);
    println!("is_perfect={}", result.is_perfect);
    println!("iterations={}", result.metadata.iterations);
    println!("blossoms_created={}", result.metadata.blossoms_created);
    println!("blossoms_expanded={}", result.metadata.blossoms_expanded);
    println!("dual_updates={}", result.metadata.dual_updates);
    println!("runtime_ms={:.3}", result.metadata.runtime_ms);
    
    if !result.edges.is_empty() {
        print!("edges=");
        for (i, &e) in result.edges.iter().enumerate() {
            if i > 0 { print!(" "); }
            print!("{}", e);
        }
        println!();
    }
    
    if let Some(out) = output {
        std::fs::write(out, serde_json::to_string_pretty(result)?)?;
    }
    Ok(())
}

fn run_repl(initial_load: Option<String>, initial_algorithm: Algorithm) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut graph: Option<graph::Graph> = None;
    let mut last_result: Option<algorithms::MatchingResult> = None;
    let mut current_algorithm = initial_algorithm;
    
    if let Some(path) = initial_load {
        match load_graph(&path) {
            Ok(g) => {
                println!("Loaded graph: {} vertices, {} edges", g.n, g.edges.len());
                graph = Some(g);
            }
            Err(e) => println!("Failed to load graph: {}", e),
        }
    }
    
    println!("Blossom Algorithm REPL");
    println!("Commands: load <file>, max, min, graph, edges, result, save <file>, algo <name>, quit");
    println!("Algorithms: edmonds, korte_vygen, fast");
    println!("File formats: JSON (.json), DIMACS (.dimacs, .dat, .txt)");
    println!();
    
    loop {
        let readline = rl.readline("blossom> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line)?;
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                
                match parts[0] {
                    "algo" | "algorithm" => {
                        if parts.len() < 2 {
                            println!("Usage: algo <edmonds|korte_vygen|korte-vygen|fast|fast_blossom>");
                            println!("Current: {:?}", current_algorithm);
                            continue;
                        }
                        current_algorithm = match parts[1].to_lowercase().as_str() {
                            "edmonds" => Algorithm::Edmonds,
                            "korte_vygen" | "korte-vygen" | "korte" | "kv" => Algorithm::KorteVygen,
                            "fast" | "fast_blossom" => Algorithm::FastBlossom,
                            _ => {
                                println!("Unknown algorithm: {}. Use 'edmonds', 'korte_vygen', or 'fast'", parts[1]);
                                continue;
                            }
                        };
                        println!("Algorithm set to: {:?}", current_algorithm);
                    }
                    "load" => {
                        if parts.len() < 2 {
                            println!("Usage: load <file>");
                            continue;
                        }
                        match load_graph(parts[1]) {
                            Ok(g) => {
                                println!("Loaded: {} vertices, {} edges", g.n, g.edges.len());
                                graph = Some(g);
                            }
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    "max" => {
                        if let Some(ref g) = graph {
                            let result = match current_algorithm {
                                Algorithm::Edmonds => {
                                    let mut algo = algorithms::EdmondsAlgorithm::new(g);
                                    algo.max_cardinality(g)
                                }
                                Algorithm::KorteVygen => {
                                    algorithms::KorteVygenEdmonds::max_cardinality_matching(g)
                                }
                                Algorithm::FastBlossom => {
                                    algorithms::FastBlossomMatching::max_cardinality_matching(g)
                                }
                            };
                            print_compact_result(&result);
                            last_result = Some(result);
                        } else {
                            println!("No graph loaded. Use 'load <file>' first.");
                        }
                    }
                    "min" => {
                        if let Some(ref g) = graph {
                            let mut algo = algorithms::EdmondsAlgorithm::new(g);
                            match algo.min_weight_perfect(g) {
                                Ok(result) => {
                                    print_compact_result(&result);
                                    last_result = Some(result);
                                }
                                Err(e) => println!("Error: {}", e),
                            }
                        } else {
                            println!("No graph loaded. Use 'load <file>' first.");
                        }
                    }
                    "graph" => {
                        if let Some(ref g) = graph {
                            println!("Graph: {} vertices, {} edges", g.n, g.edges.len());
                            if g.n <= 20 {
                                for e in &g.edges {
                                    if let Some(w) = e.weight {
                                        println!("  {} - {} (weight: {})", e.u, e.v, w);
                                    } else {
                                        println!("  {} - {}", e.u, e.v);
                                    }
                                }
                            }
                        } else {
                            println!("No graph loaded.");
                        }
                    }
                    "edges" => {
                        if let Some(ref g) = graph {
                            println!("{} edges:", g.edges.len());
                            for (i, e) in g.edges.iter().enumerate() {
                                if let Some(w) = e.weight {
                                    println!("  [{}] {} - {} (w={})", i, e.u, e.v, w);
                                } else {
                                    println!("  [{}] {} - {}", i, e.u, e.v);
                                }
                            }
                        } else {
                            println!("No graph loaded.");
                        }
                    }
                    "result" => {
                        if let Some(ref r) = last_result {
                            print_compact_result(r);
                        } else {
                            println!("No result available. Run 'max' or 'min' first.");
                        }
                    }
                    "save" => {
                        if parts.len() < 2 {
                            println!("Usage: save <file>");
                            continue;
                        }
                        if let Some(ref g) = graph {
                            let ext = std::path::Path::new(parts[1])
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            match ext.as_str() {
                                "json" => g.to_json_file(parts[1])?,
                                "dimacs" | "dat" | "txt" => g.to_dimacs_file(parts[1])?,
                                _ => g.to_json_file(parts[1])?,
                            }
                            println!("Saved to {}", parts[1]);
                        } else if let Some(ref r) = last_result {
                            std::fs::write(parts[1], serde_json::to_string_pretty(r)?)?;
                            println!("Saved result to {}", parts[1]);
                        } else {
                            println!("Nothing to save.");
                        }
                    }
                    "quit" | "exit" => {
                        println!("Goodbye!");
                        break;
                    }
                    "help" => {
                        println!("Commands:");
                        println!("  load <file>     - Load graph from JSON or DIMACS file");
                        println!("  max             - Run maximum cardinality matching");
                        println!("  min             - Run minimum weight perfect matching");
                        println!("  graph           - Show loaded graph info");
                        println!("  edges           - List all edges");
                        println!("  result          - Show last matching result");
                        println!("  save <file>     - Save graph (.json/.dimacs) or result (.json)");
                        println!("  quit / exit     - Exit REPL");
                        println!("  help            - Show this help");
                    }
                    _ => {
                        println!("Unknown command: {}. Type 'help' for commands.", parts[0]);
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("Interrupted. Use 'quit' to exit.");
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}

fn print_compact_result(result: &algorithms::MatchingResult) {
    println!("=== Matching Result ===");
    println!("  size:       {}", result.edges.len());
    println!("  weight:     {:.6}", result.total_weight);
    println!("  perfect:    {}", result.is_perfect);
    println!("  iterations: {}", result.metadata.iterations);
    println!("  blossoms:   created={}, expanded={}", 
             result.metadata.blossoms_created, result.metadata.blossoms_expanded);
    println!("  dual_updates: {}", result.metadata.dual_updates);
    println!("  time:       {:.3} ms", result.metadata.runtime_ms);
    if !result.edges.is_empty() {
        print!("  edges:      ");
        for (i, &e) in result.edges.iter().enumerate() {
            if i > 0 { print!(" "); }
            print!("{}", e);
        }
        println!();
    }
    println!();
}