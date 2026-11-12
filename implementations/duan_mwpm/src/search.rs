//! Generic EdmondsSearch: paper §2.2, Figure 2. Deliberately uses full
//! forest rebuilds, not the advanced data structures of §5.
use crate::{blossom::State, graph::Graph};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Criterion {
    Tight,
    Reduction,
    Final,
}
pub(crate) struct Forest {
    pub label: Vec<u8>,
    pub via: Vec<Option<(usize, usize)>>,
    queue: Vec<usize>,
}
impl Forest {
    fn new(n: usize) -> Self {
        Self {
            label: vec![0; n],
            via: vec![None; n],
            queue: vec![],
        }
    }
    fn resize(&mut self, n: usize) {
        self.label.resize(n, 0);
        self.via.resize(n, None);
    }
    fn assign(&mut self, s: &mut State, w: usize, label: u8, from: Option<usize>) {
        let b = s.top[w];
        assert_eq!(self.label[b], 0);
        self.label[b] = label;
        self.via[b] = from.map(|v| (v, w));
        if label == 1 {
            self.queue.extend(s.leaves(b));
        }
    }
    fn scan(&mut self, s: &State, v: usize, w: usize) -> Option<usize> {
        let mut v = Some(v);
        let mut w = Some(w);
        let mut path = vec![];
        let mut base = None;
        while let Some(x) = v {
            let mut b = s.top[x];
            if self.label[b] & 4 != 0 {
                base = Some(s.nodes[b].base);
                break;
            }
            assert_eq!(self.label[b], 1);
            path.push(b);
            self.label[b] = 5;
            if let Some((p, _)) = self.via[b] {
                b = s.top[p];
                assert_eq!(self.label[b], 2);
                v = Some(self.via[b].unwrap().0);
            } else {
                assert!(s.mate[s.nodes[b].base].is_none());
                v = None;
            }
            if w.is_some() {
                std::mem::swap(&mut v, &mut w);
            }
        }
        for b in path {
            self.label[b] = 1;
        }
        base
    }
    fn shrink(&mut self, s: &mut State, base: usize, mut v: usize, mut w: usize) {
        let bb = s.top[base];
        let mut bv = s.top[v];
        let mut bw = s.top[w];
        let b = s.allocate(base);
        self.resize(s.nodes.len());
        s.nodes[bb].parent = Some(b);
        let mut path = vec![];
        let mut edges = vec![(v, w)];
        while bv != bb {
            s.nodes[bv].parent = Some(b);
            path.push(bv);
            edges.push(self.via[bv].unwrap());
            v = self.via[bv].unwrap().0;
            bv = s.top[v];
        }
        path.push(bb);
        path.reverse();
        edges.reverse();
        while bw != bb {
            s.nodes[bw].parent = Some(b);
            path.push(bw);
            let (x, y) = self.via[bw].unwrap();
            edges.push((y, x));
            w = x;
            bw = s.top[w];
        }
        assert_eq!(path.len() % 2, 1);
        assert_eq!(path.len(), edges.len());
        if path.iter().any(|&c| s.nontrivial(c)) {
            s.nested_contractions += 1;
        }
        s.nodes[b].children = path;
        s.nodes[b].edges = edges;
        self.label[b] = 1;
        self.via[b] = self.via[bb];
        for v in s.leaves(b) {
            if self.label[s.top[v]] == 2 {
                self.queue.push(v);
            }
            s.top[v] = b;
        }
        s.contractions += 1;
    }
    fn augment(&self, s: &mut State, v: usize, w: usize) {
        for (mut x, mut j) in [(v, w), (w, v)] {
            loop {
                let bx = s.top[x];
                assert_eq!(self.label[bx], 1);
                if s.nontrivial(bx) {
                    s.augment_blossom(bx, x);
                }
                s.mate[x] = Some(j);
                if let Some((t, _)) = self.via[bx] {
                    let bt = s.top[t];
                    assert_eq!(self.label[bt], 2);
                    let (a, b) = self.via[bt].unwrap();
                    x = a;
                    j = b;
                    if s.nontrivial(bt) {
                        s.augment_blossom(bt, j);
                    }
                    s.mate[j] = Some(x);
                } else {
                    break;
                }
            }
        }
    }
}
/// Only called on edges between *different* root blossoms. Shared z is zero.
pub(crate) fn eligible(g: &Graph, s: &State, k: usize, c: Criterion) -> bool {
    let e = &g.edges[k];
    let slack = s.y[e.u] + s.y[e.v] - e.w;
    match c {
        Criterion::Tight => slack == 0,
        Criterion::Reduction => {
            if s.mate[e.u] == Some(e.v) {
                slack == 0
            } else {
                slack == -2
            }
        }
        Criterion::Final => slack == 0 || slack == -2,
    }
}
/// Return (final forest, whether an augmentation was performed). A root set F
/// restricts at least one endpoint; the other endpoint may be any free vertex.
pub(crate) fn search(g: &Graph, s: &mut State, roots: &[usize], c: Criterion) -> (Forest, bool) {
    let mut f = Forest::new(s.nodes.len());
    for &v in roots {
        if g.alive[v] && s.mate[v].is_none() && f.label[s.top[v]] == 0 {
            f.assign(s, v, 1, None);
        }
    }
    while let Some(v) = f.queue.pop() {
        assert_eq!(f.label[s.top[v]], 1);
        for &k in &g.adj[v] {
            let e = &g.edges[k];
            if !g.active_edge(e) {
                continue;
            }
            let w = if e.u == v { e.v } else { e.u };
            let bv = s.top[v];
            let bw = s.top[w];
            if bv == bw || s.mate[v] == Some(w) || !eligible(g, s, k, c) {
                continue;
            }
            if f.label[bw] == 0 {
                let base = s.nodes[bw].base;
                if s.mate[base].is_none() {
                    f.assign(s, w, 1, None);
                    f.augment(s, v, w);
                    return (f, true);
                }
                f.assign(s, w, 2, Some(v));
                let mate = s.mate[base].unwrap();
                let mk = g.edge_id(base, mate);
                if eligible(g, s, mk, c) {
                    f.assign(s, mate, 1, Some(base));
                }
            } else if f.label[bw] == 1 {
                match f.scan(s, v, w) {
                    Some(base) => f.shrink(s, base, v, w),
                    None => {
                        f.augment(s, v, w);
                        return (f, true);
                    }
                }
            }
        }
    }
    (f, false)
}
