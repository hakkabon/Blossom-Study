//! Blossom Algorithm Study - CLI and REPL for matching algorithms

use anyhow::{Context, Result};
use blossom_study::{algorithms, graph};
use clap::{Parser, Subcommand, ValueEnum};
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use rustyline::DefaultEditor;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Algorithm {
    /// Fast Blossom (DSU with timestamp LCA)
    FastBlossom,
    /// Experimental Micali–Vazirani track (may reject uncertified results)
    MicaliVazirani,
    /// Gabow-inspired heuristic; completeness and time bound are unproved
    #[value(name = "gabow-heuristic", alias = "gabow")]
    Gabow,
    /// Gabow's 2017 weighted matching approach to cardinality
    GabowWeightedApproach,
    /// Mehlhorn–Nobahari revision of Gabow's phased algorithm
    MehlhornNobahari,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum WeightedAlgorithm {
    /// Existing dense-matrix primal-dual baseline
    PrimalDual,
    /// Independent Galil/Edmonds primal-dual implementation
    BlossomEngine,
    /// Sparse event-driven primal-dual implementation
    SparseBlossom,
    /// Duan–Pettie–Su scaling reference (integer edge weights)
    DuanScaling,
}

impl WeightedAlgorithm {
    pub fn min_weight_perfect(
        &self,
        graph: &graph::Graph,
    ) -> Result<algorithms::MatchingResult, String> {
        match self {
            Self::PrimalDual => algorithms::minimum_weight_perfect_matching(graph),
            Self::BlossomEngine => {
                algorithms::blossom_engine_minimum_weight_perfect_matching(graph)
            }
            Self::SparseBlossom => algorithms::sparse_minimum_weight_perfect_matching(graph),
            Self::DuanScaling => algorithms::duan_minimum_weight_perfect_matching(graph),
        }
    }
}

impl Algorithm {
    pub fn max_cardinality(
        &self,
        graph: &graph::Graph,
    ) -> Result<algorithms::MatchingResult, String> {
        match self {
            Algorithm::FastBlossom => {
                algorithms::general_cardinality::solve_general_cardinality("fast-blossom", graph)
            }
            Algorithm::MicaliVazirani => algorithms::MicaliVazirani::new(graph)
                .max_cardinality_matching(graph)
                .map_err(|error| error.to_string()),
            Algorithm::Gabow => {
                algorithms::general_cardinality::solve_general_cardinality("gabow-heuristic", graph)
            }
            Algorithm::GabowWeightedApproach => {
                algorithms::general_cardinality::solve_general_cardinality(
                    "gabow-weighted-approach",
                    graph,
                )
            }
            Algorithm::MehlhornNobahari => {
                algorithms::general_cardinality::solve_general_cardinality(
                    "mehlhorn-nobahari",
                    graph,
                )
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "blossom_study")]
#[command(about = "Study and benchmark general-graph matching algorithms")]
#[command(version)]
struct Cli {
    /// Cardinality solver (weighted commands select their solver separately)
    #[arg(short, long, value_enum, default_value = "fast-blossom")]
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
        /// Weighted solver implementation
        #[arg(long, value_enum, default_value = "primal-dual")]
        solver: WeightedAlgorithm,
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
            if matches!(cli.algorithm, Algorithm::MicaliVazirani) {
                eprintln!(
                    "Warning: Micali–Vazirani is experimental, its completeness is unestablished, and its recursive searches are bounded; it may return an error."
                );
            }
            let result = cli
                .algorithm
                .max_cardinality(&graph)
                .map_err(anyhow::Error::msg)?;
            print_matching_result(&result, output.as_deref(), false)?;
        }
        Commands::MinWeightPerfect {
            input,
            output,
            solver,
        } => {
            let graph = load_graph(&input)?;
            let result = solver
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

    validate_benchmark_config(&config)?;

    println!("Running Blossom Benchmark Suite");
    println!("Output directory: {}", output_dir);
    println!("Graph sizes: {:?}", config.graph_sizes);
    println!("Densities: {:?}", config.densities);
    println!("Weight ranges: {:?}", config.weight_ranges);
    println!("Runs per configuration: {}", config.runs_per_config);
    println!("Graph seed: {}", config.seed);
    println!("Algorithms: {:?}", config.algorithms);
    println!();

    std::fs::create_dir_all(output_dir)?;

    println!(
        "{:<24} | {:<18} | {:>5} | {:>10} | {:>6} | {:>8} | {:>10} | {:>8} | {:>10} | {:>8}",
        "Problem",
        "Algorithm",
        "Run",
        "Weight range",
        "V",
        "E",
        "Weight",
        "Size",
        "Time (ms)",
        "Perfect"
    );
    println!(
        "{:-<24}-|-{:-<18}-|-{:-<5}-|-{:-<10}-|-{:-<6}-|-{:-<8}-|-{:-<10}-|-{:-<8}-|-{:-<10}-|-{:-<8}",
        "", "", "", "", "", "", "", "", "", ""
    );

    let mut records = Vec::new();
    let mut rng = StdRng::seed_from_u64(config.seed);

    for &n in &config.graph_sizes {
        for &density in &config.densities {
            for &(weight_min, weight_max) in &config.weight_ranges {
                for run in 0..config.runs_per_config {
                    let graph =
                        generate_benchmark_graph(n, density, (weight_min, weight_max), &mut rng);
                    let observed_weight_min = graph
                        .edges
                        .iter()
                        .map(graph::Edge::weight_or_one)
                        .reduce(f64::min);
                    let observed_weight_max = graph
                        .edges
                        .iter()
                        .map(graph::Edge::weight_or_one)
                        .reduce(f64::max);
                    let graph_signature = benchmark_graph_signature(&graph);
                    let mut expected_cardinality: Option<usize> = None;
                    let mut expected_weight: Option<f64> = None;
                    for algo_name in &config.algorithms {
                        let (problem, canonical_name, result) = match algo_name
                            .to_lowercase()
                            .as_str()
                        {
                            "fast_blossom" | "fast" => (
                                "maximum-cardinality",
                                "fast-blossom",
                                Algorithm::FastBlossom.max_cardinality(&graph),
                            ),
                            "gabow" | "gabow_heuristic" | "gabow-heuristic" => (
                                "maximum-cardinality",
                                "gabow-heuristic",
                                Algorithm::Gabow.max_cardinality(&graph),
                            ),
                            "gabow-weighted-approach" | "gabow_weighted_approach" => (
                                "maximum-cardinality",
                                "gabow-weighted-approach",
                                Algorithm::GabowWeightedApproach.max_cardinality(&graph),
                            ),
                            "mehlhorn-nobahari" | "mehlhorn_nobahari" | "mn" => (
                                "maximum-cardinality",
                                "mehlhorn-nobahari",
                                Algorithm::MehlhornNobahari.max_cardinality(&graph),
                            ),
                            "micali_vazirani" | "mv" => {
                                eprintln!(
                                    "Skipping {algo_name}: Micali–Vazirani is experimental and excluded from the generic end-to-end benchmark."
                                );
                                continue;
                            }
                            "primal_dual" | "primal-dual" | "mwpm" => (
                                "minimum-weight-perfect",
                                "primal-dual",
                                WeightedAlgorithm::PrimalDual.min_weight_perfect(&graph),
                            ),
                            "duan-scaling" | "duan_scaling" | "duan" => (
                                "minimum-weight-perfect",
                                "duan-scaling",
                                WeightedAlgorithm::DuanScaling.min_weight_perfect(&graph),
                            ),
                            "blossom_engine" | "blossom-engine" => (
                                "minimum-weight-perfect",
                                "blossom-engine",
                                WeightedAlgorithm::BlossomEngine.min_weight_perfect(&graph),
                            ),
                            "sparse_blossom" | "sparse-blossom" | "sparse" => (
                                "minimum-weight-perfect",
                                "sparse-blossom",
                                WeightedAlgorithm::SparseBlossom.min_weight_perfect(&graph),
                            ),
                            _ => anyhow::bail!("Unknown benchmark algorithm '{algo_name}'"),
                        };

                        let res = match result {
                            Ok(result) => result,
                            Err(error) => {
                                eprintln!(
                                    "Skipping {algo_name} for n={n}, density={density}: {error}"
                                );
                                continue;
                            }
                        };
                        res.validate(&graph).map_err(|error| {
                            anyhow::anyhow!("{canonical_name} returned an invalid result: {error}")
                        })?;

                        match problem {
                            "maximum-cardinality" => match expected_cardinality {
                                Some(expected) if res.edges.len() != expected => anyhow::bail!(
                                    "{canonical_name} returned cardinality {}, but another selected cardinality solver returned {expected} for n={n}, density={density}",
                                    res.edges.len()
                                ),
                                None => expected_cardinality = Some(res.edges.len()),
                                _ => {}
                            },
                            "minimum-weight-perfect" => match expected_weight {
                                Some(expected)
                                    if (res.total_weight - expected).abs()
                                        > 1e-9
                                            * expected
                                                .abs()
                                                .max(res.total_weight.abs())
                                                .max(1.0) =>
                                {
                                    anyhow::bail!(
                                        "{canonical_name} returned weight {}, but another selected weighted solver returned {expected} for n={n}, density={density}",
                                        res.total_weight
                                    )
                                }
                                None => expected_weight = Some(res.total_weight),
                                _ => {}
                            },
                            _ => unreachable!("all benchmark algorithms have a problem category"),
                        }

                        println!(
                            "{:<24} | {:<18} | {:>5} | {:>5.1}..{:>4.1} | {:>6} | {:>8} | {:>10.2} | {:>8} | {:>10.3} | {:>8}",
                            problem,
                            canonical_name,
                            run + 1,
                            weight_min,
                            weight_max,
                            graph.n,
                            graph.edges.len(),
                            res.total_weight,
                            res.edges.len(),
                            res.metadata.runtime_ms,
                            res.is_perfect
                        );

                        records.push(serde_json::json!({
                            "problem": problem,
                            "algorithm": canonical_name,
                            "n": graph.n,
                            "m": graph.edges.len(),
                            "density": density,
                            "weight_range": [weight_min, weight_max],
                            "observed_weight_min": observed_weight_min,
                            "observed_weight_max": observed_weight_max,
                            "graph_signature": graph_signature,
                            "seed": config.seed,
                            "run": run + 1,
                            "result": res,
                        }));
                    }
                }
            }
        }
    }

    let out_path = format!("{}/bench_results.json", output_dir);
    std::fs::write(&out_path, serde_json::to_string_pretty(&records)?)?;
    println!("\nWrote detailed benchmark data to {}", out_path);
    Ok(())
}

fn validate_benchmark_config(config: &blossom_study::benchmark::BenchConfig) -> Result<()> {
    if config.graph_sizes.is_empty() {
        anyhow::bail!("benchmark config must include at least one graph size");
    }
    if config.densities.is_empty() {
        anyhow::bail!("benchmark config must include at least one density");
    }
    if config.weight_ranges.is_empty() {
        anyhow::bail!("benchmark config must include at least one weight range");
    }
    if config.algorithms.is_empty() {
        anyhow::bail!("benchmark config must include at least one algorithm");
    }
    if config.runs_per_config == 0 {
        anyhow::bail!("runs_per_config must be greater than zero");
    }
    for &density in &config.densities {
        if !density.is_finite() || !(0.0..=1.0).contains(&density) {
            anyhow::bail!("benchmark density must be finite and in [0, 1], got {density}");
        }
    }
    for &(minimum, maximum) in &config.weight_ranges {
        if !minimum.is_finite() || !maximum.is_finite() || minimum >= maximum {
            anyhow::bail!(
                "benchmark weight ranges must have finite bounds with minimum < maximum, got [{minimum}, {maximum})"
            );
        }
    }
    Ok(())
}

fn generate_benchmark_graph(
    n: usize,
    density: f64,
    weight_range: (f64, f64),
    rng: &mut StdRng,
) -> graph::Graph {
    let maximum_edges = n.saturating_mul(n.saturating_sub(1)) / 2;
    let edge_count = if density >= 0.99 {
        maximum_edges
    } else {
        (maximum_edges as f64 * density) as usize
    };
    let mut pairs = Vec::with_capacity(maximum_edges);
    for u in 0..n {
        for v in (u + 1)..n {
            pairs.push((u, v));
        }
    }
    if edge_count < maximum_edges {
        pairs.shuffle(rng);
        pairs.truncate(edge_count);
    }

    let edges = pairs
        .into_iter()
        .map(|(u, v)| graph::Edge::new(u, v, Some(rng.gen_range(weight_range.0..weight_range.1))))
        .collect();
    graph::Graph::from_edges(n, edges)
}

fn benchmark_graph_signature(graph: &graph::Graph) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut mix = |value: u64| {
        hash ^= value;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(graph.n as u64);
    for edge in &graph.edges {
        mix(edge.u as u64);
        mix(edge.v as u64);
        mix(edge.weight_or_one().to_bits());
    }
    format!("{hash:016x}")
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
    println!(
        "Cardinality algorithms: fast_blossom, gabow-heuristic (alias: gabow), gabow_weighted_approach, mehlhorn_nobahari, micali_vazirani (experimental)"
    );
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
                                "Usage: algo <fast_blossom|gabow-heuristic|gabow_weighted_approach|mehlhorn_nobahari|micali_vazirani>"
                            );
                            println!("Current: {:?}", current_algorithm);
                            continue;
                        }
                        current_algorithm = match parts[1].to_lowercase().as_str() {
                            "fast" | "fast_blossom" => Algorithm::FastBlossom,
                            "mv" | "micali_vazirani" | "micali-vazirani" => {
                                Algorithm::MicaliVazirani
                            }
                            "gabow" | "gabow_heuristic" | "gabow-heuristic" => Algorithm::Gabow,
                            "gabow_weighted_approach" | "gabow-weighted-approach" => {
                                Algorithm::GabowWeightedApproach
                            }
                            "mn" | "mehlhorn_nobahari" | "mehlhorn-nobahari" => {
                                Algorithm::MehlhornNobahari
                            }
                            _ => {
                                println!(
                                    "Unknown algorithm: {}. Use 'fast_blossom', 'gabow-heuristic', 'gabow_weighted_approach', 'mehlhorn_nobahari', or 'micali_vazirani'",
                                    parts[1]
                                );
                                continue;
                            }
                        };
                        println!("Algorithm set to: {:?}", current_algorithm);
                        if matches!(current_algorithm, Algorithm::MicaliVazirani) {
                            println!(
                                "Warning: this experimental solver may return an error and is excluded from generic benchmarks."
                            );
                        }
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
                            match current_algorithm.max_cardinality(g) {
                                Ok(result) => {
                                    print_matching_result(&result, None, true)?;
                                    last_result = Some(result);
                                }
                                Err(error) => println!("Error: {error}"),
                            }
                        } else {
                            println!("No graph loaded. Use 'load <file>' first.");
                        }
                    }
                    "min" => {
                        if let Some(ref g) = graph {
                            let solver = match parts.get(1).map(|name| name.to_lowercase()) {
                                None => WeightedAlgorithm::PrimalDual,
                                Some(name) if matches!(name.as_str(), "primal-dual" | "mwpm") => {
                                    WeightedAlgorithm::PrimalDual
                                }
                                Some(name)
                                    if matches!(
                                        name.as_str(),
                                        "blossom-engine" | "blossom_engine"
                                    ) =>
                                {
                                    WeightedAlgorithm::BlossomEngine
                                }
                                Some(name)
                                    if matches!(
                                        name.as_str(),
                                        "sparse" | "sparse-blossom" | "sparse_blossom"
                                    ) =>
                                {
                                    WeightedAlgorithm::SparseBlossom
                                }
                                Some(name)
                                    if matches!(
                                        name.as_str(),
                                        "duan-scaling" | "duan_scaling" | "duan"
                                    ) =>
                                {
                                    WeightedAlgorithm::DuanScaling
                                }
                                Some(name) => {
                                    println!(
                                        "Unknown weighted solver: {name}. Use 'primal-dual', 'blossom-engine', 'sparse-blossom', or 'duan-scaling'"
                                    );
                                    continue;
                                }
                            };
                            match solver.min_weight_perfect(g) {
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
                        println!(
                            "  min [solver]    - Run a weighted solver (see README for names and status)"
                        );
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
