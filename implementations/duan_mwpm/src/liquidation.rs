//! Liquidate(B), paper §3, and Figure 5 steps 2--6.
use crate::{Error, Stats, blossom::State, dual, graph::Graph, pq_search, search::Criterion};
pub(crate) fn inherit(
    g: &mut Graph,
    s: &mut State,
    bit: u32,
    tau: usize,
    checks: bool,
    stats: &mut Stats,
) -> Result<(), Error> {
    for e in &mut g.edges {
        if g.alive[e.u] && g.alive[e.v] {
            e.w = 2 * (e.w + ((e.bar >> bit) & 1));
        }
    }
    for v in 0..s.y.len() {
        if g.alive[v] {
            s.y[v] = 2 * s.y[v] + 3;
            g.potential[v] *= 2;
        }
    }
    // Snapshot old blossoms before resetting the matching / active family.
    let mut old = vec![];
    let mut small = vec![false; s.nodes.len()];
    for (b, n) in s.nodes.iter().enumerate() {
        if n.active && !n.children.is_empty() {
            let vs = s.leaves(b);
            small[b] = vs.len() < tau;
            old.push((b, vs, 2 * n.z));
        }
    }
    let groups: Vec<Vec<usize>> = old
        .iter()
        .filter(|(b, _, _)| small[*b] && s.nodes[*b].parent.map_or(true, |p| !small[p]))
        .map(|(_, vs, _)| vs.clone())
        .collect();
    // Large liquidation, then reweight, then small liquidation. Liquidating a
    // blossom adds z/2 to EACH leaf, not just to its base.
    for (b, vs, z) in &old {
        if !small[*b] {
            stats.large_liquidations += 1;
            for &v in vs {
                s.y[v] += z / 2;
            }
        }
    }
    for e in &mut g.edges {
        if g.alive[e.u] && g.alive[e.v] {
            e.w -= s.y[e.u] + s.y[e.v];
        }
    }
    for v in 0..s.y.len() {
        if g.alive[v] {
            g.potential[v] += s.y[v];
            s.y[v] = 0;
        }
    }
    for (b, vs, z) in &old {
        if small[*b] {
            stats.small_liquidations += 1;
            for &v in vs {
                s.y[v] += z / 2;
            }
        }
    }
    let y = std::mem::take(&mut s.y);
    let contractions = s.contractions;
    let dissolutions = s.dissolutions;
    let nested = s.nested_contractions;
    *s = State::new(g.alive.len());
    s.y = y;
    s.contractions = contractions;
    s.dissolutions = dissolutions;
    s.nested_contractions = nested;
    if checks {
        dual::verify(g, s, false);
    }
    // Maximal old SMALL blossoms may be children of a large old blossom.
    for group in groups {
        loop {
            let height = group
                .iter()
                .filter(|&&v| s.mate[v].is_none())
                .map(|&v| s.y[v])
                .max()
                .unwrap_or(0);
            if height <= 0 {
                break;
            }
            let roots: Vec<_> = group
                .iter()
                .copied()
                .filter(|&v| s.mate[v].is_none() && s.y[v] == height)
                .collect();
            let next = group
                .iter()
                .filter(|&&v| s.mate[v].is_none() && s.y[v] < height)
                .map(|&v| s.y[v])
                .max()
                .unwrap_or(0)
                .max(0);
            pq_search::run(
                g,
                s,
                &roots,
                Criterion::Tight,
                Some(height - next),
                checks,
                stats,
            )?;
        }
    }
    if checks {
        for v in 0..s.y.len() {
            if g.alive[v] && s.mate[v].is_none() {
                assert_eq!(s.y[v], 0);
            }
        }
    }
    Ok(())
}
