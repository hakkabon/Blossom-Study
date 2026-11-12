//! Adapter for the Duan–Pettie–Su scaling implementation in `implementations/`.
//!
//! The implementation accepts signed `i64` costs. We preserve exact ordering
//! only when every input `f64` can be represented as an integer multiple of a
//! common binary unit within that range. Inputs outside it return an error.

use super::{
    MatchingMetadata, MatchingResult,
    mwpm::{WeightScaler, filter_edges},
};
use crate::graph::Graph;
use std::time::Instant;

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Minimum-weight perfect matching via integer scaling and the DPS solver.
pub fn duan_minimum_weight_perfect_matching(graph: &Graph) -> Result<MatchingResult, String> {
    if !graph.n.is_multiple_of(2) {
        return Err("a perfect matching requires an even number of vertices".into());
    }
    let start = Instant::now();
    let filtered = filter_edges(graph)?;
    let weights: Vec<f64> = filtered.iter().map(|e| e.weight).collect();
    let scaler = WeightScaler::new(&weights);
    let scaled: Vec<i128> = weights
        .iter()
        .map(|&w| scaler.to_integer(w))
        .collect::<Result<_, _>>()?;
    let unit = scaled
        .iter()
        .try_fold(0_i128, |unit, &cost| {
            cost.checked_abs()
                .map(|cost| gcd(unit, cost))
                .ok_or_else(|| "weight magnitude exceeds supported i128 range".to_string())
        })?
        .max(1);
    let edges = filtered
        .iter()
        .zip(&scaled)
        .map(|(edge, &cost)| {
            let max_weight = (cost / unit)
                .checked_neg()
                .and_then(|value| i64::try_from(value).ok())
                .ok_or_else(|| "exact edge costs exceed the Duan solver's i64 range".to_string())?;
            Ok(duan_mwpm::Edge::new(edge.u, edge.v, max_weight))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let solved = duan_mwpm::maximum_weight_perfect_matching_with_options(
        graph.n,
        &edges,
        &duan_mwpm::Options {
            tau: None,
            check_invariants: false,
        },
    )
    .map_err(|error| error.to_string())?;
    let mut selected = Vec::with_capacity(graph.n / 2);
    for (u, v) in solved.pairs() {
        let index = filtered
            .binary_search_by_key(&(u, v), |e| (e.u, e.v))
            .map_err(|_| format!("Duan solver returned absent edge ({u}, {v})"))?;
        selected.push(filtered[index].original_index);
    }
    selected.sort_unstable();
    let total_weight = selected
        .iter()
        .map(|&i| graph.edges[i].weight_or_one())
        .sum();
    let result = MatchingResult {
        is_perfect: selected.len() * 2 == graph.n,
        edges: selected,
        total_weight,
        metadata: MatchingMetadata {
            iterations: solved.stats.scales,
            augmentations: solved.stats.augmentations,
            blossoms_created: solved.stats.contractions,
            blossoms_expanded: solved.stats.dissolutions,
            dual_updates: solved.stats.dual_events,
            runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
        },
    };
    result.validate(graph)?;
    if !result.is_perfect {
        return Err("Duan solver did not return a perfect matching".into());
    }
    Ok(result)
}
