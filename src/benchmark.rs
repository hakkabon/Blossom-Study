//! Configuration types for the CLI benchmark runner.

use crate::{algorithms::MatchingResult, graph::Graph};
use serde::{Deserialize, Serialize};

pub use crate::algorithms::general_cardinality::GENERAL_CARDINALITY_SOLVERS;
pub use crate::algorithms::general_cardinality::GeneralCardinalitySolver;

/// A checked result from one solver in the general-cardinality comparison.
pub struct GeneralCardinalityResult {
    pub solver: GeneralCardinalitySolver,
    pub result: MatchingResult,
}

/// Run every registered solver on the same input and reject invalid or
/// disagreeing cardinalities before results are presented as a comparison.
pub fn compare_general_cardinality(graph: &Graph) -> Result<Vec<GeneralCardinalityResult>, String> {
    let mut results = Vec::with_capacity(GENERAL_CARDINALITY_SOLVERS.len());
    let mut expected_cardinality = None;

    for &solver in GENERAL_CARDINALITY_SOLVERS {
        let result = (solver.solve)(graph);
        result
            .validate(graph)
            .map_err(|error| format!("{} returned an invalid matching: {error}", solver.name))?;

        match expected_cardinality {
            Some(expected) if result.edges.len() != expected => {
                return Err(format!(
                    "{} returned cardinality {}, but the comparison baseline returned {expected}",
                    solver.name,
                    result.edges.len()
                ));
            }
            None => expected_cardinality = Some(result.edges.len()),
            _ => {}
        }

        results.push(GeneralCardinalityResult { solver, result });
    }

    Ok(results)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchConfig {
    pub graph_sizes: Vec<usize>,
    pub densities: Vec<f64>,
    pub weight_ranges: Vec<(f64, f64)>,
    pub runs_per_config: usize,
    pub algorithms: Vec<String>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            graph_sizes: vec![10, 20, 30, 40, 50, 60, 80, 100],
            densities: vec![0.1, 0.3, 0.5, 0.8, 1.0],
            weight_ranges: vec![(1.0, 100.0)],
            runs_per_config: 5,
            algorithms: vec!["fast_blossom".to_string()],
        }
    }
}
