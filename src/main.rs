//! Blossom Algorithm Study - CLI and REPL for matching algorithms

use anyhow::{Context, Result};
use blossom_study::{algorithms, graph};
use clap::{Parser, Subcommand, ValueEnum};
use rustyline::DefaultEditor;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Algorithm {
    /// Original Edmonds implementation
    Edmonds,
    /// Korte-Vygen implementation (Union-Find based)
    KorteVygen,
    /// Fast Blossom (DSU with timestamp LCA)
    FastBlossom,
}

impl Algorithm {
    pub fn max_cardinality(&self, graph: &graph::Graph) -> algorithms::MatchingResult {
        match self {
            Algorithm::Edmonds => {
                let mut algo = algorithms::EdmondsAlgorithm::new(graph);
                algo.max_cardinality(graph)
            }
            Algorithm::KorteVygen => algorithms::KorteVygenEdmonds::max_cardinality_matching(graph),
            Algorithm::FastBlossom => {
                algorithms::FastBlossomMatching::max_cardinality_matching(graph)
            }
        }
    }

    pub fn min_weight_perfect(
        &self,
        graph: &graph::Graph,
    ) -> Result<algorithms::MatchingResult, String> {
        match self {
            Algorithm::Edmonds => {
                algorithms::EdmondsAlgorithm::new(graph).min_weight_perfect(graph)
            }
            Algorithm::KorteVygen => {
                algorithms::KorteVygenEdmonds::min_weight_perfect_matching(graph)
            }
            Algorithm::FastBlossom => {
                algorithms::FastBlossomMatching::min_weight_perfect_matching(graph)
            }
        }
    }
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
            let result = cli.algorithm.max_cardinality(&graph);
            print_matching_result(&result, output.as_deref(), false)?;
        }
        Commands::MinWeightPerfect { input, output } => {
            let graph = load_graph(&input)?;
            let result = cli
                .algorithm
                .min_weight_perfect(&graph)
                .map_err(anyhow::Error::msg)?;
            print_matching_result(&result, output.as_deref(), false)?;
        }
        Commands::Generate {
            n,
            m,
            output,
            type_,
        } => {
            let graph = match type_.as_str() {
                "complete" => graph::generator::complete_graph(n, (1.0, 100.0)),
                "sparse" => graph::generator::random_sparse(n, m as usize, (1.0, 100.0)),
                "grid" => {
                    let rows = ((n as f64).sqrt() as usize).max(1);
                    let cols = n.div_ceil(rows);
                    graph::generator::grid_graph(rows, cols, (1.0, 100.0))
                }
                _ => anyhow::bail!("Unknown graph type: {}", type_),
            };
            graph.to_json_file(&output)?;
            println!(
                "Generated graph with {} vertices and {} edges",
                graph.n,
                graph.edges.len()
            );
        }
        Commands::Bench { config, output } => {
            run_benchmarks(config, &output)?;
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
        "dimacs" | "dat" | "txt" => {
            graph::Graph::from_dimacs_file(path).map_err(anyhow::Error::from)
        }
        _ => graph::Graph::from_json_file(path)
            .or_else(|_| graph::Graph::from_dimacs_file(path))
            .with_context(|| format!("Failed to parse graph from '{}'", path)),
    }
}

fn print_matching_result(
    result: &algorithms::MatchingResult,
    output: Option<&str>,
    compact: bool,
) -> Result<()> {
    if compact {
        println!("=== Matching Result ===");
        println!("  size:         {}", result.edges.len());
        println!("  weight:       {:.6}", result.total_weight);
        println!("  perfect:      {}", result.is_perfect);
        println!("  iterations:   {}", result.metadata.iterations);
        println!(
            "  blossoms:     created={}, expanded={}",
            result.metadata.blossoms_created, result.metadata.blossoms_expanded
        );
        println!("  dual_updates: {}", result.metadata.dual_updates);
        println!("  time:         {:.3} ms", result.metadata.runtime_ms);
        if !result.edges.is_empty() {
            print!("  edges:        ");
            for (i, &e) in result.edges.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", e);
            }
            println!();
        }
        println!();
    } else {
        print!("{result}");
    }

    if let Some(out) = output {
        std::fs::write(out, serde_json::to_string_pretty(result)?)?;
    }
    Ok(())
}

