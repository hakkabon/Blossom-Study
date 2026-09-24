//! Edmonds' Blossom Algorithm for Maximum Cardinality Matching via Ear Decompositions
//!
//! Based on: B. Korte & J. Vygen, *Combinatorial Optimization: Theory and Algorithms*,
//! Springer, Chapter on "Matchings and Matroids".
//!
//! # Structural Mappings
//!
//! The Korte-Vygen formulation uses three elegant mappings to maintain the alternating forest:
//! - $\mu(v)$ (`mu`): Current matching partner ($\mu(v) = v$ when $v$ is unmatched).
//! - $\phi(v)$ (`phi`): For inner vertices, points towards the tree root. For blossom vertices,
//!   it encodes the alternating ear decomposition cycle so augmenting paths can unwind
//!   blossoms without recursive unshrinking.
//! - $\rho(v)$ (`rho`): A Disjoint-Set Union (Union-Find) structure identifying the base
//!   (representative) of the outermost blossom containing $v$.
//!
//! # Algorithmic Stages
//!
//! 1. **Greedy Initialization**: An initial maximal matching is constructed greedily.
//! 2. **Forest Expansion & Ear Contraction**:
//!    Unmatched vertices start as tree roots (outer vertices). Unscanned outer vertices
//!    explore incident edges:
//!    - Edge to an unreached vertex: **Grows** the alternating tree.
//!    - Edge to an outer vertex in a different tree: **Augments** the matching along $P(x) \cup P(y)$.
//!    - Edge to an outer vertex in the same tree: **Shrinks** the odd cycle into a blossom
//!      by uniting bases in $\rho$ and closing the $\phi$ pointer cycle (converting the path into an ear).

use crate::algorithms::union_find::UnionFind;
use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VertexType {
    Inner,
    Outer,
    OutOfForest,
}

/// Korte-Vygen Edmonds Blossom Algorithm for Maximum Cardinality Matching
pub struct KorteVygenEdmonds {
    graph: Graph,

    // mu mapping: matching partner (mu[v] == v means unmatched)
    mu: Vec<usize>,

    // phi mapping: points towards tree root for inner vertices,
    // participates in ear decomposition for blossoms
    phi: Vec<usize>,

    // Queue of outer vertex candidates
    outer_vertices: VecDeque<usize>,

    // Has vertex been scanned completely?
    scanned: Vec<bool>,

    // Tree root for each vertex
    tree: Vec<usize>,

    // Forest: vertices belonging to each tree root
    forest: Vec<Vec<usize>>,

    // Union-Find for blossom mapping (rho)
    rho: UnionFind,

    n: usize,

    // Performance telemetry
    iterations: usize,
    blossoms_created: usize,
    blossoms_expanded: usize,
}

