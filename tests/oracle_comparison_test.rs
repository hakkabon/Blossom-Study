use blossom_study::{
    MatchingResult,
    algorithms::{
        blossom_engine_minimum_weight_perfect_matching, minimum_weight_perfect_matching,
        sparse_minimum_weight_perfect_matching,
    },
    benchmark::compare_general_cardinality,
    graph::{Edge, Graph},
};

fn maximum_cardinality_oracle(n: usize, edges: &[(usize, usize)]) -> usize {
    fn solve(mask: usize, neighbors: &[usize], memo: &mut [Option<usize>]) -> usize {
        if mask == 0 {
            return 0;
        }
        if let Some(result) = memo[mask] {
            return result;
        }

        let u = mask.trailing_zeros() as usize;
        let without_u = mask & !(1 << u);
        let mut best = solve(without_u, neighbors, memo);
        let mut available = neighbors[u] & without_u;
        while available != 0 {
            let bit = available & available.wrapping_neg();
            available &= !bit;
            best = best.max(1 + solve(without_u & !bit, neighbors, memo));
        }

        memo[mask] = Some(best);
        best
    }

    let mut neighbors = vec![0; n];
    for &(u, v) in edges {
        neighbors[u] |= 1 << v;
        neighbors[v] |= 1 << u;
    }
    let mut memo = vec![None; 1 << n];
    solve((1 << n) - 1, &neighbors, &mut memo)
}

#[test]
fn registered_general_cardinality_solvers_match_oracle_for_every_graph_through_six_vertices() {
    for n in 0..=6 {
        let possible_edges: Vec<_> = (0..n)
            .flat_map(|u| (u + 1..n).map(move |v| (u, v)))
            .collect();

        for edge_mask in 0..(1_usize << possible_edges.len()) {
            let edges: Vec<_> = possible_edges
                .iter()
                .enumerate()
                .filter(|(index, _)| edge_mask & (1 << index) != 0)
                .map(|(_, &edge)| edge)
                .collect();
            let graph = Graph::from_edges(
                n,
                edges.iter().map(|&(u, v)| Edge::new(u, v, None)).collect(),
            );
            let expected = maximum_cardinality_oracle(n, &edges);
            let results = compare_general_cardinality(&graph).unwrap_or_else(|error| {
                panic!("n={n}, edge_mask={edge_mask}: comparison failed: {error}")
            });

            for entry in results {
                entry.result.validate(&graph).unwrap_or_else(|error| {
                    panic!(
                        "n={n}, edge_mask={edge_mask}, solver={}: {error}",
                        entry.solver.name
                    )
                });
                assert_eq!(
                    entry.result.edges.len(),
                    expected,
                    "n={n}, edge_mask={edge_mask}, solver={}",
                    entry.solver.name
                );
            }
        }
    }
}

fn minimum_weight_oracle(costs: &[Vec<Option<f64>>]) -> Option<f64> {
    fn solve(
        mask: usize,
        costs: &[Vec<Option<f64>>],
        memo: &mut [Option<Option<f64>>],
    ) -> Option<f64> {
        if mask == 0 {
            return Some(0.0);
        }
        if let Some(result) = memo[mask] {
            return result;
        }

        let u = mask.trailing_zeros() as usize;
        let remaining = mask & !(1 << u);
        let mut candidates = remaining;
        let mut best = None;
        while candidates != 0 {
            let bit = candidates & candidates.wrapping_neg();
            candidates &= !bit;
            let v = bit.trailing_zeros() as usize;
            let Some(weight) = costs[u][v] else {
                continue;
            };
            if let Some(suffix) = solve(remaining & !bit, costs, memo) {
                let candidate = weight + suffix;
                best = Some(best.map_or(candidate, |current: f64| current.min(candidate)));
            }
        }

        memo[mask] = Some(best);
        best
    }

    if costs.len() % 2 != 0 {
        return None;
    }
    let mut memo = vec![None; 1 << costs.len()];
    solve((1 << costs.len()) - 1, costs, &mut memo)
}

