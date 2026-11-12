//! Deterministic comparisons beyond the exhaustive small-graph suite.
//! Smaller cases use independent subset DP; larger cases use cross-solver
//! agreement and matching validation, which is evidence rather than proof.

#[path = "common/differential.rs"]
mod differential;

use blossom_study::{
    MatchingResult,
    algorithms::{
        blossom_engine_minimum_weight_perfect_matching, hopcroft_karp_maximum_cardinality_matching,
        hungarian_minimum_weight_perfect_matching, minimum_weight_perfect_matching,
        sparse_minimum_weight_perfect_matching,
    },
    benchmark::GENERAL_CARDINALITY_SOLVERS,
    duan_minimum_weight_perfect_matching,
    graph::{Edge, Graph},
};
use differential::{Case, Failure, assert_consistent};

const SEEDS: [u64; 2] = [0x5eed_2026_0000_0001, 0x5eed_2026_0000_0002];

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

/// Plant a perfect matching for even-order cases, then add reproducible
/// chords. Odd-order cases deliberately leave one vertex exposed by the plant.
fn general_case(n: usize, percent: u64, seed: u64, weighted: bool) -> Case {
    let mut state = seed ^ ((n as u64) << 32) ^ percent;
    let mut edges = Vec::new();
    for u in 0..n {
        for v in u + 1..n {
            let planted = u % 2 == 0 && v == u + 1;
            if planted || next(&mut state) % 100 < percent {
                let weight = weighted.then(|| ((next(&mut state) >> 32) % 257) as f64 / 8.0 - 16.0);
                edges.push(Edge::new(u, v, weight));
            }
        }
    }
    Case {
        name: format!("general-{percent}pct-n{n}-seed{seed:x}"),
        seed,
        n,
        edges,
    }
}

fn bipartite_case(k: usize, percent: u64, seed: u64) -> Case {
    let mut state = seed ^ ((k as u64) << 32) ^ percent;
    let mut edges = Vec::new();
    for left in 0..k {
        for right in 0..k {
            if left == right || next(&mut state) % 100 < percent {
                let weight = ((next(&mut state) >> 32) % 257) as f64 / 8.0 - 16.0;
                edges.push(Edge::new(left, k + right, Some(weight)));
            }
        }
    }
    Case {
        name: format!("bipartite-{percent}pct-k{k}-seed{seed:x}"),
        seed,
        n: 2 * k,
        edges,
    }
}

/// Two odd components cannot have a perfect matching, even though the graph
/// has even order. This catches false feasibility reports on larger inputs.
fn infeasible_case(n: usize, seed: u64) -> Case {
    let split = n / 2 + usize::from((n / 2).is_multiple_of(2));
    let mut state = seed ^ ((n as u64) << 32);
    let mut edges = Vec::new();
    for u in 0..n {
        for v in u + 1..n {
            let same_component = (u < split) == (v < split);
            let cycle_edge = v == u + 1 || (u == 0 && v == split - 1) || (u == split && v == n - 1);
            if same_component && (cycle_edge || next(&mut state) % 100 < 55) {
                let weight = ((next(&mut state) >> 32) % 257) as f64 / 8.0 - 16.0;
                edges.push(Edge::new(u, v, Some(weight)));
            }
        }
    }
    Case {
        name: format!("two-odd-components-n{n}-seed{seed:x}"),
        seed,
        n,
        edges,
    }
}

