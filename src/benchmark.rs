//! Configuration types for the CLI benchmark runner.

use serde::{Deserialize, Serialize};

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
