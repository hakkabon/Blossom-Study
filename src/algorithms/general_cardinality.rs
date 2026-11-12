//! Registry for comparable general-graph maximum-cardinality solvers.
//!
//! Keep solver identity and comparison labels here so CLI dispatch and
//! benchmark suites use the same named implementations.

use crate::{
    algorithms::{self, MatchingResult},
    graph::Graph,
};

/// One general-graph maximum-cardinality solver included in direct comparisons.
#[derive(Clone, Copy)]
pub struct GeneralCardinalitySolver {
    /// Stable label used in CLI benchmark output and result files.
    pub name: &'static str,
    /// Honest role/guarantee label, not a claim that the solver is production-ready.
    pub role: &'static str,
    /// Run this solver on a graph.
    pub solve: fn(&Graph) -> MatchingResult,
}

/// Solvers admitted to end-to-end general-cardinality comparisons.
///
/// Experimental Micali–Vazirani is deliberately excluded: its completeness is
/// unestablished. Bipartite Hopcroft–Karp is benchmarked in its own category.
pub const GENERAL_CARDINALITY_SOLVERS: &[GeneralCardinalitySolver] = &[
    GeneralCardinalitySolver {
        name: "fast-blossom",
        role: "Edmonds baseline",
        solve: algorithms::FastBlossomMatching::max_cardinality_matching,
    },
    GeneralCardinalitySolver {
        name: "gabow-heuristic",
        role: "Gabow-inspired heuristic; no O(m sqrt(n)) guarantee",
        solve: algorithms::gabow_heuristic_matching,
    },
    GeneralCardinalitySolver {
        name: "gabow-weighted-approach",
        role: "Gabow 2017 phased cardinality implementation",
        solve: algorithms::gabow_maximum_cardinality_matching,
    },
    GeneralCardinalitySolver {
        name: "mehlhorn-nobahari",
        role: "Revised Gabow shortest-path phase implementation",
        solve: algorithms::mn_maximum_cardinality_matching,
    },
];

/// Run a registered solver by its stable comparison label.
pub fn solve_general_cardinality(name: &str, graph: &Graph) -> Result<MatchingResult, String> {
    let solver = GENERAL_CARDINALITY_SOLVERS
        .iter()
        .find(|solver| solver.name == name)
        .ok_or_else(|| format!("unknown registered general-cardinality solver '{name}'"))?;
    Ok((solver.solve)(graph))
}
