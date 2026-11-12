//! Plain undirected simple graph + matching container.

use crate::blossom::NONE;

/// Simple undirected graph. Self-loops and parallel edges are dropped, so an
/// edge is identified by its id *and* by its endpoints.
#[derive(Clone, Debug)]
pub struct Graph {
    pub n: usize,
    pub edges: Vec<(usize, usize)>,
    /// `adj[v] = [(neighbour, edge id), ...]`
    pub adj: Vec<Vec<(usize, usize)>>,
}

impl Graph {
    pub fn new(n: usize, raw: &[(usize, usize)]) -> Self {
        let mut es: Vec<(usize, usize)> = raw
            .iter()
            .filter(|&&(u, v)| u != v)
            .map(|&(u, v)| if u < v { (u, v) } else { (v, u) })
            .collect();
        es.sort_unstable();
        es.dedup();
        let mut adj = vec![Vec::new(); n];
        for (id, &(u, v)) in es.iter().enumerate() {
            assert!(u < n && v < n, "vertex out of range");
            adj[u].push((v, id));
            adj[v].push((u, id));
        }
        Graph { n, edges: es, adj }
    }
}

/// A matching: `mate[v]` is the partner of `v` (or `NONE`), `medge[v]` the id
/// of the matched edge at `v` (or `NONE`).
#[derive(Clone, Debug)]
pub struct Matching {
    pub mate: Vec<usize>,
    pub medge: Vec<usize>,
}

impl Matching {
    pub fn new(n: usize) -> Self {
        Matching {
            mate: vec![NONE; n],
            medge: vec![NONE; n],
        }
    }

    pub fn size(&self) -> usize {
        self.mate.iter().filter(|&&m| m != NONE).count() / 2
    }

    /// `M <- M xor P`, where `P` is an edge set forming an augmenting path
    /// (order irrelevant, as the paper notes in Sec. 3.1).
    pub fn augment(&mut self, g: &Graph, path_edges: &[usize]) {
        let mut unmatch = Vec::new();
        let mut matchit = Vec::new();
        for &e in path_edges {
            let (u, v) = g.edges[e];
            if self.medge[u] == e {
                unmatch.push((u, v));
            } else {
                matchit.push((u, v, e));
            }
        }
        for (u, v) in unmatch {
            self.mate[u] = NONE;
            self.mate[v] = NONE;
            self.medge[u] = NONE;
            self.medge[v] = NONE;
        }
        for (u, v, e) in matchit {
            self.mate[u] = v;
            self.mate[v] = u;
            self.medge[u] = e;
            self.medge[v] = e;
        }
    }

    /// Validity check used by tests.
    pub fn is_valid(&self, g: &Graph) -> bool {
        for v in 0..g.n {
            let m = self.mate[v];
            if m == NONE {
                continue;
            }
            if self.mate[m] != v {
                return false;
            }
            let e = self.medge[v];
            if e == NONE || self.medge[m] != e {
                return false;
            }
            let (a, b) = g.edges[e];
            if !((a == v && b == m) || (a == m && b == v)) {
                return false;
            }
        }
        true
    }
}