impl KorteVygenEdmonds {
    /// Creates a new solver initialized with `graph`
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        Self {
            graph: graph.clone(),
            mu: vec![0; n],
            phi: vec![0; n],
            outer_vertices: VecDeque::new(),
            scanned: vec![false; n],
            tree: vec![0; n],
            forest: vec![Vec::new(); n],
            rho: UnionFind::new(),
            n,
            iterations: 0,
            blossoms_created: 0,
            blossoms_expanded: 0,
        }
    }

    #[inline]
    fn is_outer_vertex(&self, v: usize) -> bool {
        self.mu[v] == v || self.phi[self.mu[v]] != self.mu[v]
    }

    #[inline]
    fn is_inner_vertex(&self, v: usize) -> bool {
        self.phi[self.mu[v]] == self.mu[v] && self.phi[v] != v
    }

    #[inline]
    fn vertex_type(&self, v: usize) -> VertexType {
        if self.is_outer_vertex(v) {
            VertexType::Outer
        } else if self.is_inner_vertex(v) {
            VertexType::Inner
        } else {
            VertexType::OutOfForest
        }
    }

    fn reset(&mut self) {
        self.rho.reset(self.n);
        self.outer_vertices.clear();

        for v in 0..self.n {
            self.phi[v] = v;
            self.tree[v] = v;
            self.forest[v].clear();
            self.scanned[v] = false;

            if self.is_outer_vertex(v) {
                self.outer_vertices.push_back(v);
            }
        }
    }

    fn find_unscanned_outer_vertex(&mut self) -> Option<usize> {
        while let Some(v) = self.outer_vertices.pop_front() {
            if !self.scanned[v] && self.is_outer_vertex(v) {
                return Some(v);
            }
        }
        None
    }

    fn neighbor_search(&self, x: usize) -> Option<(usize, VertexType)> {
        let x_rho = self.rho.find(x);

        for &edge_idx in &self.graph.adj_list[x] {
            let edge = &self.graph.edges[edge_idx];
            let w = if edge.u == x { edge.v } else { edge.u };

            let t = self.vertex_type(w);
            let w_rho = self.rho.find(w);

            if t == VertexType::OutOfForest || (t == VertexType::Outer && w_rho != x_rho) {
                return Some((w, t));
            }
        }
        None
    }

    fn path_to_root(&self, mut v: usize) -> Vec<usize> {
        assert!(self.is_outer_vertex(v));

        let mut path = Vec::new();
        path.push(v);

        while v != self.mu[v] {
            v = self.mu[v];
            path.push(v);

            v = self.phi[v];
            path.push(v);
        }

        path
    }

    fn remove_vertex_from_tree(&mut self, v: usize) {
        self.phi[v] = v;
        self.tree[v] = v;
        self.rho.fast_disconnect_element(v);

        // If unmatched, it becomes an outer vertex candidate
        if self.mu[v] == v {
            self.outer_vertices.push_back(v);
            self.scanned[v] = false;
        }

        // All adjacent outer vertices need to be reconsidered
        for &edge_idx in &self.graph.adj_list[v] {
            let edge = &self.graph.edges[edge_idx];
            let w = if edge.u == v { edge.v } else { edge.u };
            if self.scanned[w] {
                self.outer_vertices.push_back(w);
                self.scanned[w] = false;
            }
        }
    }

    fn augment(&mut self, px: &[usize], py: &[usize]) {
        self.blossoms_expanded += 1;
        let x = px[0];
        let y = py[0];

        // Change matching along M-alternating path Px
        for i in (1..px.len()).step_by(2) {
            let v = px[i];
            self.mu[self.phi[v]] = v;
            self.mu[v] = self.phi[v];
        }

        // Change matching along M-alternating path Py
        for i in (1..py.len()).step_by(2) {
            let v = py[i];
            self.mu[self.phi[v]] = v;
            self.mu[v] = self.phi[v];
        }

        // Add edge {x,y} to matching
        self.mu[x] = y;
        self.mu[y] = x;

        // Reset phi, rho, scanned in the affected trees
        let rx = px[px.len() - 1]; // root of x tree
        let ry = py[py.len() - 1]; // root of y tree

        // Reset root rx and all its descendants without cloning memory
        self.remove_vertex_from_tree(rx);
        let forest_rx = std::mem::take(&mut self.forest[rx]);
        for v in forest_rx {
            self.remove_vertex_from_tree(v);
        }

        // Reset root ry and all its descendants without cloning memory
        self.remove_vertex_from_tree(ry);
        let forest_ry = std::mem::take(&mut self.forest[ry]);
        for v in forest_ry {
            self.remove_vertex_from_tree(v);
        }
    }

    fn convert_path_to_ear(&mut self, path: &[usize], r_idx: usize) {
        // Search backwards in the path until we exit the blossom at r
        let mut i = path.len() as isize - r_idx as isize - 2;

        while i > 0 {
            let v = path[i as usize];
            if self.rho.is_representant(v) {
                break;
            }
            i -= 2;
        }

        if i < 0 {
            return;
        }

        // We are at an inner node v, which is its own representant.
        // This means we exited the blossom belonging to base r.
        // Go one inner node further.
        self.outer_vertices.push_back(path[i as usize]);
        i -= 2;

        while i > 0 {
            let v = path[i as usize];

            // Modify the phi pointer of our phi neighbor (outer vertex) to point back at us
            let phi_v = self.phi[v];
            self.phi[phi_v] = v;

            // Old inner vertices become outer vertices in the blossom
            self.outer_vertices.push_back(v);

            i -= 2;
        }
    }

    fn unite_bases_along_path(&mut self, path: &[usize], r: usize) {
        let mut v = path[0];

        while v != r {
            assert!(self.is_outer_vertex(v));
            assert!(v != self.phi[self.mu[v]]);

            if self.rho.is_representant(v) {
                // Outer nodes which are their own representants are bases of blossoms
                // Unite the rho class with rho(r)
                self.rho.unite(r, v);

                // Our matching partner is surely not part of a blossom
                // => it is its own representant => also merge it
                self.rho.unite(r, self.mu[v]);
            }

            v = self.phi[self.mu[v]];
        }
    }

    fn shrink(&mut self, px: &[usize], py: &[usize]) {
        self.blossoms_created += 1;
        let x = px[0];
        let y = py[0];

        // Find first vertex r in intersection of P(x) and P(y) which has
        // rho(r) == r (is the base of a blossom or inner vertex).
        // We do this by scanning the paths backwards.
        let mut r = 0;
        let mut r_idx = 0;

        let min_len = px.len().min(py.len());
        for i in 0..min_len {
            let nx = px[px.len() - 1 - i];
            let ny = py[py.len() - 1 - i];

            // If the paths disagree, we exit the intersection
            if nx != ny {
                break;
            }

            if self.rho.is_representant(nx) {
                r = nx;
                r_idx = i;
            }
        }

        // Fix the phi mapping to convert the path to an ear with base r
        self.convert_path_to_ear(px, r_idx);
        self.convert_path_to_ear(py, r_idx);

        // Close phi over {x,y}
        if self.rho.find(x) != r {
            self.phi[x] = y;
        }

        if self.rho.find(y) != r {
            self.phi[y] = x;
        }

        // Unite all rho classes we encounter along the way
        self.unite_bases_along_path(px, r);
        self.unite_bases_along_path(py, r);
    }

    fn step(&mut self, x: usize) {
        loop {
            self.iterations += 1;
            assert!(self.is_outer_vertex(x) && !self.scanned[x]);

            // Find a neighbor of x which is either out-of-tree
            // or outer and part of different tree
            match self.neighbor_search(x) {
                None => {
                    self.scanned[x] = true;
                    return;
                }
                Some((y, y_type)) => {
                    if y_type == VertexType::OutOfForest {
                        // Grow
                        self.phi[y] = x;

                        // Mark the two nodes as belonging to the current tree
                        self.tree[y] = self.tree[x];
                        self.tree[self.mu[y]] = self.tree[x];

                        self.forest[self.tree[x]].push(y);
                        self.forest[self.tree[x]].push(self.mu[y]);

                        // We got a new outer vertex
                        self.outer_vertices.push_back(self.mu[y]);

                        continue;
                    }

                    // Calculate P(x) and P(y)
                    let px = self.path_to_root(x);
                    let py = self.path_to_root(y);

                    // P(x) and P(y) are not vertex-disjoint iff they have the same
                    // "tail" ending in the shared tree root
                    if px[px.len() - 1] != py[py.len() - 1] {
                        // The paths end in different trees -> AUGMENT along Px,Py
                        self.augment(&px, &py);

                        // After the augment the current tree is destroyed
                        return;
                    } else {
                        // The paths end in the same tree -> SHRINK the blossom
                        self.shrink(&px, &py);
                    }
                }
            }
        }
    }

    /// Computes maximum cardinality matching
    pub fn max_cardinality(&mut self) -> MatchingResult {
        let start = Instant::now();

        // Setup
        self.mu.resize(self.n, 0);
        self.phi.resize(self.n, 0);
        self.rho.reset(self.n);
        self.scanned.resize(self.n, false);
        self.tree.resize(self.n, 0);
        self.forest = vec![Vec::new(); self.n];
        self.iterations = 0;
        self.blossoms_created = 0;
        self.blossoms_expanded = 0;

        // Initialize empty matching
        let mut sorting: Vec<usize> = (0..self.n).collect();

        for v in 0..self.n {
            self.mu[v] = v;
        }

        // Sort by vertex degree for initial greedy matching
        sorting.sort_by_key(|&v| self.graph.adj_list[v].len());

        // Start with greedy matching (each vertex examined is counted as one iteration)
        for &v in &sorting {
            if self.mu[v] != v {
                continue;
            }

            self.iterations += 1;

            for &edge_idx in &self.graph.adj_list[v] {
                let edge = &self.graph.edges[edge_idx];
                let w = if edge.u == v { edge.v } else { edge.u };

                if self.mu[w] == w {
                    self.mu[w] = v;
                    self.mu[v] = w;
                    break;
                }
            }
        }

        // Reset forest pointers and init outer vertex queue
        self.reset();

        // While there is an unscanned outer vertex x, call step(x)
        while let Some(x) = self.find_unscanned_outer_vertex() {
            self.step(x);
        }

        // Recover matching from mu
        let mut edges = Vec::new();
        let mut scanned = vec![false; self.n];

        for v in 0..self.n {
            if scanned[v] || self.mu[v] == v {
                continue;
            }
            if let Some(edge_idx) = self.graph.edge_index(v, self.mu[v]) {
                edges.push(edge_idx);
                scanned[self.mu[v]] = true;
            }
        }

        let edge_count = edges.len();
        let total_weight = edges
            .iter()
            .map(|&i| self.graph.edges[i].weight.unwrap_or(1.0))
            .sum();

        MatchingResult {
            edges,
            total_weight,
            is_perfect: edge_count * 2 == self.n,
            metadata: MatchingMetadata {
                iterations: self.iterations,
                blossoms_created: self.blossoms_created,
                blossoms_expanded: self.blossoms_expanded,
                dual_updates: 0,
                runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
        }
    }

    /// Algorithm name
    pub fn name(&self) -> &'static str {
        "korte_vygen"
    }
}

impl KorteVygenEdmonds {
    /// Computes maximum cardinality matching on `graph`
    pub fn max_cardinality_matching(graph: &Graph) -> MatchingResult {
        let mut algo = Self::new(graph);
        algo.max_cardinality()
    }

    /// Computes minimum weight perfect matching on `graph`
    pub fn min_weight_perfect_matching(graph: &Graph) -> Result<MatchingResult, String> {
        crate::algorithms::minimum_weight_perfect_matching(graph)
    }
}
