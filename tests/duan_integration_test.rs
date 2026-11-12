use blossom_study::{
    duan_minimum_weight_perfect_matching,
    graph::{Edge, Graph},
};

fn oracle(n: usize, edges: &[Edge]) -> Option<f64> {
    fn recurse(
        mask: usize,
        costs: &[Vec<Option<f64>>],
        memo: &mut [Option<Option<f64>>],
    ) -> Option<f64> {
        if mask == 0 {
            return Some(0.0);
        }
        if let Some(answer) = memo[mask] {
            return answer;
        }
        let u = mask.trailing_zeros() as usize;
        let remaining = mask & !(1 << u);
        let mut best: Option<f64> = None;
        for v in u + 1..costs.len() {
            if remaining & (1 << v) == 0 {
                continue;
            }
            if let (Some(cost), Some(rest)) =
                (costs[u][v], recurse(remaining & !(1 << v), costs, memo))
            {
                best = Some(best.map_or(cost + rest, |current| current.min(cost + rest)));
            }
        }
        memo[mask] = Some(best);
        best
    }
    if n % 2 != 0 {
        return None;
    }
    let mut costs = vec![vec![None; n]; n];
    for edge in edges {
        if edge.u == edge.v {
            continue;
        }
        let cost = edge.weight_or_one();
        costs[edge.u][edge.v] = Some(costs[edge.u][edge.v].map_or(cost, |old: f64| old.min(cost)));
        costs[edge.v][edge.u] = costs[edge.u][edge.v];
    }
    recurse((1 << n) - 1, &costs, &mut vec![None; 1 << n])
}

#[test]
fn duan_scaling_agrees_with_exhaustive_small_graphs() {
    let mut state = 0x5eed_1234_5678_9abc_u64;
    for n in [0, 2, 4, 6, 8] {
        for case in 0..35 {
            let mut edges = Vec::new();
            for u in 0..n {
                for v in u + 1..n {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if state >> 61 < 5 {
                        let cost = ((state >> 32) % 81) as f64 / 4.0 - 10.0;
                        edges.push(Edge::new(u, v, Some(cost)));
                        if state & 7 == 0 {
                            edges.push(Edge::new(u, v, Some(cost + 2.0)));
                        }
                    }
                }
            }
            let graph = Graph::from_edges(n, edges);
            let expected = oracle(n, &graph.edges);
            match (duan_minimum_weight_perfect_matching(&graph), expected) {
                (Ok(result), Some(weight)) => {
                    result.validate(&graph).unwrap();
                    assert!(result.is_perfect, "n={n}, case={case}");
                    assert!(
                        (result.total_weight - weight).abs() < 1e-9,
                        "n={n}, case={case}"
                    );
                }
                (Err(_), None) => {}
                (actual, expected) => {
                    panic!("n={n}, case={case}: got {actual:?}, expected {expected:?}")
                }
            }
        }
    }
}
