//! Paper §2: a laminar forest of nested odd cycles, with explicit path lifting.
//! Alternating-cycle rotation follows the standard Edmonds construction.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub parent: Option<usize>,
    pub base: usize,
    pub children: Vec<usize>,
    pub edges: Vec<(usize, usize)>,
    pub z: i128,
    pub active: bool,
}
#[derive(Clone)]
pub(crate) struct State {
    pub nodes: Vec<Node>,
    pub leaf: Vec<usize>,
    pub top: Vec<usize>,
    pub y: Vec<i128>,
    pub mate: Vec<Option<usize>>,
    pub contractions: usize,
    pub dissolutions: usize,
    pub nested_contractions: usize,
}
impl State {
    pub fn new(n: usize) -> Self {
        let mut s = Self {
            nodes: vec![],
            leaf: vec![],
            top: vec![],
            y: vec![],
            mate: vec![],
            contractions: 0,
            dissolutions: 0,
            nested_contractions: 0,
        };
        for _ in 0..n {
            s.add_vertex(0);
        }
        s
    }
    pub fn add_vertex(&mut self, y: i128) {
        let v = self.y.len();
        let b = self.nodes.len();
        self.nodes.push(Node {
            parent: None,
            base: v,
            children: vec![],
            edges: vec![],
            z: 0,
            active: true,
        });
        self.leaf.push(b);
        self.top.push(b);
        self.y.push(y);
        self.mate.push(None);
    }
    pub fn leaves(&self, b: usize) -> Vec<usize> {
        let mut out = vec![];
        let mut stack = vec![b];
        while let Some(t) = stack.pop() {
            let n = &self.nodes[t];
            if n.children.is_empty() {
                out.push(n.base);
            } else {
                stack.extend(n.children.iter().copied());
            }
        }
        out
    }
    pub fn nontrivial(&self, b: usize) -> bool {
        !self.nodes[b].children.is_empty()
    }
    pub fn allocate(&mut self, base: usize) -> usize {
        let node = Node {
            parent: None,
            base,
            children: vec![],
            edges: vec![],
            z: 0,
            active: true,
        };
        if let Some(b) = self.nodes.iter().position(|b| !b.active) {
            self.nodes[b] = node;
            b
        } else {
            self.nodes.push(node);
            self.nodes.len() - 1
        }
    }
    /// Expand a zero-dual root. Searches are rebuilt afterwards; no fragile
    /// in-search relabeling or cached least-slack edges are retained (§2.2).
    pub fn dissolve(&mut self, b: usize) {
        assert!(self.nodes[b].parent.is_none());
        assert_eq!(self.nodes[b].z, 0);
        for c in self.nodes[b].children.clone() {
            self.nodes[c].parent = None;
            for v in self.leaves(c) {
                self.top[v] = c;
            }
        }
        self.nodes[b].active = false;
        self.dissolutions += 1;
    }
    pub fn dissolve_zero_roots(&mut self) {
        loop {
            let b = self.nodes.iter().enumerate().find_map(|(b, n)| {
                (n.active && n.parent.is_none() && !n.children.is_empty() && n.z == 0).then_some(b)
            });
            match b {
                Some(b) => self.dissolve(b),
                None => break,
            }
        }
    }
    /// Lift an alternating path from vertex v to the base of blossom b,
    /// flipping its matched edges and rotating the representation to base v.
    pub fn augment_blossom(&mut self, b: usize, v: usize) {
        let mut t = self.leaf[v];
        while self.nodes[t].parent != Some(b) {
            t = self.nodes[t].parent.expect("vertex outside blossom");
        }
        if self.nontrivial(t) {
            self.augment_blossom(t, v);
        }
        let children = self.nodes[b].children.clone();
        let edges = self.nodes[b].edges.clone();
        let len = children.len() as isize;
        let i = children.iter().position(|&c| c == t).unwrap();
        let mut j = i as isize;
        let step = if i % 2 == 1 {
            j -= len;
            1
        } else {
            -1
        };
        while j != 0 {
            j += step;
            let t = children[j.rem_euclid(len) as usize];
            let (w, x) = if step == 1 {
                edges[j.rem_euclid(len) as usize]
            } else {
                let (x, w) = edges[(j - 1).rem_euclid(len) as usize];
                (w, x)
            };
            if self.nontrivial(t) {
                self.augment_blossom(t, w);
            }
            j += step;
            let t = children[j.rem_euclid(len) as usize];
            if self.nontrivial(t) {
                self.augment_blossom(t, x);
            }
            self.mate[w] = Some(x);
            self.mate[x] = Some(w);
        }
        self.nodes[b].children.rotate_left(i);
        self.nodes[b].edges.rotate_left(i);
        self.nodes[b].base = self.nodes[self.nodes[b].children[0]].base;
        assert_eq!(self.nodes[b].base, v);
    }
}
