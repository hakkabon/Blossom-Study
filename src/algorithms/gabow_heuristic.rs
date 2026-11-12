//! Gabow-inspired heuristic phased cardinality matching in general graphs.
//!
//! # Relationship to the Literature
//!
//! This module is **inspired by** the phased approach described by Ansaripour,
//! Danaei, and Mehlhorn (2024, arXiv:2409.14849) and Gabow (2017,
//! arXiv:1611.07541), but it is **not** a faithful implementation of their
//! algorithm. The key missing component is Gabow's **auxiliary graph H**:
//!
//! - **What Gabow's algorithm does**: Phase 1 constructs a directed layered
//!   auxiliary graph H whose edges encode admissible alternating steps
//!   (including directed blossom-cycle traversals). Phase 2 runs a DFS within
//!   H with dead-end pruning to extract a maximal set of vertex-disjoint
//!   shortest augmenting paths in O(m) time. This is what gives the algorithm
//!   its O(m√n) worst-case guarantee.
//!
//! - **What this code does instead**: Phase 1 runs a multi-source Edmonds-style
//!   BFS with union-find blossom contraction and collects candidate cross-tree
//!   bridges. Phase 2 traces alternating walks back to exposed roots using
//!   parent pointers in the original graph G (not in H), greedily skipping
//!   paths that overlap already-committed vertices. When the phased search
//!   stalls or misses paths hidden inside nested blossoms, a single-path
//!   Edmonds fallback search guarantees correctness.
//!
//! The result is a **correct maximum-cardinality matcher** (verified against
//! exhaustive brute-force oracles), but without the O(m√n) worst-case bound.
//! It is best described as a heuristic that happens to be very fast in
//! practice, particularly on dense graphs.
//!
//! # Algorithmic Architecture
//!
//! The search organizes work into iterations with two phases:
//!
//! 1. **Phase 1: Multi-source BFS with blossom contraction**
//!    - Seeds the BFS queue with all currently exposed (unmatched) vertices at
//!      level 0.
//!    - Grows alternating trees level by level, classifying vertices as *outer*
//!      (even distance) or *inner* (odd distance).
//!    - When an edge connects two outer vertices in the same tree, a blossom
//!      (odd cycle) is detected. The cycle is contracted into its LCA base in
//!      the union-find, and parent pointers are updated around the cycle.
//!    - When an edge connects two outer vertices in different trees, an
//!      augmenting bridge is found. The shortest such bridge length L is
//!      recorded, and all bridges of length L are collected.
//!    - **Not constructed**: the directed auxiliary graph H with admissible
//!      blossom-cycle edges that Gabow's algorithm uses in Phase 2.
//!
//! 2. **Phase 2: Greedy candidate-path augmentation**
//!    - Iterates over collected bridges and traces alternating walks back to
//!      exposed roots via parent pointers in G.
//!    - Commits vertex-disjoint paths and flips their matched/unmatched edges.
//!    - Falls back to a single-path Edmonds search when the greedy pass yields
//!      no augmentations (e.g. because nested blossoms hid the path from the
//!      BFS parent structure).

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::collections::VecDeque;
use std::time::Instant;

pub use blossom_dsu::UnionFind as Dsu;

/// Heuristic phased cardinality solver inspired by Gabow's algorithm.
///
/// Produces a correct maximum-cardinality matching (verified against
/// brute-force oracles), but does not construct Gabow's auxiliary graph H
/// and therefore lacks the O(m√n) worst-case guarantee. The single-path
/// Edmonds fallback ensures correctness when the greedy phased search
/// misses paths hidden in nested blossoms.
#[derive(Clone, Debug)]
pub struct GabowHeuristic {
    n: usize,
    mate: Vec<Option<usize>>,

    // Phase 1 structural arrays
    dsu: Dsu,
    level: Vec<usize>,
    label: Vec<i8>, // -1: unvisited, 0: outer (even), 1: inner (odd)
    p: Vec<usize>,
    orig_p: Vec<usize>,
    root_of: Vec<usize>,
    vis_token: usize,
    vis: Vec<usize>,