/// Independent bitmask oracle, used only when n <= 18.
fn cardinality_oracle(graph: &Graph) -> usize {
    fn solve(mask: usize, neighbors: &[usize], memo: &mut [Option<u8>]) -> u8 {
        if mask == 0 {
            return 0;
        }
        if let Some(value) = memo[mask] {
            return value;
        }
        let u = mask.trailing_zeros() as usize;
        let rest = mask & !(1 << u);
        let mut best = solve(rest, neighbors, memo);
        let mut candidates = neighbors[u] & rest;
        while candidates != 0 {
            let bit = candidates & candidates.wrapping_neg();
            candidates &= !bit;
            best = best.max(1 + solve(rest & !bit, neighbors, memo));
        }
        memo[mask] = Some(best);
        best
    }
    let mut neighbors = vec![0; graph.n];
    for edge in &graph.edges {
        neighbors[edge.u] |= 1 << edge.v;
        neighbors[edge.v] |= 1 << edge.u;
    }
    solve(
        (1 << graph.n) - 1,
        &neighbors,
        &mut vec![None; 1 << graph.n],
    ) as usize
}

fn weighted_oracle(graph: &Graph) -> Option<f64> {
    fn solve(
        mask: usize,
        costs: &[Vec<Option<f64>>],
        memo: &mut [Option<Option<f64>>],
    ) -> Option<f64> {
        if mask == 0 {
            return Some(0.0);
        }
        if let Some(value) = memo[mask] {
            return value;
        }
        let u = mask.trailing_zeros() as usize;
        let rest = mask & !(1 << u);
        let mut best: Option<f64> = None;
        let mut candidates = rest;
        while candidates != 0 {
            let bit = candidates & candidates.wrapping_neg();
            candidates &= !bit;
            let v = bit.trailing_zeros() as usize;
            if let (Some(weight), Some(suffix)) = (costs[u][v], solve(rest & !bit, costs, memo)) {
                let candidate = weight + suffix;
                best = Some(best.map_or(candidate, |current| current.min(candidate)));
            }
        }
        memo[mask] = Some(best);
        best
    }
    let mut costs = vec![vec![None; graph.n]; graph.n];
    for edge in &graph.edges {
        let cost = edge.weight_or_one();
        for (u, v) in [(edge.u, edge.v), (edge.v, edge.u)] {
            costs[u][v] = Some(costs[u][v].map_or(cost, |old: f64| old.min(cost)));
        }
    }
    solve((1 << graph.n) - 1, &costs, &mut vec![None; 1 << graph.n])
}

fn same_weight(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-9 * actual.abs().max(expected.abs()).max(1.0)
}

fn check_cardinality(graph: &Graph) -> Result<(), Failure> {
    let oracle = (graph.n <= 18).then(|| cardinality_oracle(graph));
    let mut baseline = None;
    for solver in GENERAL_CARDINALITY_SOLVERS {
        let result = (solver.solve)(graph);
        result
            .validate(graph)
            .map_err(|error| Failure::new(format!("invalid:{}", solver.name), error))?;
        let expected = oracle.or(baseline).unwrap_or(result.edges.len());
        if result.edges.len() != expected {
            return Err(Failure::new(
                format!("cardinality:{}", solver.name),
                format!("got {}, expected {expected}", result.edges.len()),
            ));
        }
        baseline.get_or_insert(result.edges.len());
    }
    Ok(())
}

const WEIGHTED: [(&str, fn(&Graph) -> Result<MatchingResult, String>); 4] = [
    ("primal-dual", minimum_weight_perfect_matching),
    (
        "blossom-engine",
        blossom_engine_minimum_weight_perfect_matching,
    ),
    ("sparse-blossom", sparse_minimum_weight_perfect_matching),
    ("duan-scaling", duan_minimum_weight_perfect_matching),
];