fn weighted_solvers() -> [(&'static str, fn(&Graph) -> Result<MatchingResult, String>); 3] {
    [
        ("primal-dual", minimum_weight_perfect_matching),
        (
            "blossom-engine",
            blossom_engine_minimum_weight_perfect_matching,
        ),
        ("sparse-blossom", sparse_minimum_weight_perfect_matching),
    ]
}

fn add_weighted_edge(u: usize, v: usize, state: &mut u64, edges: &mut Vec<Edge>) {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let weight = ((*state >> 32) % 161) as f64 / 8.0 - 10.0;
    edges.push(Edge::new(u, v, Some(weight)));
}

fn weighted_cost_matrix(graph: &Graph) -> Vec<Vec<Option<f64>>> {
    let mut costs = vec![vec![None; graph.n]; graph.n];
    for edge in &graph.edges {
        let weight = edge.weight_or_one();
        for (u, v) in [(edge.u, edge.v), (edge.v, edge.u)] {
            costs[u][v] = Some(costs[u][v].map_or(weight, |current: f64| current.min(weight)));
        }
    }
    costs
}

fn assert_weighted_solvers_match_oracle(graph: &Graph, expected: Option<f64>, context: &str) {
    for (name, solve) in weighted_solvers() {
        match (solve(graph), expected) {
            (Ok(result), Some(expected_weight)) => {
                result.validate(graph).unwrap_or_else(|error| {
                    panic!("{context}, solver={name}: invalid result: {error}")
                });
                assert!(result.is_perfect, "{context}, solver={name}: not perfect");
                let tolerance = 1e-9
                    * expected_weight
                        .abs()
                        .max(result.total_weight.abs())
                        .max(1.0);
                assert!(
                    (result.total_weight - expected_weight).abs() <= tolerance,
                    "{context}, solver={name}: got {}, expected {expected_weight}",
                    result.total_weight
                );
            }
            (Err(error), None) => {
                assert!(
                    !error.is_empty(),
                    "{context}, solver={name}: infeasibility error should be explicit"
                );
            }
            (Ok(result), None) => panic!(
                "{context}, solver={name}: returned weight {} without a perfect matching",
                result.total_weight
            ),
            (Err(error), Some(expected_weight)) => {
                panic!(
                    "{context}, solver={name}: rejected feasible optimum {expected_weight}: {error}"
                )
            }
        }
    }
}

#[test]
fn weighted_solvers_match_independent_oracle_on_sparse_planted_instances() {
    let mut state = 0x6f72_6163_6c65_5eed_u64;
    for n in [2_usize, 4, 6, 8, 10] {
        for case in 0..40 {
            let mut edges = Vec::new();

            for u in (0..n).step_by(2) {
                add_weighted_edge(u, u + 1, &mut state, &mut edges);
            }
            for u in 0..n {
                for v in (u + 1)..n {
                    if edges.iter().any(|edge: &Edge| {
                        (edge.u == u && edge.v == v) || (edge.u == v && edge.v == u)
                    }) {
                        continue;
                    }
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if state.is_multiple_of(4) {
                        add_weighted_edge(u, v, &mut state, &mut edges);
                    }
                }
            }

            let graph = Graph::from_edges(n, edges);
            let costs = weighted_cost_matrix(&graph);
            let expected = minimum_weight_oracle(&costs);
            assert_weighted_solvers_match_oracle(&graph, expected, &format!("n={n}, case={case}"));
        }
    }
}

#[test]
fn weighted_solvers_reject_odd_and_even_infeasible_graphs() {
    for (graph, n) in [
        (
            Graph::from_edges(
                3,
                vec![
                    Edge::new(0, 1, Some(1.0)),
                    Edge::new(1, 2, Some(2.0)),
                    Edge::new(2, 0, Some(3.0)),
                ],
            ),
            3,
        ),
        (
            Graph::from_edges(
                6,
                vec![
                    Edge::new(0, 1, Some(1.0)),
                    Edge::new(1, 2, Some(1.0)),
                    Edge::new(2, 0, Some(1.0)),
                    Edge::new(3, 4, Some(1.0)),
                    Edge::new(4, 5, Some(1.0)),
                    Edge::new(5, 3, Some(1.0)),
                ],
            ),
            6,
        ),
    ] {
        assert_eq!(graph.n, n);
        assert_eq!(minimum_weight_oracle(&weighted_cost_matrix(&graph)), None);
        assert_weighted_solvers_match_oracle(&graph, None, &format!("infeasible n={n}"));
    }
}

#[test]
fn weighted_solvers_accept_the_empty_perfect_matching() {
    let graph = Graph::from_edges(0, vec![]);
    assert_weighted_solvers_match_oracle(&graph, Some(0.0), "empty graph");
}