    // Metrics
    iterations: usize,
    augmentations: usize,
    blossoms_created: usize,
}

impl GabowHeuristic {
    /// Create a new solver initialized with an empty matching.
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        Self {
            n,
            mate: vec![None; n],
            dsu: Dsu::new(n),
            level: vec![usize::MAX; n],
            label: vec![-1; n],
            p: vec![usize::MAX; n],
            orig_p: vec![usize::MAX; n],
            root_of: (0..n).collect(),
            vis_token: 0,
            vis: vec![0; n],
            iterations: 0,
            augmentations: 0,
            blossoms_created: 0,
        }
    }

    /// Create a solver initialized with a pre-existing valid matching.
    pub fn with_matching(graph: &Graph, mate: Vec<Option<usize>>) -> Result<Self, &'static str> {
        let n = graph.n;
        if mate.len() != n {
            return Err("matching vector size does not match graph vertex count");
        }
        for (u, &m) in mate.iter().enumerate() {
            if let Some(v) = m {
                if v >= n || v == u || mate[v] != Some(u) {
                    return Err("invalid mate symmetry");
                }
                if graph.edge_index(u, v).is_none() {
                    return Err("matched edge not present in graph");
                }
            }
        }
        let mut solver = Self::new(graph);
        solver.mate = mate;
        Ok(solver)
    }

    pub fn mate(&self) -> &[Option<usize>] {
        &self.mate
    }

    /// Convenience entry point: creates a solver and runs it.
    pub fn compute_matching(graph: &Graph) -> MatchingResult {
        gabow_heuristic_matching(graph)
    }

    /// Solves maximum cardinality matching, returning the [`MatchingResult`].
    pub fn max_cardinality_matching(&mut self, graph: &Graph) -> MatchingResult {
        let start_time = Instant::now();
        self.iterations = 0;
        self.augmentations = 0;
        self.blossoms_created = 0;

        loop {
            self.iterations += 1;

            // Phase 1: alternating search for candidate bridges (not graph H).
            let (shortest_len_opt, candidate_bridges) = self.phase1_search(graph);
            let Some(target_len) = shortest_len_opt else {
                // The phased search is not a complete optimality certificate.
                // In particular, nested blossoms can hide an augmenting path
                // even when no candidate bridge was reported.
                let single = self.augment_single_path(graph);
                if single == 0 {
                    break;
                }
                self.augmentations += single;
                continue;
            };

            // Phase 2: Maximal Disjoint Augmentation
            let aug_count = self.phase2_augment(graph, target_len, &candidate_bridges);
            if aug_count == 0 {
                // Fallback: if phased disjoint batch completed or hit boundary, try single augment
                let single = self.augment_single_path(graph);
                if single == 0 {
                    break;
                }
                self.augmentations += single;
            } else {
                self.augmentations += aug_count;
            }
        }

        self.build_matching_result(graph, start_time)
    }

    /// Convenience solver entry point.
    pub fn solve(&mut self, graph: &Graph) -> &[Option<usize>] {
        self.max_cardinality_matching(graph);
        &self.mate
    }

    // =========================================================================
    // PHASE 1: Multi-source BFS with Blossom Contraction
    // =========================================================================

    pub fn phase1_search(&mut self, graph: &Graph) -> (Option<usize>, Vec<(usize, usize, usize)>) {
        self.dsu.reset(self.n);
        self.level.fill(usize::MAX);
        self.label.fill(-1);
        self.p.fill(usize::MAX);
        self.orig_p.fill(usize::MAX);
        for i in 0..self.n {
            self.root_of[i] = i;
        }

        let mut queue = VecDeque::new();

        // Initialize exposed vertices at level 0 (outer)
        for u in 0..self.n {
            if self.mate[u].is_none() {
                self.label[u] = 0;
                self.level[u] = 0;
                self.root_of[u] = u;
                queue.push_back(u);
            }
        }

        if queue.is_empty() {
            return (None, Vec::new());
        }

        let mut shortest_aug_len: Option<usize> = None;
        let mut candidate_bridges = Vec::new();

        while let Some(u) = queue.pop_front() {
            let u_lvl = self.level[u];

            if let Some(shortest) = shortest_aug_len {
                if u_lvl >= shortest {
                    break;
                }
            }

            for &edge_idx in graph.adj_edges(u) {
                let edge = &graph.edges[edge_idx];
                let v = if edge.u == u { edge.v } else { edge.u };

                if self.mate[u] == Some(v) {
                    continue;
                }

                let base_u = self.dsu.find(u);
                let base_v = self.dsu.find(v);

                if base_u == base_v {
                    continue;
                }

                if self.label[v] == 0 {
                    let v_lvl = self.level[v];
                    let path_len = u_lvl + v_lvl + 1;

                    let root_u = self.find_root(u);
                    let root_v = self.find_root(v);

                    if root_u != root_v {
                        // Augmenting bridge
                        if shortest_aug_len.is_none_or(|min_len| path_len < min_len) {
                            shortest_aug_len = Some(path_len);
                        }
                        if shortest_aug_len == Some(path_len) {
                            candidate_bridges.push((edge_idx, u, v));
                        }
                    } else {
                        // Blossom
                        let lca_base = self.lca(u, v);
                        self.blossoms_created += 1;
                        self.mark_blossom(u, lca_base, v, &mut queue, u_lvl + 1);
                        self.mark_blossom(v, lca_base, u, &mut queue, u_lvl + 1);
                    }
                } else if self.label[v] == -1 {
                    let next_lvl = u_lvl + 1;
                    self.label[v] = 1; // Inner
                    self.level[v] = next_lvl;
                    self.p[v] = u;
                    self.orig_p[v] = u;

                    if let Some(match_v) = self.mate[v] {
                        let mate_lvl = next_lvl + 1;
                        self.label[match_v] = 0; // Outer
                        self.level[match_v] = mate_lvl;
                        self.root_of[match_v] = self.root_of[u];
                        queue.push_back(match_v);
                    }
                }
            }
        }

        (shortest_aug_len, candidate_bridges)
    }

    /// Monotonic timestamp LCA search within the alternating forest.
    fn lca(&mut self, mut u: usize, mut v: usize) -> usize {
        self.vis_token += 1;
        let token = self.vis_token;

        u = self.dsu.find(u);
        v = self.dsu.find(v);

        loop {
            if u != usize::MAX {
                if self.vis[u] == token {
                    return u;
                }
                self.vis[u] = token;

                if let Some(m) = self.mate[u] {
                    if self.orig_p[m] != usize::MAX {
                        u = self.dsu.find(self.orig_p[m]);
                    } else {
                        u = usize::MAX;
                    }
                } else {
                    u = usize::MAX;
                }
            }
            std::mem::swap(&mut u, &mut v);
        }
    }

    fn find_root(&mut self, mut v: usize) -> usize {
        v = self.dsu.find(v);
        while let Some(m) = self.mate[v] {
            if self.orig_p[m] == usize::MAX {
                break;
            }
            v = self.dsu.find(self.orig_p[m]);
        }
        self.dsu.find(v)
    }

    fn mark_blossom(
        &mut self,
        mut v: usize,
        b: usize,
        mut u: usize,
        q: &mut VecDeque<usize>,
        new_lvl: usize,
    ) {
        while self.dsu.find(v) != b {
            let u_next = self.mate[v].expect("non-base blossom vertex is matched");

            if self.label[u_next] == 1 {
                self.label[u_next] = 0;
                if self.level[u_next] > new_lvl {
                    self.level[u_next] = new_lvl;
                }
                q.push_back(u_next);
            }

            self.p[v] = u;
            self.orig_p[v] = u;

            self.dsu.union_into(v, b);
            self.dsu.union_into(u_next, b);

            u = u_next;
            v = self.orig_p[u_next];
        }
    }

    // =========================================================================
    // PHASE 2: Maximal Disjoint Augmentation
    // =========================================================================

    pub fn phase2_augment(
        &mut self,
        graph: &Graph,
        target_len: usize,
        candidate_bridges: &[(usize, usize, usize)],
    ) -> usize {
        let mut committed = vec![false; self.n];
        let mut augmentations = 0;

        for &(_edge_idx, u, v) in candidate_bridges {
            if committed[u] || committed[v] {
                continue;
            }

            // Extract path from u to root
            let path_u = self.trace_to_root(u);
            let path_v = self.trace_to_root(v);

            let root_u = *path_u.last().unwrap();
            let root_v = *path_v.last().unwrap();

            if root_u == root_v || self.mate[root_u].is_some() || self.mate[root_v].is_some() {
                continue;
            }

            // Check disjointness
            let mut conflict = false;
            for &w in &path_u {
                if committed[w] {
                    conflict = true;
                    break;
                }
            }
            if !conflict {
                for &w in &path_v {
                    if committed[w] {
                        conflict = true;
                        break;
                    }
                }
            }
            if conflict {
                continue;
            }

            // Assemble full path: root_u -> ... -> u -- v -> ... -> root_v
            let mut full_path: Vec<usize> = path_u.iter().rev().copied().collect();
            full_path.extend_from_slice(&path_v);

            if full_path.len() == target_len + 1 && self.is_valid_augmenting_path(graph, &full_path)
            {
                self.apply_path_augmentation(&full_path);
                for &w in &full_path {
                    committed[w] = true;
                }
                augmentations += 1;
            }
        }

        augmentations
    }

    /// Traces an alternating path from an outer (even-level) vertex `curr`
    /// back to an exposed root, following the BFS alternating-tree structure.
    ///
    /// ## Alternating Path Pattern
    ///
    /// In Edmonds' BFS, the alternating tree alternates between **outer**
    /// (even-level) and **inner** (odd-level) vertices:
    ///
    /// ```text
    ///   root(outer) ──unmatched──> inner ──matched──> outer ──unmatched──> inner ──…
    /// ```
    ///
    /// Tracing **backward** from an outer vertex toward the root means each
    /// step takes:
    ///   1. The **matched** edge from `outer` to its `inner` partner, then
    ///   2. The **unmatched** edge from `inner` to its BFS parent (`p[inner]`).
    ///
    /// ## Why `p[curr]` Is Wrong for Outer Vertices
    ///
    /// During BFS, `p[v]` is set only for **inner** vertices — the outer
    /// vertex that discovered them.  For outer vertices that are the *mate*
    /// of an inner vertex, `p` is never set (it stays `usize::MAX`).  The
    /// previous implementation used `p[curr]` for the outer vertex, which
    /// returned `usize::MAX` and produced a single-element path `[curr]`.
    /// This meant `phase2_augment` could only succeed for length-1 paths
    /// (where the bridge endpoints are themselves the exposed roots).
    ///
    /// ## Blossom Handling
    ///
    /// `mark_blossom` overwrites `p[v]` for **outer** blossom vertices to
    /// create a shortcut through the blossom.  This function avoids those
    /// shortcuts because it uses `p[inner]` (the BFS parent of the inner
    /// partner), which `mark_blossom` never modifies.  The path therefore
    /// follows the original BFS tree through the blossom interior, producing
    /// a correctly alternating path.
    fn trace_to_root(&self, mut curr: usize) -> Vec<usize> {
        let mut path = vec![curr];
        loop {
            // Step 1: from outer vertex, follow the MATCHED edge to inner partner.
            // In an alternating path starting at an outer vertex, the edge
            // toward the root is the matched edge.
            let partner = match self.mate[curr] {
                Some(p) => p,
                None => break, // curr is an exposed root — path is complete
            };
            path.push(partner);

            // Step 2: from inner vertex, follow p[partner] — the UNMATCHED
            // edge that originally discovered `partner` during BFS.  This
            // leads back to the next outer vertex toward the root.
            let parent = self.p[partner];
            if parent == usize::MAX {
                break; // partner was discovered by a root directly
            }
            path.push(parent);
            curr = parent;
        }
        path
    }

    pub fn augment_single_path(&mut self, graph: &Graph) -> usize {
        self.dsu.reset(self.n);
        self.label.fill(-1);
        self.p.fill(usize::MAX);
        self.orig_p.fill(usize::MAX);

        let mut q = VecDeque::new();

        for i in 0..self.n {
            if self.mate[i].is_none() {
                let mut end_node = usize::MAX;
                self.label.fill(-1);
                self.p.fill(usize::MAX);
                self.orig_p.fill(usize::MAX);
                self.dsu.reset(self.n);

                q.clear();
                q.push_back(i);
                self.label[i] = 0;

                'bfs: while let Some(u) = q.pop_front() {
                    for &edge_idx in graph.adj_edges(u) {
                        let edge = &graph.edges[edge_idx];
                        let v = if edge.u == u { edge.v } else { edge.u };

                        if self.dsu.find(u) == self.dsu.find(v) || self.mate[u] == Some(v) {
                            continue;
                        }

                        if self.label[v] == 0 {
                            let base_lca = self.lca(u, v);
                            self.mark_blossom(u, base_lca, v, &mut q, 0);
                            self.mark_blossom(v, base_lca, u, &mut q, 0);
                        } else if self.label[v] == -1 {
                            self.p[v] = u;
                            self.orig_p[v] = u;

                            if self.mate[v].is_none() {
                                end_node = v;
                                break 'bfs;
                            }

                            let match_v = self.mate[v].unwrap();
                            self.label[v] = 1;
                            self.label[match_v] = 0;
                            q.push_back(match_v);
                        }
                    }
                }

                if end_node != usize::MAX {
                    while end_node != usize::MAX {
                        let curr_p = self.p[end_node];
                        let next_node = if curr_p != usize::MAX {
                            self.mate[curr_p]
                        } else {
                            None
                        };
                        self.mate[end_node] = Some(curr_p);
                        self.mate[curr_p] = Some(end_node);
                        end_node = next_node.unwrap_or(usize::MAX);
                    }
                    return 1;
                }
            }
        }
        0
    }

    fn is_valid_augmenting_path(&self, graph: &Graph, path: &[usize]) -> bool {
        if path.len() < 2 || path.len() % 2 != 0 {
            return false;
        }
        if self.mate[path[0]].is_some() || self.mate[*path.last().unwrap()].is_some() {
            return false;
        }
        let mut seen = vec![false; self.n];
        for &v in path {
            if v >= self.n || seen[v] {
                return false;
            }
            seen[v] = true;
        }
        for (i, pair) in path.windows(2).enumerate() {
            let (u, v) = (pair[0], pair[1]);
            if graph.edge_index(u, v).is_none() {
                return false;
            }
            let is_matched = self.mate[u] == Some(v);
            let expected_matched = i % 2 == 1;
            if is_matched != expected_matched {
                return false;
            }
        }
        true
    }

    fn apply_path_augmentation(&mut self, path: &[usize]) {
        for chunk in path.chunks_exact(2) {
            let u = chunk[0];
            let v = chunk[1];
            self.mate[u] = Some(v);
            self.mate[v] = Some(u);
        }
    }

    fn build_matching_result(&self, graph: &Graph, start_time: Instant) -> MatchingResult {
        let mut edges = Vec::new();
        let mut scanned = vec![false; self.n];

        for u in 0..self.n {
            if scanned[u] {
                continue;
            }
            if let Some(v) = self.mate[u] {
                if let Some(edge_idx) = graph.edge_index(u, v) {
                    edges.push(edge_idx);
                    scanned[u] = true;
                    scanned[v] = true;
                }
            }
        }

        let edge_count = edges.len();
        let total_weight = edges.iter().map(|&i| graph.edges[i].weight_or_one()).sum();

        MatchingResult {
            edges,
            total_weight,
            is_perfect: edge_count * 2 == self.n,
            metadata: MatchingMetadata {
                iterations: self.iterations,
                augmentations: self.augmentations,
                blossoms_created: self.blossoms_created,
                blossoms_expanded: 0,
                dual_updates: 0,
                runtime_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            },
        }
    }
}

/// Convenience function: solves maximum cardinality matching using the
/// Gabow-inspired heuristic phased solver.
pub fn gabow_heuristic_matching(graph: &Graph) -> MatchingResult {
    let mut solver = GabowHeuristic::new(graph);
    solver.max_cardinality_matching(graph)
}