fn check_weighted(graph: &Graph) -> Result<(), Failure> {
    let oracle = (graph.n <= 18).then(|| weighted_oracle(graph));
    let feasible = oracle.map_or_else(
        || {
            blossom_study::FastBlossomMatching::max_cardinality_matching(graph)
                .edges
                .len()
                * 2
                == graph.n
        },
        |optimum| optimum.is_some(),
    );
    let mut baseline = oracle.flatten();
    for (name, solve) in WEIGHTED {
        match (feasible, solve(graph)) {
            (false, Err(error)) => {
                if error.is_empty() {
                    return Err(Failure::new(
                        format!("empty-error:{name}"),
                        "infeasibility error is empty",
                    ));
                }
            }
            (false, Ok(result)) => {
                return Err(Failure::new(
                    format!("false-feasible:{name}"),
                    format!("returned weight {}", result.total_weight),
                ));
            }
            (true, Err(error)) => {
                return Err(Failure::new(format!("rejected-feasible:{name}"), error));
            }
            (true, Ok(result)) => {
                result
                    .validate(graph)
                    .map_err(|error| Failure::new(format!("invalid:{name}"), error))?;
                if !result.is_perfect {
                    return Err(Failure::new(
                        format!("not-perfect:{name}"),
                        "returned partial matching",
                    ));
                }
                if let Some(expected) = baseline {
                    if !same_weight(result.total_weight, expected) {
                        return Err(Failure::new(
                            format!("weight:{name}"),
                            format!("got {}, expected {expected}", result.total_weight),
                        ));
                    }
                } else {
                    baseline = Some(result.total_weight);
                }
            }
        }
    }
    Ok(())
}

fn check_bipartite(graph: &Graph) -> Result<(), Failure> {
    check_cardinality(graph)?;
    check_weighted(graph)?;
    let k = graph.n / 2;
    let hopcroft_karp = hopcroft_karp_maximum_cardinality_matching(graph, k)
        .map_err(|error| Failure::new("hopcroft-karp-error", error))?;
    hopcroft_karp
        .validate(graph)
        .map_err(|error| Failure::new("hopcroft-karp-invalid", error))?;
    let reference = blossom_study::FastBlossomMatching::max_cardinality_matching(graph);
    if hopcroft_karp.edges.len() != reference.edges.len() {
        return Err(Failure::new(
            "hopcroft-karp-cardinality",
            format!(
                "got {}, expected {}",
                hopcroft_karp.edges.len(),
                reference.edges.len()
            ),
        ));
    }
    let hungarian = hungarian_minimum_weight_perfect_matching(graph, k)
        .map_err(|error| Failure::new("hungarian-error", error))?;
    hungarian
        .validate(graph)
        .map_err(|error| Failure::new("hungarian-invalid", error))?;
    let baseline = minimum_weight_perfect_matching(graph)
        .map_err(|error| Failure::new("bipartite-baseline-error", error))?;
    if !same_weight(hungarian.total_weight, baseline.total_weight) {
        return Err(Failure::new(
            "hungarian-weight",
            format!(
                "got {}, expected {}",
                hungarian.total_weight, baseline.total_weight
            ),
        ));
    }
    Ok(())
}

#[test]
fn larger_seeded_general_cardinality_cases_agree() {
    for n in [12, 18, 20, 32, 40] {
        for percent in [15, 65] {
            for seed in SEEDS {
                let case = general_case(n, percent, seed, false);
                assert_consistent("cardinality", &case, check_cardinality);
            }
        }
    }
    for seed in SEEDS {
        let case = general_case(21, 25, seed, false);
        assert_consistent("cardinality", &case, check_cardinality);
    }
}

#[test]
fn larger_seeded_weighted_cases_agree() {
    for n in [12, 18, 24, 32] {
        for percent in [18, 65] {
            for seed in SEEDS {
                let case = general_case(n, percent, seed, true);
                assert_consistent("weighted", &case, check_weighted);
            }
        }
    }
    for n in [18, 30] {
        for seed in SEEDS {
            let case = infeasible_case(n, seed);
            assert_consistent("weighted", &case, check_weighted);
        }
    }
}

#[test]
fn larger_seeded_bipartite_cases_agree() {
    for k in [12, 20] {
        for percent in [20, 70] {
            for seed in SEEDS {
                let case = bipartite_case(k, percent, seed);
                assert_consistent("bipartite", &case, check_bipartite);
            }
        }
    }
}
