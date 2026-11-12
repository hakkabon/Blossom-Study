//! Phase 1: one search of Edmonds' weighted-matching algorithm (paper Fig. 2)
//! with `w(e) = 2` on matched edges and `0` otherwise, followed by the
//! construction of the graph `H` of permissible edges (Sec. 3.3 / Sec. 5).
//!
//! Dual bookkeeping uses the paper's counter `Delta` (sum of all dual
//! adjustments so far): an outer vertex has `y = off - Delta`, an inner one
//! `y = off + Delta`, a vertex outside the search structure `S` has `y = 1`.
//! The priority queue is the paper's array of lists `L(d)` (buckets by Delta).

use crate::blossom::{BlossomForest, Contraction, Link, NONE};
use crate::graph::{Graph, Matching};

/// Edge of `H`; `pre` is its preimage edge id in `G`.
#[derive(Clone, Copy, Debug)]
pub struct HEdge {
    pub a: usize,
    pub b: usize,
    pub pre: usize,
}

/// The graph `H` of permissible edges: `G` with every positive blossom
/// contracted, keeping only tight edges that join distinct `H`-vertices.
/// Augmenting paths of `H` correspond 1-to-1 to shortest augmenting paths of `G`.
///
/// `H` is an *undirected* graph; alternation is defined by the matching
/// `mate/medge` induced on it (matched edges of `G` between distinct vertices).
pub struct HGraph {
    pub n: usize,
    pub edges: Vec<HEdge>,
    /// `adj[h] = [(neighbour, H-edge id)]`
    pub adj: Vec<Vec<(usize, usize)>>,
    /// matched partner in `H` (or `NONE`) and the matched `H`-edge id
    pub mate: Vec<usize>,
    pub medge: Vec<usize>,
    /// `label[v]`: the `H`-vertex containing `G`-vertex `v`
    pub label: Vec<usize>,
    /// `top[h]`: blossom-forest node of `h` (a leaf = plain vertex of `G`)
    pub top: Vec<usize>,
    /// Phase-1 blossoms; used to expand `H`-vertices back into `G`-paths
    pub forest: BlossomForest,
    /// final value of Delta (= `-2*y(f) + 2` up to the paper's conventions)
    pub delta: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum St {
    Unreached,
    Inner,
    Outer,
}

struct Search<'a> {
    g: &'a Graph,
    m: &'a Matching,
    delta: i64,
    st: Vec<St>,
    off: Vec<i64>,
    root: Vec<usize>,
    /// for an inner vertex u: the outer vertex that grew it, and that edge
    inner_parent: Vec<usize>,
    inner_eid: Vec<usize>,
    c: Contraction,
    buckets: Vec<Vec<usize>>,
    ready: Vec<usize>,
    mark_a: Vec<u32>,
    mark_b: Vec<u32>,
    stamp: u32,
}

impl<'a> Search<'a> {
    fn new(g: &'a Graph, m: &'a Matching) -> Self {
        let n = g.n;
        Search {
            g,
            m,
            delta: 0,
            st: vec![St::Unreached; n],
            off: vec![0; n],
            root: vec![NONE; n],
            inner_parent: vec![NONE; n],
            inner_eid: vec![NONE; n],
            c: Contraction::new(n),
            buckets: Vec::new(),
            ready: Vec::new(),
            mark_a: vec![0; n],
            mark_b: vec![0; n],
            stamp: 0,
        }
    }

    fn y(&self, v: usize) -> i64 {
        match self.st[v] {
            St::Unreached => 1,
            St::Outer => self.off[v] - self.delta,
            St::Inner => self.off[v] + self.delta,
        }
    }
    fn make_outer(&mut self, v: usize) {
        let cur = self.y(v);
        self.st[v] = St::Outer;
        self.off[v] = cur + self.delta;
    }
    fn make_inner(&mut self, v: usize) {
        let cur = self.y(v);
        self.st[v] = St::Inner;
        self.off[v] = cur - self.delta;
    }

    /// edge `e` becomes tight `d` units of Delta from now
    fn schedule(&mut self, e: usize, d: i64) {
        if d <= 0 {
            self.ready.push(e);
        } else {
            let t = (self.delta + d) as usize;
            if t >= self.buckets.len() {
                self.buckets.resize_with(t + 1, Vec::new);
            }
            self.buckets[t].push(e);
        }
    }

