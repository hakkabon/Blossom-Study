//! Blossom bookkeeping shared by both phases.
//!
//! * [`BlossomForest`] is the paper's `R_B` representation: every blossom is
//!   an ordered ring of children `C_0..C_k` plus the edge `c_i c_{i+1}` joining
//!   consecutive children (Sec. 3.1).  `path_to_base` implements `P(v, b)`.
//! * [`Contraction`] couples the forest with a disjoint-set structure that
//!   answers "which maximal blossom / which base does vertex `x` have?"
//!   (`B_x`, `b(x)`).  The paper uses the linear-time incremental-tree
//!   set-merging of Gabow-Tarjan; a union-find with path halving is used here
//!   (inverse-Ackermann instead of strictly linear).

pub const NONE: usize = usize::MAX;

/// Edge `eid` joins vertex `p` of ring child `i` to vertex `q` of child `i+1`.
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub eid: usize,
    pub p: usize,
    pub q: usize,
}

#[derive(Clone, Debug)]
pub struct Blossom {
    pub base: usize,
    /// value of the dual-adjustment counter Delta when the blossom was formed
    pub stamp: i64,
    /// ring `C_0..C_k` (k even); `C_0` holds the base. `cycle[i].1` joins
    /// `C_i` to `C_{(i+1) mod (k+1)}`.  Link `i` is matched iff `i` is odd.
    pub cycle: Vec<(usize, Link)>,
}

/// Node ids `0..n` are the leaves (the vertices); `n..` are blossoms.
#[derive(Clone, Debug)]
pub struct BlossomForest {
    n: usize,
    blossoms: Vec<Blossom>,
    parent: Vec<usize>,
    idx: Vec<usize>,
}

impl BlossomForest {
    pub fn new(n: usize) -> Self {
        BlossomForest {
            n,
            blossoms: Vec::new(),
            parent: vec![NONE; n],
            idx: vec![0; n],
        }
    }
    pub fn is_leaf(&self, id: usize) -> bool {
        id < self.n
    }
    pub fn base(&self, id: usize) -> usize {
        if self.is_leaf(id) {
            id
        } else {
            self.blossoms[id - self.n].base
        }
    }
    pub fn stamp(&self, id: usize) -> i64 {
        if self.is_leaf(id) {
            -1
        } else {
            self.blossoms[id - self.n].stamp
        }
    }
    pub fn cycle(&self, id: usize) -> &[(usize, Link)] {
        &self.blossoms[id - self.n].cycle
    }

    pub fn new_blossom(&mut self, base: usize, stamp: i64, cycle: Vec<(usize, Link)>) -> usize {
        let id = self.n + self.blossoms.len();
        for (i, &(c, _)) in cycle.iter().enumerate() {
            self.parent[c] = id;
            self.idx[c] = i;
        }
        self.parent.push(NONE);
        self.idx.push(0);
        self.blossoms.push(Blossom { base, stamp, cycle });
        id
    }

    /// index of the ring child of `node` that contains leaf `v`
    fn child_index(&self, node: usize, v: usize) -> usize {
        let mut cur = v;
        while self.parent[cur] != node {
            cur = self.parent[cur];
            debug_assert!(cur != NONE, "vertex not inside blossom");
        }
        self.idx[cur]
    }

    /// Appends to `out` the edge ids of `P(v, b)`: the even alternating path
    /// inside `node` from `v` to the base of `node`, starting with the matched
    /// edge at `v` (empty if `v` is the base).  Edge order is irrelevant for
    /// augmenting, so edges are appended in discovery order.
    pub fn path_to_base(&self, node: usize, v: usize, out: &mut Vec<usize>) {
        if self.is_leaf(node) {
            return;
        }
        let cyc = self.cycle(node);
        let len = cyc.len();
        let j = self.child_index(node, v);
        let cj = cyc[j].0;
        self.path_to_base(cj, v, out); // inside C_j: v -> base(C_j)
        if j == 0 {
            return;
        }
        if j % 2 == 1 {
            // matched ring edge e_j leaves C_j at its base: walk forward to C_0
            let mut t = j;
            loop {
                out.push(cyc[t].1.eid);
                let next = (t + 1) % len;
                let q = cyc[t].1.q;
                if next == 0 {
                    self.path_to_base(cyc[0].0, q, out);
                    break;
                }
                let c = cyc[next].0;
                let exit = cyc[next].1.p;
                let bc = self.base(c);
                debug_assert!(q == bc || exit == bc);
                let other = if q == bc { exit } else { q };
                self.path_to_base(c, other, out);
                t = next;
            }
        } else {
            // matched ring edge e_{j-1} leaves C_j at its base: walk backward to C_0
            let mut t = j - 1;
            loop {
                out.push(cyc[t].1.eid);
                let p = cyc[t].1.p;
                if t == 0 {
                    self.path_to_base(cyc[0].0, p, out);
                    break;
                }
                let c = cyc[t].0;
                let exit = cyc[t - 1].1.q;
                let bc = self.base(c);
                debug_assert!(p == bc || exit == bc);
                let other = if p == bc { exit } else { p };
                self.path_to_base(c, other, out);
                t -= 1;
            }
        }
    }
}

use blossom_dsu::UnionFind as Dsu;

/// `B_x` / `b(x)` queries plus blossom creation.
#[derive(Clone, Debug)]
pub struct Contraction {
    pub dsu: Dsu,
    /// indexed by dsu root
    pub base: Vec<usize>,
    pub top: Vec<usize>,
    pub forest: BlossomForest,
}

impl Contraction {
    pub fn new(n: usize) -> Self {
        Contraction {
            dsu: Dsu::new(n),
            base: (0..n).collect(),
            top: (0..n).collect(),
            forest: BlossomForest::new(n),
        }
    }
    /// `b(x)`: base vertex of the maximal blossom containing `x`
    pub fn base_of(&mut self, x: usize) -> usize {
        let r = self.dsu.find(x);
        self.base[r]
    }
    /// forest node of the maximal blossom `B_x`
    pub fn top_of(&mut self, x: usize) -> usize {
        let r = self.dsu.find(x);
        self.top[r]
    }
    /// Contract the ring `cycle` (first element holds the base) into one blossom.
    pub fn contract(&mut self, cycle: Vec<(usize, Link)>, stamp: i64) -> usize {
        let b0 = self.forest.base(cycle[0].0);
        let members: Vec<usize> = cycle.iter().map(|&(nd, _)| self.forest.base(nd)).collect();
        let node = self.forest.new_blossom(b0, stamp, cycle);
        let mut r = self.dsu.find(members[0]);
        for &m in &members[1..] {
            r = self.dsu.union_root(r, m);
        }
        self.base[r] = b0;
        self.top[r] = node;
        node
    }
}
