//! Liquidationist, Figure 5 (§3): the actual scaling driver, not repeated
//! calls to an exact black-box matcher. All inherited state goes through
//! liquidation, reweighting, height reduction, SearchOne, and perfection.
use crate::{
    Error, Options, Stats,
    blossom::State,
    dual,
    graph::{Edge, Graph},
    liquidation, pq_search,
    search::Criterion,
    search_one,
};
pub(crate) fn solve(
    n: usize,
    edges: &[Edge],
    options: &Options,
) -> Result<(Vec<Option<usize>>, Stats), Error> {
    if n == 0 {
        return Ok((vec![], Stats::default()));
    }
    if n % 2 != 0 {
        return Err(Error::NoPerfectMatching);
    }
    let mut g = Graph::new(n, edges);
    let mut s = State::new(n);
    let mut stats = Stats::default();
    let maxbar = g.edges.iter().map(|e| e.bar).max().unwrap_or(0);
    let bits = if maxbar == 0 {
        1
    } else {
        128 - maxbar.leading_zeros()
    };
    let tau = options.tau.unwrap_or_else(|| {
        let mut t = 1;
        while t * t < n {
            t += 1;
        }
        t
    });
    if tau == 0 {
        return Err(Error::InvalidTau);
    }
    for bit in (0..bits).rev() {
        liquidation::inherit(
            &mut g,
            &mut s,
            bit,
            tau,
            options.check_invariants,
            &mut stats,
        )?;
        for _ in 0..tau {
            let roots = free(&g, &s);
            search_one::run(&g, &mut s, &roots, options.check_invariants, &mut stats);
        }
        // Figure 5 step 8: remove FREE old pendants; matched old pendants persist.
        for v in n..g.alive.len() {
            if g.alive[v] && s.mate[v].is_none() {
                g.alive[v] = false;
            }
        }
        let roots = free(&g, &s);
        for u in roots {
            assert!(u < n);
            let v = g.add_dummy();
            s.add_vertex(tau as i128);
            // A newly created zero-weight pendant has no exposed weight bits.
            // Its potential cancels its real endpoint's accumulated reweighting.
            g.potential[v] = -g.potential[u];
            g.add_edge(u, v, 0, 0);
            s.mate[u] = Some(v);
            s.mate[v] = Some(u);
            stats.dummies_created += 1;
        }
        if options.check_invariants {
            dual::verify(&g, &s, true);
        }
        stats.scales += 1;
    }
    // Remove ALL pendants, then repair under Criterion 3, never Criterion 2.
    for v in n..g.alive.len() {
        if let Some(u) = s.mate[v].take() {
            s.mate[u] = None;
        }
        g.alive[v] = false;
    }
    loop {
        let mut roots = free(&g, &s);
        if roots.is_empty() {
            break;
        }
        // Old retained pendants can expose different dual parities on deletion.
        // EdmondsSearch permits any root subset F, but requires equal parity.
        // Search one parity class; a path may end at ANY other free vertex.
        let parity = s.y[roots[0]].rem_euclid(2);
        roots.retain(|&v| s.y[v].rem_euclid(2) == parity);
        pq_search::run(
            &g,
            &mut s,
            &roots,
            Criterion::Final,
            None,
            options.check_invariants,
            &mut stats,
        )?;
    }
    if options.check_invariants {
        dual::verify(&g, &s, true);
        for e in &g.edges {
            if e.u < n && e.v < n {
                assert_eq!(e.w, 2 * e.bar - g.potential[e.u] - g.potential[e.v]);
            }
        }
    }
    stats.contractions = s.contractions;
    stats.dissolutions = s.dissolutions;
    stats.nested_contractions = s.nested_contractions;
    s.mate.truncate(n);
    Ok((s.mate, stats))
}
pub(crate) fn free(g: &Graph, s: &State) -> Vec<usize> {
    (0..g.alive.len())
        .filter(|&v| g.alive[v] && s.mate[v].is_none())
        .collect()
}
