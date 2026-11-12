//! Part I (Mehlhorn & Nobahari, Sec. 2): find a shortest augmenting path by
//! growing alternating search structures breadth-first, with **no dual
//! variables**.  Every vertex carries only its canonical-path length:
//!
//! * `lcp(v)`     for even vertices (shortest even alternating path from a free vertex),
//! * `lcp_odd(v)` for vertices born odd.
//!
//! Phases `Delta = 0, 1, 2, ...`:
//! * even `Delta`: *growth steps* out of even vertices with `lcp = Delta-2`;
//! * every `Delta`: *bridge steps* on non-matching even-even edges `uv` with
//!   `lcp(u)+lcp(v) = 2*Delta-2` joining different maximal blossoms.
//!   Different trees => augmenting path of length `2*Delta-1`, Part I ends.
//!   Same tree     => blossom; each odd node `z` on the cycle becomes even with
//!   `lcp(z) = lcp(x)+1+lcp(y)-lcp_odd(z)`.
//!
//! The priority queue is an array of buckets indexed by phase (Sec. 2.3):
//! bucket `Delta` holds the even vertices with `lcp = Delta-2` and the
//! even-even edges with lcp-sum `2*Delta-2`.

use crate::blossom::{Contraction, Link, NONE};
use crate::graph::{Graph, Matching};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lab {
    Unlabeled,
    Odd,
    Even,
}

pub struct Search<'a> {
    pub g: &'a Graph,
    pub m: &'a Matching,
    pub lab: Vec<Lab>,
    /// valid for even vertices
    pub lcp: Vec<i64>,
    /// valid for vertices that were born odd (kept after they turn even)
    pub lcp_odd: Vec<i64>,
    pub root: Vec<usize>,
    /// tree parent (an even vertex) and the growth edge of a vertex born odd
    par: Vec<usize>,
    par_eid: Vec<usize>,
    pub c: Contraction,
    grow_q: Vec<Vec<usize>>,
    bridge_q: Vec<Vec<usize>>,
    max_phase: usize,
    /// current phase
    pub delta: usize,
    /// vertices turned even by blossom steps in the current phase
    pub phase_log: Vec<usize>,
    mark_a: Vec<u32>,
    mark_b: Vec<u32>,
    stamp: u32,
}

impl<'a> Search<'a> {
    pub fn new(g: &'a Graph, m: &'a Matching) -> Self {
        let n = g.n;
        Search {
            g,
            m,
            lab: vec![Lab::Unlabeled; n],
            lcp: vec![-1; n],
            lcp_odd: vec![-1; n],
            root: vec![NONE; n],
            par: vec![NONE; n],
            par_eid: vec![NONE; n],
            c: Contraction::new(n),
            grow_q: Vec::new(),
            bridge_q: Vec::new(),
            max_phase: 0,
            delta: 0,
            phase_log: Vec::new(),
            mark_a: vec![0; n],
            mark_b: vec![0; n],
            stamp: 0,
        }
    }

    fn push_grow(&mut self, phase: usize, v: usize) {
        if phase >= self.grow_q.len() {
            self.grow_q.resize_with(phase + 1, Vec::new);
        }
        self.grow_q[phase].push(v);
        self.max_phase = self.max_phase.max(phase);
    }

    fn push_bridge(&mut self, phase: usize, e: usize) {
        // Lemmas 3, 4 and 6: events never go to an earlier phase
        debug_assert!(phase >= self.delta, "event for an earlier phase");
        let phase = phase.max(self.delta);
        if phase >= self.bridge_q.len() {
            self.bridge_q.resize_with(phase + 1, Vec::new);
        }
        self.bridge_q[phase].push(e);
        self.max_phase = self.max_phase.max(phase);
    }

    /// `u` has just become even (its `lcp` is set): schedule growth out of it
    /// and every bridge to an already even neighbour.
    fn activate(&mut self, u: usize) {
        let g = self.g;
        self.push_grow((self.lcp[u] + 2) as usize, u);
        for &(v, e) in &g.adj[u] {
            if self.m.mate[u] == v || self.lab[v] != Lab::Even {
                continue;
            }
            let sum = self.lcp[u] + self.lcp[v];
            debug_assert!(sum % 2 == 0, "lcp values of even vertices are even");
            self.push_bridge((sum / 2 + 1) as usize, e);
        }
    }

    /// Runs the phases.  `Some(Delta)`: breakthrough in phase `Delta` (a sap of
    /// length `2*Delta-1` exists).  `None`: `M` has maximum cardinality; the
    /// search structure is then complete (every even-even edge was processed).
    pub fn run(&mut self) -> Option<usize> {
        let n = self.g.n;
        // phase 0: free vertices are the roots, lcp = 0
        for f in 0..n {
            if self.m.mate[f] == NONE {
                self.lab[f] = Lab::Even;
                self.lcp[f] = 0;
                self.root[f] = f;
            }
        }
        for f in 0..n {
            if self.m.mate[f] == NONE {
                self.activate(f);
            }
        }
        let mut d = 1;
        while d <= self.max_phase {
            self.delta = d;
            self.phase_log.clear();
            if d % 2 == 0 && d < self.grow_q.len() {
                let list = std::mem::take(&mut self.grow_q[d]);
                for v in list {
                    self.grow_from(v);
                }
            }
            while d < self.bridge_q.len() {
                let Some(e) = self.bridge_q[d].pop() else {
                    break;
                };
                let (a, b) = self.g.edges[e];
                if self.lab[a] != Lab::Even || self.lab[b] != Lab::Even {
                    continue;
                }
                if self.c.base_of(a) == self.c.base_of(b) {
                    continue; // same maximal blossom
                }
                debug_assert_eq!(self.lcp[a] + self.lcp[b], 2 * d as i64 - 2);
                if self.root[a] != self.root[b] {
                    return Some(d); // bridge between two trees: augmenting path
                }
                self.blossom(a, b, e);
            }
            d += 1;
        }
        None
    }

