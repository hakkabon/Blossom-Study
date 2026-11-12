//! Dual adjustment (Figure 2) and relaxed complementary slackness (§2.1).
use crate::{
    blossom::State,
    graph::Graph,
    search::{Criterion, Forest},
};
fn direction(label: u8) -> i128 {
    match label {
        1 => -1,
        2 => 1,
        _ => 0,
    }
}
pub(crate) fn adjust(g: &Graph, s: &mut State, f: &Forest, delta: i128) {
    assert!(delta > 0);
    for v in 0..s.y.len() {
        if g.alive[v] {
            s.y[v] += direction(f.label[s.top[v]]) * delta;
        }
    }
    for b in 0..s.nodes.len() {
        let n = &s.nodes[b];
        if n.active && n.parent.is_none() && !n.children.is_empty() {
            let d = direction(f.label[b]);
            s.nodes[b].z -= 2 * d * delta;
            assert!(s.nodes[b].z >= 0);
        }
    }
    s.dissolve_zero_roots();
}
/// Simulate repeated unit adjustments by jumping to the next integer event.
/// Rebuilding the complete forest after every event avoids the §5 split-findmin
/// and priority queue implementation; it does NOT attain the paper's bound.
pub(crate) fn next_event(g: &Graph, s: &State, f: &Forest, c: Criterion) -> Option<i128> {
    let mut delta = None;
    let mut take = |x: i128| {
        if x > 0 {
            delta = Some(delta.map_or(x, |d: i128| d.min(x)));
        }
    };
    for (b, n) in s.nodes.iter().enumerate() {
        if n.active && n.parent.is_none() && !n.children.is_empty() && f.label[b] == 2 {
            take(n.z / 2);
        }
    }
    for e in &g.edges {
        if !g.active_edge(e) || s.top[e.u] == s.top[e.v] {
            continue;
        }
        let slope = direction(f.label[s.top[e.u]]) + direction(f.label[s.top[e.v]]);
        if slope == 0 {
            continue;
        }
        let slack = s.y[e.u] + s.y[e.v] - e.w;
        let targets: &[i128] = match c {
            Criterion::Tight => &[0],
            Criterion::Final => &[0, -2],
            Criterion::Reduction => unreachable!(),
        };
        for &target in targets {
            let gap = target - slack;
            if gap % slope == 0 {
                take(gap / slope);
            }
        }
    }
    delta
}
/// Strong run-time checks, enabled by default and in all randomized tests.
/// Property 2 and Lemma 2.2; this verifies feasibility, not a standalone proof.
pub(crate) fn verify(g: &Graph, s: &State, relaxed: bool) {
    let bound = if relaxed { -2 } else { 0 };
    for v in 0..s.y.len() {
        if !g.alive[v] {
            continue;
        }
        if let Some(w) = s.mate[v] {
            assert!(g.alive[w]);
            assert_eq!(s.mate[w], Some(v));
        }
    }
    for e in &g.edges {
        if !g.active_edge(e) {
            continue;
        }
        let mut ancestors = vec![];
        let mut a = Some(s.leaf[e.u]);
        while let Some(b) = a {
            ancestors.push(b);
            a = s.nodes[b].parent;
        }
        let mut sum = s.y[e.u] + s.y[e.v];
        let mut a = Some(s.leaf[e.v]);
        while let Some(b) = a {
            if ancestors.contains(&b) {
                sum += s.nodes[b].z;
            }
            a = s.nodes[b].parent;
        }
        let slack = sum - e.w;
        assert!(
            slack >= bound,
            "domination: ({},{}) slack {}",
            e.u,
            e.v,
            slack
        );
        if s.mate[e.u] == Some(e.v) {
            assert!(slack <= 0, "matched edge too slack: {}", slack);
        }
    }
    for (b, n) in s.nodes.iter().enumerate() {
        if !n.active || n.children.is_empty() {
            continue;
        }
        assert!(n.z >= 0 && n.z % 2 == 0);
        assert!(n.parent.is_some() || n.z > 0);
        let vs = s.leaves(b);
        let mut count = 0;
        for &v in &vs {
            if let Some(w) = s.mate[v] {
                if v < w && vs.contains(&w) {
                    count += 1;
                }
            }
        }
        assert_eq!(count, vs.len() / 2);
        for &(u, v) in &n.edges {
            let mut sum = s.y[u] + s.y[v];
            let mut a = Some(s.leaf[u]);
            let mut aa = vec![];
            while let Some(x) = a {
                aa.push(x);
                a = s.nodes[x].parent;
            }
            let mut a = Some(s.leaf[v]);
            while let Some(x) = a {
                if aa.contains(&x) {
                    sum += s.nodes[x].z;
                }
                a = s.nodes[x].parent;
            }
            let slack = sum - g.edges[g.edge_id(u, v)].w;
            assert!(slack <= 0 && slack >= bound, "blossom edge slack {}", slack);
        }
    }
}
