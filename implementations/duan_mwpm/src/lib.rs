//! Integer-weight matching in general undirected graphs.
//!
//! Implements the Liquidationist scaling framework of Duan, Pettie and Su,
//! *Scaling Algorithms for Weighted Matching in General Graphs*, §3, Figure 5.
//! The forest / event routines are deliberately simpler than the paper's §5;
//! the published O(m sqrt(n) log(nN)) bound does NOT apply to this crate.
mod blossom;
mod dual;
mod graph;
mod liquidation;
mod pq_search;
mod scaling;
mod search;
mod search_one;
pub use graph::Edge;
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug)]
pub struct Options {
    /// Large blossoms have at least tau vertices. Default: ceil(sqrt(n)) on
    /// the perfect-matching working graph (2n for the unconstrained API).
    pub tau: Option<usize>,
    /// Expensive edge, dual, blossom fullness, and matching checks. Default true.
    pub check_invariants: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            tau: None,
            check_invariants: true,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub scales: usize,
    pub forest_searches: usize,
    pub augmentations: usize,
    pub dual_events: usize,
    pub dual_units: i128,
    pub dummies_created: usize,
    pub contractions: usize,
    pub dissolutions: usize,
    pub nested_contractions: usize,
    pub large_liquidations: usize,
    pub small_liquidations: usize,
}
#[derive(Clone, Debug)]
pub struct Matching {
    pub mate: Vec<Option<usize>>,
    pub total_weight: i128,
    pub stats: Stats,
}
impl Matching {
    pub fn pairs(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.mate
            .iter()
            .enumerate()
            .filter_map(|(u, &v)| v.filter(|&v| u < v).map(|v| (u, v)))
    }
    pub fn cardinality(&self) -> usize {
        self.pairs().count()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidVertex { vertex: usize, n: usize },
    SelfLoop { vertex: usize },
    NoPerfectMatching,
    InvalidTau,
    GraphTooLarge,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVertex { vertex, n } => write!(f, "vertex {} outside 0..{}", vertex, n),
            Self::SelfLoop { vertex } => {
                write!(f, "self-loop at vertex {} is not supported", vertex)
            }
            Self::NoPerfectMatching => write!(f, "graph has no perfect matching"),
            Self::InvalidTau => write!(f, "tau must be positive"),
            Self::GraphTooLarge => write!(f, "graph exceeds supported vertex count"),
        }
    }
}
impl std::error::Error for Error {}
fn canonical(n: usize, edges: &[Edge], o: &Options) -> Result<Vec<Edge>, Error> {
    if n > u32::MAX as usize / 2 {
        return Err(Error::GraphTooLarge);
    }
    if o.tau == Some(0) {
        return Err(Error::InvalidTau);
    }
    let mut map = BTreeMap::new();
    for e in edges {
        for v in [e.u, e.v] {
            if v >= n {
                return Err(Error::InvalidVertex { vertex: v, n });
            }
        }
        if e.u == e.v {
            return Err(Error::SelfLoop { vertex: e.u });
        }
        let key = (e.u.min(e.v), e.u.max(e.v));
        map.entry(key)
            .and_modify(|w: &mut i64| *w = (*w).max(e.weight))
            .or_insert(e.weight);
    }
    Ok(map
        .into_iter()
        .map(|((u, v), weight)| Edge { u, v, weight })
        .collect())
}
fn result(mate: Vec<Option<usize>>, edges: &[Edge], stats: Stats) -> Matching {
    let total_weight = edges
        .iter()
        .filter(|e| mate[e.u] == Some(e.v))
        .map(|e| e.weight as i128)
        .sum();
    Matching {
        mate,
        total_weight,
        stats,
    }
}
/// Maximum weight, with NO cardinality requirement. Vertices are 0-based.
/// Negative edges may be left unused; the empty matching is allowed.
/// Parallel edges are collapsed to the heaviest; self-loops are rejected.
pub fn maximum_weight_matching(n: usize, edges: &[Edge]) -> Result<Matching, Error> {
    maximum_weight_matching_with_options(n, edges, &Options::default())
}
pub fn maximum_weight_matching_with_options(
    n: usize,
    edges: &[Edge],
    o: &Options,
) -> Result<Matching, Error> {
    let edges = canonical(n, edges, o)?;
    // One-time MWM -> MWPM reduction: two copies of G and zero-weight edges
    // v--v'. This is DISTINCT from Figure 5's per-scale pendant dummies.
    // Lift any matching twice to a perfect matching; every reduced perfect
    // matching projects to two matchings, each of weight <= OPT. Thus the
    // reduced optimum is exactly 2*OPT and either projection is optimal.
    let mut reduced = Vec::with_capacity(2 * edges.len() + n);
    for e in &edges {
        reduced.push(*e);
        reduced.push(Edge::new(e.u + n, e.v + n, e.weight));
    }
    for v in 0..n {
        reduced.push(Edge::new(v, v + n, 0));
    }
    let (mates, stats) = scaling::solve(2 * n, &reduced, o)?;
    let mate = (0..n).map(|v| mates[v].filter(|&w| w < n)).collect();
    Ok(result(mate, &edges, stats))
}
/// Maximum weight among PERFECT matchings; negative edges are retained.
/// Returns NoPerfectMatching when the graph is infeasible.
pub fn maximum_weight_perfect_matching(n: usize, edges: &[Edge]) -> Result<Matching, Error> {
    maximum_weight_perfect_matching_with_options(n, edges, &Options::default())
}
pub fn maximum_weight_perfect_matching_with_options(
    n: usize,
    edges: &[Edge],
    o: &Options,
) -> Result<Matching, Error> {
    let edges = canonical(n, edges, o)?;
    let (mate, stats) = scaling::solve(n, &edges, o)?;
    Ok(result(mate, &edges, stats))
}

#[cfg(test)]
mod tests;
