//! Mutable working graph. Paper §2: even working weights, integral y and even z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub weight: i64,
}
impl Edge {
    pub fn new(u: usize, v: usize, weight: i64) -> Self {
        Self { u, v, weight }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct WorkEdge {
    pub u: usize,
    pub v: usize,
    pub bar: i128,
    pub w: i128,
}
#[derive(Clone)]
pub(crate) struct Graph {
    pub edges: Vec<WorkEdge>,
    pub adj: Vec<Vec<usize>>,
    pub alive: Vec<bool>,
    pub potential: Vec<i128>,
}
impl Graph {
    pub fn new(n: usize, edges: &[Edge]) -> Self {
        let mut g = Self {
            edges: vec![],
            adj: vec![vec![]; n],
            alive: vec![true; n],
            potential: vec![0; n],
        };
        let min = edges
            .iter()
            .map(|e| e.weight as i128)
            .min()
            .unwrap_or(0)
            .min(0);
        let multiplier = (n / 2 + 1) as i128;
        for e in edges {
            g.add_edge(e.u, e.v, (e.weight as i128 - min) * multiplier, 0);
        }
        g
    }
    pub fn add_edge(&mut self, u: usize, v: usize, bar: i128, w: i128) {
        let k = self.edges.len();
        self.edges.push(WorkEdge { u, v, bar, w });
        self.adj[u].push(k);
        self.adj[v].push(k);
    }
    pub fn add_dummy(&mut self) -> usize {
        let v = self.alive.len();
        self.alive.push(true);
        self.adj.push(vec![]);
        self.potential.push(0);
        v
    }
    pub fn active_edge(&self, e: &WorkEdge) -> bool {
        self.alive[e.u] && self.alive[e.v]
    }
    pub fn edge_id(&self, u: usize, v: usize) -> usize {
        *self.adj[u]
            .iter()
            .find(|&&k| {
                let e = &self.edges[k];
                (e.u == u && e.v == v) || (e.u == v && e.v == u)
            })
            .expect("missing matched edge")
    }
}
