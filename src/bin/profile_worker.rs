//! One solver invocation per process for peak-RSS profiling.
//!
//! The Python driver creates the graph and measures the child process. This
//! worker keeps dispatch, result validation, and JSON output identical across
//! solvers, including the bipartite and experimental tracks.

use anyhow::{Context, Result, bail};
use blossom_study::{
    MicaliVazirani,
    algorithms::{
        self, FastBlossomMatching, gabow_heuristic_matching, gabow_maximum_cardinality_matching,
        mn_maximum_cardinality_matching,
    },
    graph::Graph,
};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    solver: String,
    #[arg(long)]
    input: String,
    #[arg(long)]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let graph = Graph::from_json_file(&args.input)
        .with_context(|| format!("could not read graph {}", args.input))?;
    let result = match args.solver.as_str() {
        "fast-blossom" => Ok(FastBlossomMatching::max_cardinality_matching(&graph)),
        "gabow-heuristic" => Ok(gabow_heuristic_matching(&graph)),
        "gabow-weighted-approach" => Ok(gabow_maximum_cardinality_matching(&graph)),
        "mehlhorn-nobahari" => Ok(mn_maximum_cardinality_matching(&graph)),
        "micali-vazirani" => MicaliVazirani::new(&graph)
            .max_cardinality_matching(&graph)
            .map_err(|error| error.to_string()),
        "primal-dual" => algorithms::minimum_weight_perfect_matching(&graph),
        "blossom-engine" => algorithms::blossom_engine_minimum_weight_perfect_matching(&graph),
        "sparse-blossom" => algorithms::sparse_minimum_weight_perfect_matching(&graph),
        "duan-scaling" => algorithms::duan_minimum_weight_perfect_matching(&graph),
        "hopcroft-karp" => {
            algorithms::hopcroft_karp_maximum_cardinality_matching(&graph, graph.n / 2)
        }
        "hungarian" => algorithms::hungarian_minimum_weight_perfect_matching(&graph, graph.n / 2),
        other => bail!("unknown profiler solver {other}"),
    }
    .map_err(anyhow::Error::msg)?;
    result.validate(&graph).map_err(anyhow::Error::msg)?;
    std::fs::write(&args.output, serde_json::to_vec(&result)?)
        .with_context(|| format!("could not write result {}", args.output))?;
    Ok(())
}
