//! Maximum cardinality matching in O(sqrt(n) m): the variant of Gabow's
//! algorithm by K. Mehlhorn and R. Nobahari, "Gabow's O(sqrt(n) m) Maximum
//! Cardinality Matching Algorithm, Revisited", arXiv:2603.22909 (2026).
//!
//! Every iteration has three parts:
//!   I   [`part1`]  find a shortest augmenting path with lcp-labelled search
//!                  structures (no dual variables) -- the new contribution
//!   II  [`h`]      build the auxiliary graph H of maximal blossoms
//!   III [`part3`]  maximal set of vertex-disjoint augmenting paths in H
//!                  (Gabow-Tarjan DFS), lift to G, augment

pub mod blossom;
pub mod graph;
pub mod h;
pub mod part1;
pub mod part3;

pub use graph::{Graph, Matching};

#[derive(Clone, Debug)]
pub struct IterStats {
    /// breakthrough phase `Delta`; every augmenting path found has length `2*Delta-1`
    pub breakthrough_phase: usize,
    pub h_nodes: usize,
    pub h_edges: usize,
    pub path_lengths: Vec<usize>,
}

pub fn maximum_matching(g: &Graph) -> (Matching, Vec<IterStats>) {
    let mut m = Matching::new(g.n);
    let mut stats = Vec::new();
    loop {
        let mut s = part1::Search::new(g, &m);
        let Some(delta) = s.run() else { break };
        let h = h::construct(s, delta);
        let paths = part3::find_ap_set(&h);
        assert!(
            !paths.is_empty(),
            "H contains an augmenting path, Part III must find one"
        );
        let lifted: Vec<Vec<usize>> = paths
            .iter()
            .map(|p| part3::lift_to_g(g, &m, &h, p))
            .collect();
        stats.push(IterStats {
            breakthrough_phase: delta,
            h_nodes: h.n,
            h_edges: h.edges.len(),
            path_lengths: lifted.iter().map(|p| p.len()).collect(),
        });
        for p in &lifted {
            m.augment(g, p);
        }
    }
    (m, stats)
}

/// Optimality certificate of Sec. 2.3 for a maximum matching `m`
/// (returns `None` if `m` is not maximum).
pub fn certificate(g: &Graph, m: &Matching) -> Option<Vec<u32>> {
    let mut s = part1::Search::new(g, m);
    if s.run().is_some() {
        return None;
    }
    Some(s.certificate())
}
