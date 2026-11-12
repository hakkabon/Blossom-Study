//! Event-jumping replacement for PQSearch (§5, Figure 5 step 6 / finalization).
//! Events are found by scanning all edges; this is a reference implementation,
//! NOT the split-findmin/priority-queue implementation analyzed in the paper.
use crate::{
    Error, Stats,
    blossom::State,
    dual,
    graph::Graph,
    search::{self, Criterion},
};
pub(crate) fn run(
    g: &Graph,
    s: &mut State,
    roots: &[usize],
    c: Criterion,
    budget: Option<i128>,
    checks: bool,
    stats: &mut Stats,
) -> Result<bool, Error> {
    if checks && !roots.is_empty() {
        assert!(
            roots.iter().all(|&v| (s.y[v] - s.y[roots[0]]) % 2 == 0),
            "mixed root parity: {:?}",
            roots.iter().map(|&v| (v, s.y[v])).collect::<Vec<_>>()
        );
    }
    let mut spent = 0;
    loop {
        let (f, aug) = search::search(g, s, roots, c);
        stats.forest_searches += 1;
        if aug {
            stats.augmentations += 1;
            s.dissolve_zero_roots();
            if checks {
                dual::verify(g, s, c == Criterion::Final);
            }
            return Ok(true);
        }
        if budget == Some(spent) {
            s.dissolve_zero_roots();
            return Ok(false);
        }
        let event = dual::next_event(g, s, &f, c);
        let delta = match (event, budget) {
            (Some(d), Some(b)) => d.min(b - spent),
            (Some(d), None) => d,
            (None, Some(b)) => b - spent,
            (None, None) => return Err(Error::NoPerfectMatching),
        };
        dual::adjust(g, s, &f, delta);
        stats.dual_events += 1;
        stats.dual_units += delta;
        spent += delta;
        if checks {
            dual::verify(g, s, c == Criterion::Final);
        }
        // A budget-limited call must stop at its boundary, even if an edge has
        // just become eligible. The next root-height iteration handles it.
        if budget == Some(spent) {
            return Ok(false);
        }
    }
}