    /// `u` just became outer: scan its unmatched edges (paper Sec. 5, "Dual
    /// adjustment steps").  Edges to inner vertices are rescanned later, when
    /// that vertex becomes outer.  Stale entries are filtered when popped.
    fn scan(&mut self, u: usize) {
        let g = self.g;
        for &(v, e) in &g.adj[u] {
            if self.m.mate[u] == v {
                continue;
            }
            let d = match self.st[v] {
                St::Unreached => self.y(u) + 1, // y(v) == 1, y(u) shrinks by 1 per unit
                St::Outer => {
                    let s = self.y(u) + self.y(v);
                    debug_assert!(s.rem_euclid(2) == 0, "outer duals must share parity");
                    s / 2 // both shrink
                }
                St::Inner => continue,
            };
            self.schedule(e, d);
        }
    }

    /// Returns `true` iff an augmenting path exists (an augment step fired).
    fn run(&mut self) -> bool {
        let n = self.g.n;
        for f in 0..n {
            if self.m.mate[f] == NONE {
                self.make_outer(f); // y(f) = 1 - Delta
                self.root[f] = f;
            }
        }
        for f in 0..n {
            if self.m.mate[f] == NONE {
                self.scan(f);
            }
        }
        loop {
            while let Some(e) = self.ready.pop() {
                let (a, b) = self.g.edges[e];
                if self.y(a) + self.y(b) != 0 {
                    continue; // stale bucket entry
                }
                let (x, y) = if self.st[a] == St::Outer {
                    (a, b)
                } else if self.st[b] == St::Outer {
                    (b, a)
                } else {
                    continue;
                };
                if self.c.base_of(x) == self.c.base_of(y) {
                    continue; // B_x == B_y
                }
                match self.st[y] {
                    St::Unreached => self.grow(x, y, e),
                    St::Outer => {
                        if self.root[x] == self.root[y] {
                            self.blossom(x, y, e);
                        } else {
                            return true; // augment step: search ends
                        }
                    }
                    St::Inner => {}
                }
            }
            // dual adjustment step: jump Delta to the next non-empty bucket
            let mut d = self.delta as usize + 1;
            while d < self.buckets.len() && self.buckets[d].is_empty() {
                d += 1;
            }
            if d >= self.buckets.len() {
                return false; // delta = infinity: M has maximum cardinality
            }
            self.delta = d as i64;
            let batch = std::mem::take(&mut self.buckets[d]);
            self.ready.extend(batch);
        }
    }

    fn grow(&mut self, x: usize, y: usize, e: usize) {
        let y2 = self.m.mate[y]; // every unreached vertex is matched
        self.make_inner(y);
        self.inner_parent[y] = x;
        self.inner_eid[y] = e;
        self.root[y] = self.root[x];
        self.make_outer(y2);
        self.root[y2] = self.root[x];
        self.scan(y2);
    }

    /// parent of outer blossom (given by its base `b`) in the contracted forest
    fn parent_outer(&mut self, b: usize) -> Option<usize> {
        let u = self.m.mate[b];
        if u == NONE {
            return None; // b is a free root
        }
        debug_assert!(self.st[u] == St::Inner);
        let p = self.inner_parent[u];
        Some(self.c.base_of(p))
    }

