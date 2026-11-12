//! Fast Blossom Matching Algorithm using DSU with Path Compression
//!
//! Based on Edmonds' Blossom Algorithm optimized with:
//! - Disjoint Set Union (DSU) for near-$O(1)$ blossom contraction.
//! - Monotonic timestamp-based Lowest Common Ancestor (LCA) search ($O(\text{cycle})$ without array resets).
//! - Direct path backtracking without explicit recursive blossom structures.
//!
//! # Algorithmic Details
//!
//! 1. **Alternating Forest & DSU Contraction**:
//!    Vertices are labeled as even (outer / 0) or odd (inner / 1). When two even vertices
//!    in the same alternating tree are connected by an edge, an odd cycle (blossom) is found.
//!    All vertices along the cycle are contracted into a single super-vertex represented
//!    by their LCA root in the DSU.
//!
//! 2. **Timestamp LCA**:
//!    Finding the LCA of two vertices along alternating paths typically requires marking
//!    visited nodes. Instead of clearing a boolean array of size $V$ for every blossom,
//!    we increment a monotonic `vis_token` counter, giving $O(\text{cycle length})$ LCA lookup.
//!
//! 3. **Augmentation**:
//!    When an even vertex discovers an edge to an unmatched vertex (or an even vertex in a
//!    different tree), an augmenting path is found. Edges along the path are flipped in $O(V)$ time.

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::collections::VecDeque;
use std::time::Instant;

use blossom_dsu::UnionFind as Dsu;

/// Fast Blossom Matching Algorithm
///
/// Maximum cardinality matching using Edmonds' Blossom Algorithm
/// with DSU-based blossom contraction and timestamp-based LCA.
#[derive(Clone, Debug)]
pub struct FastBlossomMatching {
    n: usize,
    adj: Vec<Vec<usize>>,
    pub match_pair: Vec<usize>,

    // Core structural arrays
    p: Vec<usize>,      // Parent array for alternating tree
    orig_p: Vec<usize>, // Helps step out of contracted blossoms correctly
    label: Vec<i8>,     // Labels: -1 = unvisited, 0 = outer (even), 1 = inner (odd)

    // Pre-allocated tracking structures
    dsu: Dsu,
    vis: Vec<usize>,  // Timestamp array for rapid O(1) LCA lookups
    vis_token: usize, // Incremental token to avoid resetting 'vis' array

    // Execution metrics
    iterations: usize,
    blossoms_created: usize,
    augmentations: usize,
}

impl FastBlossomMatching {
    /// Creates a new matcher with given adjacency list
    pub fn new(n: usize, adj: Vec<Vec<usize>>) -> Self {
        Self {
            n,
            adj,
            match_pair: vec![usize::MAX; n],
            p: vec![usize::MAX; n],
            orig_p: vec![usize::MAX; n],
            label: vec![-1; n],
            dsu: Dsu::new(n),
            vis: vec![0; n],
            vis_token: 0,
            iterations: 0,
            blossoms_created: 0,
            augmentations: 0,
        }
    }

    /// Creates matcher from a [`Graph`] reference
    pub fn from_graph(graph: &Graph) -> Self {
        let n = graph.n;
        let mut adj = vec![Vec::new(); n];
        for e in &graph.edges {
            if e.u < n && e.v < n && e.u != e.v {
                adj[e.u].push(e.v);
                adj[e.v].push(e.u);
            }
        }
        Self::new(n, adj)
    }

    /// Fast $O(\text{cycle})$ LCA using monotonic timestamps
    fn lca(&mut self, mut u: usize, mut v: usize) -> usize {
        self.vis_token += 1;
        u = self.dsu.find(u);
        v = self.dsu.find(v);

        loop {
            if u != usize::MAX {
                if self.vis[u] == self.vis_token {
                    return u;
                }
                self.vis[u] = self.vis_token;
                if self.match_pair[u] != usize::MAX {
                    u = self.dsu.find(self.orig_p[self.match_pair[u]]);
                } else {
                    u = usize::MAX;
                }
            }
            // Swap pointers to alternate backtracking up the tree
            std::mem::swap(&mut u, &mut v);
        }
    }

