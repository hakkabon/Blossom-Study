//! Fast Blossom Matching Algorithm using DSU with Path Compression
//!
//! Based on the implementation from blossom-dsu project.
//! Features efficient LCA with timestamp-based approach and DSU-based blossom contraction.
//!
//! Tested against pathological cases:
//! - Pentagon with Tail (odd cycle with extension)
//! - Nested Blossoms (blossom inside blossom)
//! - Unmatchable Triangle (odd cycle, maximum sub-pair limit)

use std::collections::VecDeque;

/// Union-Find (Disjoint Set Union) with path compression
#[derive(Clone, Debug)]
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn reset(&mut self) {
        for i in 0..self.parent.len() {
            self.parent[i] = i;
        }
    }

    /// Finds the representative (blossom root) with path compression
    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            self.parent[i] = self.find(self.parent[i]);
            self.parent[i]
        }
    }

    /// Merges child blossom into the parent blossom root
    fn union_sets(&mut self, child: usize, parent: usize) {
        let root_child = self.find(child);
        let root_parent = self.find(parent);
        if root_child != root_parent {
            self.parent[root_child] = root_parent;
        }
    }
}

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
}

impl FastBlossomMatching {
    /// Create a new matcher with adjacency list
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
        }
    }

    /// Create from our Graph structure
    pub fn from_graph(graph: &crate::graph::Graph) -> Self {
        let n = graph.n;
        // Convert edge indices to vertex adjacency list
        let mut adj = vec![Vec::new(); n];
        for e in &graph.edges {
            adj[e.u].push(e.v);
            adj[e.v].push(e.u);
        }
        Self::new(n, adj)
    }

    /// Fast O(Length of Cycle) LCA using monotonic timestamps
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

    /// Contracts one side of the blossom up to the LCA base (b)
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
            self.dsu.union_sets(v, b);
            self.dsu.union_sets(u_next, b);

            // Backtrack via the parent pointer of the matched pair
            v = self.orig_p[u_next];
        }
    }

    fn find_augmenting_path(&mut self, root: usize, q: &mut VecDeque<usize>) -> usize {
        self.label.fill(-1);
        self.p.fill(usize::MAX);
        self.orig_p.fill(usize::MAX);
        self.dsu.reset();

        q.clear();
        q.push_back(root);
        self.label[root] = 0;

        while let Some(u) = q.pop_front() {
            // Collect neighbors to avoid borrow conflicts
            let neighbors = self.adj[u].to_vec();

            for &v in &neighbors {
                // If v is already part of the same contracted blossom, skip
                if self.dsu.find(u) == self.dsu.find(v) || self.match_pair[u] == v {
                    continue;
                }

                // Case 1: Blossom detected (collision of two 'outer' nodes)
                if self.label[v] == 0 {
                    let base_lca = self.lca(u, v);

                    // Call the DSU loop ONLY for nodes along the cycle paths
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

    /// Solve for maximum cardinality matching
    pub fn solve(&mut self) -> &[usize] {
        let mut q = VecDeque::with_capacity(self.n);

        for i in 0..self.n {
            if self.match_pair[i] == usize::MAX {
                let mut end_node = self.find_augmenting_path(i, &mut q);

                // Flip edges along the path back to the root
                while end_node != usize::MAX {
                    let curr_p = self.p[end_node];
                    let next_node = self.match_pair[curr_p];
                    self.match_pair[end_node] = curr_p;
                    self.match_pair[curr_p] = end_node;
                    end_node = next_node;
                }
            }
        }
        &self.match_pair
    }
}

/// Convert match_pair array to our MatchingResult format
impl FastBlossomMatching {
    pub fn max_cardinality_matching(
        graph: &crate::graph::Graph,
    ) -> crate::algorithms::MatchingResult {
        let mut matcher = Self::from_graph(graph);
        matcher.solve();

        let mut edges = Vec::new();
        let mut scanned = vec![false; matcher.n];

        for v in 0..matcher.n {
            if !scanned[v] && matcher.match_pair[v] != usize::MAX {
                edges.push(
                    graph
                        .edge_index(v, matcher.match_pair[v])
                        .expect("matched edge must exist"),
                );
                scanned[matcher.match_pair[v]] = true;
            }
        }

        let edge_count = edges.len();
        let total_weight = edges
            .iter()
            .map(|&i| graph.edges[i].weight.unwrap_or(1.0))
            .sum();

        crate::algorithms::MatchingResult {
            edges,
            total_weight,
            is_perfect: edge_count * 2 == matcher.n,
            metadata: crate::algorithms::MatchingMetadata {
                iterations: 0,
                blossoms_created: 0,
                blossoms_expanded: 0,
                dual_updates: 0,
                runtime_ms: 0.0,
            },
        }
    }

    pub fn min_weight_perfect_matching(
        graph: &crate::graph::Graph,
    ) -> Result<crate::algorithms::MatchingResult, String> {
        crate::algorithms::minimum_weight_perfect_matching(graph)
    }
}
