//! Reduce a weighted false-infeasibility profile failure in one process.

use anyhow::{Context, Result, ensure};
use blossom_study::{
    FastBlossomMatching,
    algorithms::{
        blossom_engine_minimum_weight_perfect_matching, duan_minimum_weight_perfect_matching,
        minimum_weight_perfect_matching, sparse_minimum_weight_perfect_matching,
    },
    graph::{Edge, Graph},
};
use clap::Parser;
use serde_json::json;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: String,
    #[arg(long)]
    output: String,
    #[arg(long)]
    solver: String,
    #[arg(long, default_value = "graph does not have a perfect matching")]
    error_substring: String,
}

fn solve(graph: &Graph, name: &str) -> Result<(), String> {
    match name {
        "primal-dual" => minimum_weight_perfect_matching(graph).map(|_| ()),
        "blossom-engine" => blossom_engine_minimum_weight_perfect_matching(graph).map(|_| ()),
        "sparse-blossom" => sparse_minimum_weight_perfect_matching(graph).map(|_| ()),
        "duan-scaling" => duan_minimum_weight_perfect_matching(graph).map(|_| ()),
        _ => Err(format!("unknown weighted solver {name}")),
    }
}

fn preserves(n: usize, edges: Vec<Edge>, args: &Args, evaluations: &mut usize) -> bool {
    *evaluations += 1;
    let graph = Graph::from_edges(n, edges);
    let cardinality = FastBlossomMatching::max_cardinality_matching(&graph)
        .edges
        .len();
    cardinality * 2 == n
        && solve(&graph, &args.solver).is_err_and(|error| error.contains(&args.error_substring))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let graph = Graph::from_json_file(&args.input)
        .with_context(|| format!("could not read {}", args.input))?;
    let mut edges = graph.edges.clone();
    let original_count = edges.len();
    let mut evaluations = 0;
    ensure!(
        preserves(graph.n, edges.clone(), &args, &mut evaluations),
        "input graph does not reproduce a false-infeasibility failure"
    );

    // Remove large chunks first. A final single-edge pass gives the precise
    // local minimality guarantee stated in the output artifact.
    let mut granularity = 2;
    while edges.len() >= 2 {
        let chunk_size = edges.len().div_ceil(granularity);
        let mut reduced = false;
        for start in (0..edges.len()).step_by(chunk_size) {
            let end = (start + chunk_size).min(edges.len());
            let candidate = edges[..start]
                .iter()
                .chain(&edges[end..])
                .cloned()
                .collect();
            if preserves(graph.n, candidate, &args, &mut evaluations) {
                edges.drain(start..end);
                granularity = granularity.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if !reduced {
            if granularity >= edges.len() {
                break;
            }
            granularity = (2 * granularity).min(edges.len());
        }
    }

    let mut index = 0;
    while index < edges.len() {
        let mut candidate = edges.clone();
        candidate.remove(index);
        if preserves(graph.n, candidate, &args, &mut evaluations) {
            edges.remove(index);
            index = 0;
        } else {
            index += 1;
        }
    }

    let minimized_count = edges.len();
    let artifact = json!({
        "n": graph.n,
        "edges": edges,
        "solver": args.solver,
        "error_substring": args.error_substring,
        "reference": "fast-blossom returned a perfect matching",
        "original_edge_count": original_count,
        "minimized_edge_count": minimized_count,
        "predicate_evaluations": evaluations,
        "reduction": "one-edge-minimal for false infeasibility at fixed vertex count and weights"
    });
    std::fs::write(&args.output, serde_json::to_vec_pretty(&artifact)?)
        .with_context(|| format!("could not write {}", args.output))?;
    println!(
        "{} -> {} edges in {} evaluations",
        original_count, minimized_count, evaluations
    );
    Ok(())
}
