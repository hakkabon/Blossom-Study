//! SearchOne(F), paper Figure 3; Figure 5 step 7. Criterion 2 is essential:
//! a newly matched external edge has slack -2 and becomes ineligible, preventing
//! eligible augmenting paths from reusing that contracted vertex (Lemma 2.3).
use crate::{
    Stats,
    blossom::State,
    dual,
    graph::Graph,
    search::{self, Criterion},
};
pub(crate) fn run(g: &Graph, s: &mut State, roots: &[usize], checks: bool, stats: &mut Stats) {
    // Greedy exhaustive searches realize a maximal disjoint augmenting set.
    // Unlike the paper's linear-time DFS/union-find implementation, each search
    // rebuilds its complete forest. No claim of O(m) SearchOne is made.
    let forest = loop {
        let (f, aug) = search::search(g, s, roots, Criterion::Reduction);
        stats.forest_searches += 1;
        if !aug {
            break f;
        }
        stats.augmentations += 1;
        s.dissolve_zero_roots();
    };
    dual::adjust(g, s, &forest, 1);
    stats.dual_events += 1;
    stats.dual_units += 1;
    if checks {
        dual::verify(g, s, true);
    }
}
