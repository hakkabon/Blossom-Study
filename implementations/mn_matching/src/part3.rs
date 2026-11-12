//! Part III (Sec. 3.2): maximal set of vertex-disjoint augmenting paths in `H`, using the
//! path-preserving depth-first search of Gabow-Tarjan (paper Fig. 6), then
//! lifting those paths from `H` back to `G`.

use crate::blossom::{Contraction, Link, NONE};
use crate::graph::Graph;
use crate::h::HGraph;

struct Dfs<'h> {
    h: &'h HGraph,
    in_s: Vec<bool>,
    outer: Vec<bool>,
    /// time at which a vertex became outer (only bases are ever compared)
    order: Vec<u32>,
    counter: u32,
    /// `ell[y']`: outer vertex that grew the inner mate of `y'`; `ell_e` = that edge
    ell: Vec<usize>,
    ell_e: Vec<usize>,
    c: Contraction,
    paths: Vec<Vec<usize>>,
    done: bool,
}

impl<'h> Dfs<'h> {
    fn new(h: &'h HGraph) -> Self {
        Dfs {
            h,
            in_s: vec![false; h.n],
            outer: vec![false; h.n],
            order: vec![0; h.n],
            counter: 0,
            ell: vec![NONE; h.n],
            ell_e: vec![NONE; h.n],
            c: Contraction::new(h.n),
            paths: Vec::new(),
            done: false,
        }
    }

    fn tick(&mut self) -> u32 {
        self.counter += 1;
        self.counter
    }

    /// Edges of `P(x)`: the alternating path in `H` from outer `x` to its root,
    /// traversing the blossoms built by this search with `P(v, b)`.
    fn p_of(&mut self, x: usize, out: &mut Vec<usize>) {
        let h = self.h;
        let mut cur = x;
        loop {
            let r = self.c.base_of(cur);
            let top = self.c.top_of(cur);
            self.c.forest.path_to_base(top, cur, out);
            if h.free[r] {
                break; // reached the free root
            }
            out.push(h.medge[r]);
            out.push(self.ell_e[r]);
            cur = self.ell[r];
        }
    }

    fn find_ap(&mut self, x: usize) {
        let h = self.h;
        for &(y, e) in &h.adj[x] {
            if h.medge[x] == e {
                continue; // scan only unmatched edges
            }
            if !self.in_s[y] {
                if h.free[y] {
                    // y is free: xy + P(x) is an augmenting path of H
                    self.in_s[y] = true;
                    let mut path = vec![e];
                    self.p_of(x, &mut path);
                    self.paths.push(path);
                    self.done = true; // terminate every active find_ap
                    return;
                }
                // grow step (a non-free node has its matched edge in H)
                let y2 = h.mate[y];
                debug_assert!(y2 != NONE);
                if y2 == NONE {
                    continue;
                }
                self.in_s[y] = true;
                self.in_s[y2] = true;
                self.outer[y2] = true;
                self.order[y2] = self.tick();
                self.ell[y2] = x;
                self.ell_e[y2] = e;
                self.find_ap(y2);
                if self.done {
                    return;
                }
            } else {
                let bx = self.c.base_of(x);
                let by = self.c.base_of(y);
                // "b(y) is an outer proper descendant of b(x)" <=> b(y) became
                // outer strictly after b(x)  (Appendix A)
                if self.outer[by] && self.order[by] > self.order[bx] {
                    self.blossom_step(x, y, e, bx);
                    if self.done {
                        return;
                    }
                }
            }
        }
    }

    fn blossom_step(&mut self, x: usize, y: usize, e: usize, bx: usize) {
        let h = self.h;
        // outer blossom bases ob_1 = b(y), ob_2, ... up to (excluding) b(x)
        let mut chain = Vec::new();
        let mut cur = self.c.base_of(y);
        while cur != bx {
            chain.push(cur);
            let p = self.ell[cur];
            debug_assert!(p != NONE, "b(x) must be an ancestor of b(y)");
            cur = self.c.base_of(p);
        }
        let mut nodes = vec![self.c.top_of(bx)];
        let mut links = vec![Link { eid: e, p: x, q: y }];
        let mut inners = Vec::new(); // u_1 (closest to y) .. u_k
        for &ob in &chain {
            let u = h.mate[ob];
            nodes.push(self.c.top_of(ob));
            links.push(Link {
                eid: h.medge[ob],
                p: ob,
                q: u,
            });
            nodes.push(u);
            links.push(Link {
                eid: self.ell_e[ob],
                p: u,
                q: self.ell[ob],
            });
            inners.push(u);
        }
        let cycle: Vec<(usize, Link)> = nodes.into_iter().zip(links).collect();
        self.c.contract(cycle, 0);
        for &u in &inners {
            self.outer[u] = true;
        }
        // explore the new outer vertices deepest-first (decreasing |P(u_i)|)
        for &u in &inners {
            self.find_ap(u);
            if self.done {
                return;
            }
        }
    }
}

fn run(h: &HGraph) -> Vec<Vec<usize>> {
    let mut d = Dfs::new(h);
    for f in 0..h.n {
        if h.free[f] && !d.in_s[f] {
            d.in_s[f] = true;
            d.outer[f] = true;
            d.order[f] = d.tick();
            d.find_ap(f);
            d.done = false;
        }
    }
    d.paths
}

/// Maximal set of vertex-disjoint augmenting paths of `H`, each as a set of `H`-edge ids.
/// (Recursion depth is O(|V(H)|), so the search runs on a thread with a large stack.)
pub fn find_ap_set(h: &HGraph) -> Vec<Vec<usize>> {
    let stack = (h.n * 400).max(16 << 20);
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(s, || run(h))
            .expect("spawn")
            .join()
            .expect("find_ap panicked")
    })
}

/// Lift an augmenting path of `H` to the edge set of the corresponding
/// shortest augmenting path of `G`: map every `H`-edge to its preimage and,
/// at each contracted blossom the path touches, splice in `P(v, b)` where `v`
/// is the end of the path's unmatched edge inside the blossom.
pub fn lift_to_g(g: &Graph, m: &crate::graph::Matching, h: &HGraph, path: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    for &he in path {
        let pre = h.edges[he].pre;
        out.push(pre);
        let (a, b) = g.edges[pre];
        if m.medge[a] == pre {
            continue; // matched edge: its blossom end is the base, nothing to splice
        }
        for v in [a, b] {
            let t = h.top[h.label[v]];
            h.forest.path_to_base(t, v, &mut out);
        }
    }
    out
}