fn run_benchmarks(config_path: Option<String>, output_dir: &str) -> Result<()> {
    let config = if let Some(ref path) = config_path {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Could not read config file '{}'", path))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Could not parse config file '{}'", path))?
    } else {
        blossom_study::benchmark::BenchConfig::default()
    };

    println!("Running Blossom Benchmark Suite");
    println!("Output directory: {}", output_dir);
    println!("Graph sizes: {:?}", config.graph_sizes);
    println!("Densities: {:?}", config.densities);
    println!("Algorithms: {:?}", config.algorithms);
    println!();

    std::fs::create_dir_all(output_dir)?;

    println!(
        "{:<15} | {:>6} | {:>8} | {:>10} | {:>8} | {:>10} | {:>8}",
        "Algorithm", "V", "E", "Weight", "Size", "Time (ms)", "Perfect"
    );
    println!(
        "{:-<15}-|-{:-<6}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<10}-|-{:-<8}",
        "", "", "", "", "", "", ""
    );

    let mut records = Vec::new();

    for &n in &config.graph_sizes {
        for &density in &config.densities {
            let m = (((n * (n.saturating_sub(1))) as f64 * density * 0.5) as usize).max(1);
            let graph = if density >= 0.99 {
                graph::generator::complete_graph(n, (1.0, 100.0))
            } else {
                graph::generator::random_sparse(n, m, (1.0, 100.0))
            };

            for algo_name in &config.algorithms {
                let algo = match algo_name.to_lowercase().as_str() {
                    "edmonds" => Algorithm::Edmonds,
                    "korte_vygen" | "kv" => Algorithm::KorteVygen,
                    "fast_blossom" | "fast" => Algorithm::FastBlossom,
                    _ => continue,
                };

                let res = algo.max_cardinality(&graph);

                println!(
                    "{:<15} | {:>6} | {:>8} | {:>10.2} | {:>8} | {:>10.3} | {:>8}",
                    algo_name,
                    graph.n,
                    graph.edges.len(),
                    res.total_weight,
                    res.edges.len(),
                    res.metadata.runtime_ms,
                    res.is_perfect
                );

                records.push(serde_json::json!({
                    "algorithm": algo_name,
                    "n": graph.n,
                    "m": graph.edges.len(),
                    "density": density,
                    "result": res,
                }));
            }
        }
    }

    let out_path = format!("{}/bench_results.json", output_dir);
    std::fs::write(&out_path, serde_json::to_string_pretty(&records)?)?;
    println!("\nWrote detailed benchmark data to {}", out_path);
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
    println!(
        "Commands: load <file>, max, min, graph, edges, result, save <file>, algo <name>, quit"
    );
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
                            println!(
                                "Usage: algo <edmonds|korte_vygen|korte-vygen|fast|fast_blossom>"
                            );
                            println!("Current: {:?}", current_algorithm);
                            continue;
                        }
                        current_algorithm = match parts[1].to_lowercase().as_str() {
                            "edmonds" => Algorithm::Edmonds,
                            "korte_vygen" | "korte-vygen" | "korte" | "kv" => Algorithm::KorteVygen,
                            "fast" | "fast_blossom" => Algorithm::FastBlossom,
                            _ => {
                                println!(
                                    "Unknown algorithm: {}. Use 'edmonds', 'korte_vygen', or 'fast'",
                                    parts[1]
                                );
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
                            let result = current_algorithm.max_cardinality(g);
                            print_matching_result(&result, None, true)?;
                            last_result = Some(result);
                        } else {
                            println!("No graph loaded. Use 'load <file>' first.");
                        }
                    }
                    "min" => {
                        if let Some(ref g) = graph {
                            match current_algorithm.min_weight_perfect(g) {
                                Ok(result) => {
                                    print_matching_result(&result, None, true)?;
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
                            print_matching_result(r, None, true)?;
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
                        println!(
                            "  save <file>     - Save graph (.json/.dimacs) or result (.json)"
                        );
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
