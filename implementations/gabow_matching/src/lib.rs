//! Maximum cardinality matching in O(sqrt(n) m) following
//! H. N. Gabow, "The Weighted Matching Approach to Maximum Cardinality
//! Matching", Fundamenta Informaticae 154 (2017) 109-130 (arXiv:1703.03998).
//!
//! Each iteration (Fig. 1 of the paper):
//!   Phase 1  one Edmonds search with w(matched)=2, w(unmatched)=0, all y=1;
//!            contract positive blossoms, keep tight edges  ->  graph H
//!   Phase 2  maximal set of vertex-disjoint augmenting paths in H
//!            (Gabow-Tarjan path-preserving DFS), lift to G, augment.

pub mod blossom;
pub mod graph;
pub mod phase1;
pub mod phase2;

pub use graph::{Graph, Matching};

#[derive(Clone, Debug)]
pub struct PhaseStats {
    pub h_vertices: usize,
    pub h_edges: usize,
    pub final_delta: i64,
    /// length (in edges of G) of every path augmented in this phase
    pub path_lengths: Vec<usize>,
}

pub fn maximum_matching(g: &Graph) -> (Matching, Vec<PhaseStats>) {
    let mut m = Matching::new(g.n);
    let mut stats = Vec::new();
    while let Some(h) = phase1::build_help_graph(g, &m) {
        let paths = phase2::find_ap_set(&h);
        assert!(
            !paths.is_empty(),
            "H has an augmenting path, so Phase 2 must find one"
        );
        let lifted: Vec<Vec<usize>> = paths
            .iter()
            .map(|p| phase2::lift_to_g(g, &m, &h, p))
            .collect();
        stats.push(PhaseStats {
            h_vertices: h.n,
            h_edges: h.edges.len(),
            final_delta: h.delta,
            path_lengths: lifted.iter().map(|p| p.len()).collect(),
        });
        for p in &lifted {
            m.augment(g, p);
        }
    }
    (m, stats)
}
