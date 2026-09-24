//! Union-Find (Disjoint Set Union) data structure for blossom contraction
//!
//! Translated from the Korte-Vygen C++ implementation

use std::cell::RefCell;

pub struct UnionFind {
    parent: RefCell<Vec<usize>>,
    rank: RefCell<Vec<usize>>,
    // Store the representant value for each element
    value: RefCell<Vec<usize>>,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            parent: RefCell::new(Vec::new()),
            rank: RefCell::new(Vec::new()),
            value: RefCell::new(Vec::new()),
        }
    }

    pub fn reset(&mut self, n: usize) {
        let mut parent = self.parent.borrow_mut();
        let mut rank = self.rank.borrow_mut();
        let mut value = self.value.borrow_mut();

        parent.resize(n, 0);
        rank.resize(n, 0);
        value.resize(n, 0);

        for i in 0..n {
            parent[i] = i;
            rank[i] = 0;
            value[i] = i;
        }
    }

    pub fn find(&self, mut v: usize) -> usize {
        // Path compression
        while self.parent.borrow()[v] != v {
            let parent_v = self.parent.borrow()[v];
            let grandparent_v = self.parent.borrow()[parent_v];
            self.parent.borrow_mut()[v] = grandparent_v;
            v = parent_v;
        }
        v
    }

    pub fn unite(&self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return;
        }

        // Union by rank
        let rank_a = self.rank.borrow()[ra];
        let rank_b = self.rank.borrow()[rb];

        if rank_a < rank_b {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent.borrow_mut()[rb] = ra;
        if rank_a == rank_b {
            self.rank.borrow_mut()[ra] += 1;
        }
    }

    pub fn is_representant(&self, v: usize) -> bool {
        self.find(v) == v
    }

    pub fn fast_disconnect_element(&self, v: usize) {
        self.parent.borrow_mut()[v] = v;
        self.rank.borrow_mut()[v] = 0;
    }

    pub fn dissolve(&self, values: &[usize]) {
        for &val in values {
            self.fast_disconnect_element(val);
        }
    }
}