    /// Blossom step for the tight edge `e = xy`, both ends outer in one tree.
    fn blossom(&mut self, x: usize, y: usize, e: usize) {
        let a0 = self.c.base_of(x);
        let b0 = self.c.base_of(y);
        self.stamp += 1;
        let s = self.stamp;
        let mut xs = vec![a0];
        let mut ys = vec![b0];
        self.mark_a[a0] = s;
        self.mark_b[b0] = s;
        let (mut ca, mut cb) = (Some(a0), Some(b0));
        // climb both paths in parallel: O(|cycle|)
        let nca = loop {
            if let Some(a) = ca {
                if self.mark_b[a] == s {
                    break a;
                }
            }
            if let Some(b) = cb {
                if self.mark_a[b] == s {
                    break b;
                }
            }
            if let Some(a) = ca {
                ca = self.parent_outer(a);
                if let Some(a2) = ca {
                    self.mark_a[a2] = s;
                    xs.push(a2);
                }
            }
            if let Some(b) = cb {
                cb = self.parent_outer(b);
                if let Some(b2) = cb {
                    self.mark_b[b2] = s;
                    ys.push(b2);
                }
            }
        };
        let ix = xs.iter().position(|&v| v == nca).unwrap();
        let iy = ys.iter().position(|&v| v == nca).unwrap();

        // Ring C_0 = nca, ..., Bx, (edge xy), By, ..., back to nca.
        let down: Vec<usize> = xs[..=ix].iter().rev().copied().collect();
        let mut nodes: Vec<usize> = vec![self.c.top_of(down[0])];
        let mut links: Vec<Link> = Vec::new();
        for w in down.windows(2) {
            let child_base = w[1];
            let u = self.m.mate[child_base]; // inner vertex between the two outer blossoms
            links.push(Link {
                eid: self.inner_eid[u],
                p: self.inner_parent[u],
                q: u,
            });
            nodes.push(u);
            links.push(Link {
                eid: self.m.medge[u],
                p: u,
                q: child_base,
            });
            nodes.push(self.c.top_of(child_base));
        }
        links.push(Link { eid: e, p: x, q: y });
        for &ob in &ys[..iy] {
            nodes.push(self.c.top_of(ob));
            let u = self.m.mate[ob];
            links.push(Link {
                eid: self.m.medge[ob],
                p: ob,
                q: u,
            });
            nodes.push(u);
            links.push(Link {
                eid: self.inner_eid[u],
                p: u,
                q: self.inner_parent[u],
            });
        }
        debug_assert_eq!(nodes.len(), links.len());
        debug_assert!(nodes.len() % 2 == 1);

        let n = self.g.n;
        let inners: Vec<usize> = nodes
            .iter()
            .copied()
            .filter(|&nd| nd < n && self.st[nd] == St::Inner)
            .collect();
        let cycle: Vec<(usize, Link)> = nodes.into_iter().zip(links).collect();
        self.c.contract(cycle, self.delta);
        for &u in &inners {
            self.make_outer(u);
        }
        for &u in &inners {
            self.scan(u);
        }
    }

    /// Build `H` (paper Sec. 3.3 and Sec. 5, "Constructing graph H").
    fn into_h(mut self) -> HGraph {
        let g = self.g;
        let n = g.n;
        let final_delta = self.delta;
        let yv: Vec<i64> = (0..n).map(|v| self.y(v)).collect();

        // (1) vertices of H: maximal blossoms that are positive, i.e. formed
        //     strictly before the last dual adjustment (stamp < final Delta).
        //     Blossoms stamped with the final Delta have z = 0: discard them
        //     and expose their children.
        let mut label = vec![NONE; n];
        let mut top: Vec<usize> = Vec::new();
        for v in 0..n {
            if self.c.dsu.find(v) != v {
                continue;
            }
            let t = self.c.top[v];
            let mut stack = vec![t];
            while let Some(nd) = stack.pop() {
                let f = &self.c.forest;
                if f.is_leaf(nd) || f.stamp(nd) < final_delta {
                    let h = top.len();
                    top.push(nd);
                    let mut st2 = vec![nd];
                    while let Some(z) = st2.pop() {
                        if f.is_leaf(z) {
                            label[z] = h;
                        } else {
                            st2.extend(f.cycle(z).iter().map(|&(c, _)| c));
                        }
                    }
                } else {
                    stack.extend(f.cycle(nd).iter().map(|&(c, _)| c));
                }
            }
        }
        let nh = top.len();

        // (2) edges of H: tight w.r.t. the final y, joining distinct H-vertices.
        //     (z values are never needed: no blossom contains both ends.)
        let mut edges = Vec::new();
        let mut adj = vec![Vec::new(); nh];
        let mut mate = vec![NONE; nh];
        let mut medge = vec![NONE; nh];
        for (eid, &(a, b)) in g.edges.iter().enumerate() {
            let (la, lb) = (label[a], label[b]);
            if la == lb {
                continue;
            }
            let matched = self.m.medge[a] == eid;
            let w = if matched { 2 } else { 0 };
            if yv[a] + yv[b] != w {
                continue;
            }
            let id = edges.len();
            edges.push(HEdge {
                a: la,
                b: lb,
                pre: eid,
            });
            adj[la].push((lb, id));
            adj[lb].push((la, id));
            if matched {
                mate[la] = lb;
                mate[lb] = la;
                medge[la] = id;
                medge[lb] = id;
            }
        }
        HGraph {
            n: nh,
            edges,
            adj,
            mate,
            medge,
            label,
            top,
            forest: self.c.forest,
            delta: final_delta,
        }
    }
}

/// Run Phase 1. Returns `None` iff `m` already has maximum cardinality.
pub fn build_help_graph(g: &Graph, m: &Matching) -> Option<HGraph> {
    let mut s = Search::new(g, m);
    if s.run() { Some(s.into_h()) } else { None }
}