    /// Growth steps out of even vertex `v` (`lcp(v) = Delta-2`).
    fn grow_from(&mut self, v: usize) {
        let g = self.g;
        let d = self.delta as i64;
        for &(x, e) in &g.adj[v] {
            if self.m.mate[v] == x || self.lab[x] != Lab::Unlabeled {
                continue;
            }
            let x2 = self.m.mate[x]; // unlabeled vertices come in matched pairs
            self.lab[x] = Lab::Odd;
            self.lcp_odd[x] = d - 1;
            self.par[x] = v;
            self.par_eid[x] = e;
            self.root[x] = self.root[v];
            self.lab[x2] = Lab::Even;
            self.lcp[x2] = d;
            self.root[x2] = self.root[v];
            self.activate(x2);
        }
    }

    /// parent of the outer (even) blossom with base `b` in the contracted tree
    fn parent_even(&mut self, b: usize) -> Option<usize> {
        let u = self.m.mate[b];
        if u == NONE {
            return None;
        }
        debug_assert!(self.lab[u] == Lab::Odd);
        let p = self.par[u];
        Some(self.c.base_of(p))
    }

    /// Bridge `e = xy` inside one tree closes a blossom.
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
        // walk from both ends towards the roots in lock step (Sec. 4)
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
                ca = self.parent_even(a);
                if let Some(a2) = ca {
                    self.mark_a[a2] = s;
                    xs.push(a2);
                }
            }
            if let Some(b) = cb {
                cb = self.parent_even(b);
                if let Some(b2) = cb {
                    self.mark_b[b2] = s;
                    ys.push(b2);
                }
            }
        };
        let ix = xs.iter().position(|&v| v == nca).unwrap();
        let iy = ys.iter().position(|&v| v == nca).unwrap();

        // ring C_0 = B (lowest common ancestor), down to B_x, bridge, B_y, up to B
        let down: Vec<usize> = xs[..=ix].iter().rev().copied().collect();
        let mut nodes: Vec<usize> = vec![self.c.top_of(down[0])];
        let mut links: Vec<Link> = Vec::new();
        for w in down.windows(2) {
            let cb_ = w[1];
            let u = self.m.mate[cb_];
            links.push(Link {
                eid: self.par_eid[u],
                p: self.par[u],
                q: u,
            });
            nodes.push(u);
            links.push(Link {
                eid: self.m.medge[u],
                p: u,
                q: cb_,
            });
            nodes.push(self.c.top_of(cb_));
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
                eid: self.par_eid[u],
                p: u,
                q: self.par[u],
            });
        }
        debug_assert_eq!(nodes.len(), links.len());
        debug_assert!(nodes.len() % 2 == 1);

        let n = self.g.n;
        let odd: Vec<usize> = nodes
            .iter()
            .copied()
            .filter(|&nd| nd < n && self.lab[nd] == Lab::Odd)
            .collect();
        let cycle: Vec<(usize, Link)> = nodes.into_iter().zip(links).collect();
        // the blossom's stamp is the phase in which it is formed
        self.c.contract(cycle, self.delta as i64);

        // every odd node z of the cycle becomes even
        let (lx, ly) = (self.lcp[x], self.lcp[y]);
        for &z in &odd {
            self.lab[z] = Lab::Even;
            self.lcp[z] = lx + 1 + ly - self.lcp_odd[z];
            debug_assert!(self.lcp[z] >= self.delta as i64, "Lemma 4");
            self.phase_log.push(z);
        }
        for &z in &odd {
            self.activate(z);
        }
    }

    /// Certificate of optimality (Sec. 2.3).  Only valid after `run()` returned
    /// `None`.  Labels satisfy: every edge has an endpoint labelled 1 or two
    /// endpoints with the same label >= 2.  With `n_i` = number of vertices
    /// labelled `i`:  `|M| <= n_1 + sum_{i>=2} floor(n_i/2)`, equality for a
    /// maximum matching.
    pub fn certificate(&mut self) -> Vec<u32> {
        let n = self.g.n;
        let mut label = vec![0u32; n];
        let mut next = 2u32;
        let mut ids = std::collections::HashMap::new();
        let mut loose: Vec<(usize, usize)> = Vec::new();
        for v in 0..n {
            match self.lab[v] {
                Lab::Odd => label[v] = 1,
                Lab::Even => {
                    let r = self.c.dsu.find(v);
                    if self.c.forest.is_leaf(self.c.top[r]) {
                        label[v] = 0;
                    } else {
                        label[v] = *ids.entry(r).or_insert_with(|| {
                            next += 1;
                            next - 1
                        });
                    }
                }
                Lab::Unlabeled => {
                    let w = self.m.mate[v];
                    if v < w {
                        loose.push((v, w));
                    }
                }
            }
        }
        // Unreached matched pairs: one common label c >= 2 for all of them.  Every
        // edge touching them goes to another such vertex (same label) or to an odd
        // vertex (label 1), and the pairs contribute exactly |loose| to the bound.
        // (The paper's recipe -- 0/1 for a single pair, otherwise 1 and c per
        // pair -- is valid but leaves floor(k/2) slack in the counting bound.)
        if !loose.is_empty() {
            for &(a, b) in &loose {
                label[a] = next;
                label[b] = next;
            }
        }
        label
    }
}