    /// Contracts one side of the blossom up to the LCA base (`b`)
    fn mark_blossom(&mut self, mut v: usize, b: usize, u: usize, q: &mut VecDeque<usize>) {
        while self.dsu.find(v) != b {
            let u_next = self.match_pair[v];

            // If the node was odd (inner), it now becomes even (outer)
            if self.label[u_next] == 1 {
                self.label[u_next] = 0;
                q.push_back(u_next);
            }

            self.p[v] = u;
            self.orig_p[v] = u;

            // Union both endpoints of the matched edge into the LCA base
            self.dsu.union_into(v, b);
            self.dsu.union_into(u_next, b);

            // Backtrack via the parent pointer of the matched pair
            v = self.orig_p[u_next];
        }
    }

    fn find_augmenting_path(&mut self, root: usize, q: &mut VecDeque<usize>) -> usize {
        self.label.fill(-1);
        self.p.fill(usize::MAX);
        self.orig_p.fill(usize::MAX);
        self.dsu.reset(self.n);

        q.clear();
        q.push_back(root);
        self.label[root] = 0;

        while let Some(u) = q.pop_front() {
            self.iterations += 1;

            // Index-based iteration avoids allocating a temporary neighbor vector
            for i in 0..self.adj[u].len() {
                let v = self.adj[u][i];

                // If v is already part of the same contracted blossom, skip
                if self.dsu.find(u) == self.dsu.find(v) || self.match_pair[u] == v {
                    continue;
                }

                // Case 1: Blossom detected (collision of two 'outer' nodes)
                if self.label[v] == 0 {
                    let base_lca = self.lca(u, v);
                    self.blossoms_created += 1;

                    // Call the DSU loop for nodes along both branches of the cycle
                    self.mark_blossom(u, base_lca, v, q);
                    self.mark_blossom(v, base_lca, u, q);
                }
                // Case 2: Unvisited node discovered
                else if self.label[v] == -1 {
                    self.p[v] = u;
                    self.orig_p[v] = u;

                    if self.match_pair[v] == usize::MAX {
                        return v; // Unmatched node found -> Augmenting path complete!
                    }

                    // Extend the alternating tree: the match becomes an 'inner' node
                    let match_v = self.match_pair[v];
                    self.label[v] = 1;
                    self.label[match_v] = 0;
                    q.push_back(match_v);
                }
            }
        }
        usize::MAX
    }

    /// Solves for maximum cardinality matching, returning slice of matched pairs
    pub fn solve(&mut self) -> &[usize] {
        let mut q = VecDeque::with_capacity(self.n);

        for i in 0..self.n {
            if self.match_pair[i] == usize::MAX {
                let mut end_node = self.find_augmenting_path(i, &mut q);

                if end_node != usize::MAX {
                    // Flip edges along the path back to the root
                    while end_node != usize::MAX {
                        let curr_p = self.p[end_node];
                        let next_node = self.match_pair[curr_p];
                        self.match_pair[end_node] = curr_p;
                        self.match_pair[curr_p] = end_node;
                        end_node = next_node;
                    }
                    self.augmentations += 1;
                }
            }
        }
        &self.match_pair
    }

    /// Algorithm name
    pub fn name(&self) -> &'static str {
        "fast_blossom"
    }
}

/// Convert match_pair array to our MatchingResult format
impl FastBlossomMatching {
    /// Solves maximum cardinality matching on `graph`
    pub fn max_cardinality_matching(graph: &Graph) -> MatchingResult {
        let start = Instant::now();
        let mut matcher = Self::from_graph(graph);
        matcher.solve();

        let mut edges = Vec::new();
        let mut scanned = vec![false; matcher.n];

        for v in 0..matcher.n {
            if scanned[v] || matcher.match_pair[v] == usize::MAX {
                continue;
            }
            if let Some(edge_idx) = graph.edge_index(v, matcher.match_pair[v]) {
                edges.push(edge_idx);
                scanned[matcher.match_pair[v]] = true;
            }
        }

        let edge_count = edges.len();
        let total_weight = edges.iter().map(|&i| graph.edges[i].weight_or_one()).sum();

        MatchingResult {
            edges,
            total_weight,
            is_perfect: edge_count * 2 == matcher.n,
            metadata: MatchingMetadata {
                iterations: matcher.iterations,
                augmentations: matcher.augmentations,
                blossoms_created: matcher.blossoms_created,
                blossoms_expanded: 0,
                dual_updates: 0,
                runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
        }
    }
}
