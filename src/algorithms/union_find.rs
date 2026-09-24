//! Union-Find (Disjoint Set Union) data structure for blossom contraction
//!
//! Translated from the Korte-Vygen C++ implementation

pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            parent: Vec::new(),
            rank: Vec::new(),
        }
    }

    pub fn reset(&mut self, n: usize) {
        self.parent.resize(n, 0);
        self.rank.resize(n, 0);

        for i in 0..n {
            self.parent[i] = i;
            self.rank[i] = 0;
        }
    }

    pub fn find(&mut self, mut v: usize) -> usize {
        // Path compression
        while self.parent[v] != v {
            let parent_v = self.parent[v];
            self.parent[v] = self.parent[parent_v];
            v = parent_v;
        }
        v
    }

    pub fn unite(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return;
        }

        // Union by rank
        let rank_a = self.rank[ra];
        let rank_b = self.rank[rb];

        if rank_a < rank_b {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        if rank_a == rank_b {
            self.rank[ra] += 1;
        }
    }

    pub fn is_representant(&mut self, v: usize) -> bool {
        self.find(v) == v
    }

    pub fn fast_disconnect_element(&mut self, v: usize) {
        self.parent[v] = v;
        self.rank[v] = 0;
    }

    pub fn dissolve(&mut self, values: &[usize]) {
        for &val in values {
            self.fast_disconnect_element(val);
        }
    }
}
