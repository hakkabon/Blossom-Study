//! Edmonds' Blossom Algorithm for Maximum Cardinality and Minimum Weight Perfect Matching
//!
//! Based on Gerards (1995) "Matching" chapter in Network Models,
//! and the classic blossom contraction technique for general graphs.

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::{Edge, Graph};
use std::collections::VecDeque;

const EVEN: i32 = 2;
const ODD: i32 = 1;
const UNLABELED: i32 = 0;
const EPS: f64 = 1e-9;

#[inline]
fn greater(a: f64, b: f64) -> bool {
    a - b > EPS
}

/// Edmonds' Blossom Algorithm implementation for general matching.
pub struct EdmondsAlgorithm {
    // Graph representation
    n: usize,
    m: usize,
    edges: Vec<Edge>,
    adj_list: Vec<Vec<usize>>,
    edge_weights: Vec<f64>,

    // Blossom tracking
    outer: Vec<usize>,        // outermost blossom containing vertex
    deep: Vec<Vec<usize>>,    // all original vertices inside blossom
    shallow: Vec<Vec<usize>>, // immediate children of blossom
    tip: Vec<usize>,          // tip (base) of blossom
    active: Vec<bool>,        // whether blossom is active

    // Forest / alternating tree
    label: Vec<i32>,  // EVEN, ODD, UNLABELED
    forest: Vec<i32>, // parent in alternating forest (-1 for root/none)
    root: Vec<usize>, // root of tree

    // Matching
    mate: Vec<i32>,      // matched vertex (-1 for unmatched)
    mate_edge: Vec<i32>, // matched edge index

    // Dual variables (for weighted matching / slack)
    dual: Vec<f64>,
    slack: Vec<f64>,

    // Blocked blossoms (dual > 0)
    blocked: Vec<bool>,

    // Free blossom index pool
    free_indices: Vec<usize>,

    // BFS queue
    queue: VecDeque<usize>,

    // Visited tracking
    visited: Vec<bool>,

    // Execution metadata
    iterations: usize,
    blossoms_created: usize,
    blossoms_expanded: usize,
    dual_updates: usize,
}

impl EdmondsAlgorithm {
    /// Creates a new `EdmondsAlgorithm` instance initialized for the given graph.
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        let m = graph.edges.len();
        let max_vertices = 2 * n;

        // Build adjacency structures
        let mut adj_list = vec![Vec::new(); n];
        let mut edge_weights = vec![0.0; m];

        for (idx, e) in graph.edges.iter().enumerate() {
            if e.u < n && e.v < n {
                adj_list[e.u].push(idx);
                adj_list[e.v].push(idx);
                edge_weights[idx] = e.weight_or_one();
            }
        }

        Self {
            n,
            m,
            edges: graph.edges.clone(),
            adj_list,
            edge_weights,
            outer: vec![0; max_vertices],
            deep: vec![Vec::new(); max_vertices],
            shallow: vec![Vec::new(); max_vertices],
            tip: vec![0; max_vertices],
            active: vec![false; max_vertices],
            label: vec![UNLABELED; max_vertices],
            forest: vec![-1; max_vertices],
            root: vec![0; max_vertices],
            mate: vec![-1; max_vertices],
            mate_edge: vec![-1; max_vertices],
            dual: vec![0.0; max_vertices],
            slack: vec![0.0; m],
            blocked: vec![false; max_vertices],
            free_indices: Vec::new(),
            queue: VecDeque::new(),
            visited: vec![false; max_vertices],
            iterations: 0,
            blossoms_created: 0,
            blossoms_expanded: 0,
            dual_updates: 0,
        }
    }

    fn reset_state(&mut self) {
        for i in 0..2 * self.n {
            self.forest[i] = -1;
            self.root[i] = i;
            if i >= self.n && self.active[i] && self.outer[i] == i {
                self.destroy_blossom(i);
            }
        }
        self.visited.fill(false);
        self.queue.clear();

        for i in 0..self.n {
            if self.mate[self.outer[i]] == -1 {
                self.label[self.outer[i]] = EVEN;
                if !self.visited[self.outer[i]] {
                    self.queue.push_back(i);
                    self.visited[self.outer[i]] = true;
                }
            } else {
                self.label[self.outer[i]] = UNLABELED;
            }
        }
    }

    fn clear_all(&mut self) {
        self.free_indices.clear();
        for i in self.n..2 * self.n {
            self.free_indices.push(i);
        }

        for i in 0..2 * self.n {
            self.outer[i] = i;
            self.deep[i].clear();
            if i < self.n {
                self.deep[i].push(i);
            }
            self.shallow[i].clear();
            self.active[i] = i < self.n;
            self.label[i] = UNLABELED;
            self.forest[i] = -1;
            self.root[i] = i;
            self.blocked[i] = false;
            self.dual[i] = 0.0;
            self.mate[i] = -1;
            self.mate_edge[i] = -1;
            self.tip[i] = i;
        }
        self.slack.fill(0.0);
        self.iterations = 0;
        self.blossoms_created = 0;
        self.blossoms_expanded = 0;
        self.dual_updates = 0;
    }

    fn get_free_index(&mut self) -> usize {
        if let Some(idx) = self.free_indices.pop() {
            idx
        } else {
            let new_idx = self.outer.len();
            self.outer.push(0);
            self.deep.push(Vec::new());
            self.shallow.push(Vec::new());
            self.tip.push(0);
            self.active.push(false);
            self.label.push(UNLABELED);
            self.forest.push(-1);
            self.root.push(0);
            self.blocked.push(false);
            self.dual.push(0.0);
            self.mate.push(-1);
            self.mate_edge.push(-1);
            self.visited.push(false);
            new_idx
        }
    }

    #[inline]
    fn is_edge_blocked_idx(&self, idx: usize) -> bool {
        greater(self.slack[idx], 0.0)
    }

    /// Grows the alternating forest and performs augmentations or blossom contractions.
    /// Returns `true` if an augmenting path was found during this stage.
    fn grow(&mut self) -> bool {
        self.reset_state();
        let mut augmented = false;

        'bfs: while let Some(v) = self.queue.pop_front() {
            self.iterations += 1;

            let w = self.outer[v];
            let deep_w = self.deep[w].clone();

            'scan_deep: for &di in &deep_w {
                for &edge_idx in &self.adj_list[di] {
                    let u = if self.edges[edge_idx].u == di {
                        self.edges[edge_idx].v
                    } else {
                        self.edges[edge_idx].u
                    };

                    if self.is_edge_blocked_idx(edge_idx) {
                        continue;
                    }

                    let ou = self.outer[u];
                    if self.label[ou] == ODD {
                        continue;
                    }

                    if self.label[ou] != EVEN {
                        let vm = self.mate[ou];
                        if vm < 0 {
                            continue;
                        }
                        self.forest[ou] = w as i32;
                        self.label[ou] = ODD;
                        self.root[ou] = self.root[w];
                        let outer_vm = self.outer[vm as usize];
                        self.forest[outer_vm] = ou as i32;
                        self.label[outer_vm] = EVEN;
                        self.root[outer_vm] = self.root[w];

                        if !self.visited[outer_vm] {
                            self.queue.push_back(vm as usize);
                            self.visited[outer_vm] = true;
                        }
                    } else if self.root[ou] != self.root[w] {
                        self.augment(w, u);
                        augmented = true;
                        self.reset_state();
                        break 'bfs;
                    } else if w != ou {
                        self.blossom(w, u);
                        break 'scan_deep;
                    }
                }
            }
        }

        augmented
    }

    /// Contracts an odd alternating cycle between `u` and `v` into a new blossom.
    fn blossom(&mut self, u: usize, v: usize) -> usize {
        let t = self.get_free_index();
        self.blossoms_created += 1;

        let max_idx = t.max(self.outer.len() - 1);
        let mut is_in_path = vec![false; max_idx + 1];

        // Trace path from u to root
        let mut u_ = u;
        while u_ != usize::MAX {
            let ou = self.outer[u_];
            if ou < is_in_path.len() {
                is_in_path[ou] = true;
            }
            let parent = self.forest[ou];
            u_ = if parent >= 0 {
                parent as usize
            } else {
                usize::MAX
            };
        }

        // Find LCA with path from v
        let mut v_ = self.outer[v];
        while v_ < is_in_path.len() && !is_in_path[v_] {
            let parent = self.forest[v_];
            if parent < 0 {
                break;
            }
            v_ = self.outer[parent as usize];
        }
        self.tip[t] = v_;

        // Build odd circuit in true cyclic order: tip -> ... -> u -> v -> ... -> tip
        let mut path_u = Vec::new();
        u_ = self.outer[u];
        while u_ != self.tip[t] {
            path_u.push(u_);
            let parent = self.forest[u_];
            if parent < 0 {
                break;
            }
            u_ = self.outer[parent as usize];
        }

        let mut path_v = Vec::new();
        v_ = self.outer[v];
        while v_ != self.tip[t] {
            path_v.push(v_);
            let parent = self.forest[v_];
            if parent < 0 {
                break;
            }
            v_ = self.outer[parent as usize];
        }

        self.shallow[t].clear();
        self.deep[t].clear();

        self.shallow[t].push(self.tip[t]);
        for &x in path_u.iter().rev() {
            self.shallow[t].push(x);
        }
        for &x in &path_v {
            self.shallow[t].push(x);
        }

        // Build deep vertices and update outer pointers
        let shallow_t = self.shallow[t].clone();
        for &x in &shallow_t {
            self.outer[x] = t;
            let deep_x = self.deep[x].clone();
            for &y in &deep_x {
                self.deep[t].push(y);
                self.outer[y] = t;
            }
        }

        self.forest[t] = self.forest[self.tip[t]];
        self.label[t] = EVEN;
        self.root[t] = self.root[self.tip[t]];
        self.active[t] = true;
        self.outer[t] = t;
        self.mate[t] = self.mate[self.tip[t]];
        self.mate_edge[t] = self.mate_edge[self.tip[t]];

        if !self.visited[t] {
            self.queue.push_back(self.tip[t]);
            self.visited[t] = true;
        }

        t
    }

    /// Expands a contracted blossom vertex back to its constituents.
    fn expand(&mut self, u: usize, expand_blocked: bool) {
        if u < self.n || (self.blocked[u] && !expand_blocked) {
            return;
        }

        self.blossoms_expanded += 1;
        let mate_u = self.mate[u];
        if mate_u < 0 {
            return;
        }
        let v = self.outer[mate_u as usize];

        // Find minimum index edge connecting u and v
        let mut min_idx = self.m;
        let mut p = usize::MAX;
        let mut q = usize::MAX;

        let mut in_v = vec![false; self.n];
        for &vertex in &self.deep[v] {
            in_v[vertex] = true;
        }
        for &di in &self.deep[u] {
            for &idx in &self.adj_list[di] {
                let edge = &self.edges[idx];
                let dj = if edge.u == di { edge.v } else { edge.u };
                if in_v[dj] && !self.is_edge_blocked_idx(idx) && idx < min_idx {
                    min_idx = idx;
                    p = di;
                    q = dj;
                }
            }
        }

        self.mate[u] = q as i32;
        self.mate[v] = p as i32;
        self.mate_edge[u] = min_idx as i32;
        self.mate_edge[v] = min_idx as i32;

        // Find new tip
        let mut found = false;
        while !found && !self.shallow[u].is_empty() {
            let si = self.shallow[u][0];
            for &x in &self.deep[si] {
                if x == p {
                    found = true;
                    break;
                }
            }
            if !found {
                let si = self.shallow[u].remove(0);
                self.shallow[u].push(si);
            }
        }

        if !self.shallow[u].is_empty() {
            // Adjust mates along circuit
            self.mate[self.shallow[u][0]] = self.mate[u];
            let mut i = 1;
            while i < self.shallow[u].len() {
                let a = self.shallow[u][i];
                let b = self.shallow[u][(i + 1) % self.shallow[u].len()];
                self.mate[a] = b as i32;
                self.mate[b] = a as i32;
                i += 2;
            }
        }

        // Deactivate blossom
        let shallow_u = self.shallow[u].clone();
        for &s in &shallow_u {
            self.outer[s] = s;
            for &x in &self.deep[s] {
                self.outer[x] = s;
            }
        }
        self.active[u] = false;
        self.free_indices.push(u);

        // Recursively expand
        for s in shallow_u {
            self.expand(s, expand_blocked);
        }
    }

    /// Augments the current matching along the path between `u` and `v`.
    fn augment(&mut self, u: usize, v: usize) {
        let mut p = self.outer[u];
        let mut q = self.outer[v];
        let outv = q;
        let mut fp = self.forest[p];

        self.mate[p] = q as i32;
        self.mate[q] = p as i32;
        self.expand(p, false);
        self.expand(q, false);

        while fp >= 0 {
            q = self.outer[fp as usize];
            let fq = self.forest[q];
            if fq < 0 {
                break;
            }
            p = self.outer[fq as usize];
            fp = self.forest[p];
            self.mate[p] = q as i32;
            self.mate[q] = p as i32;
            self.expand(p, false);
            self.expand(q, false);
        }

        p = outv;
        fp = self.forest[p];
        while fp >= 0 {
            q = self.outer[fp as usize];
            let fq = self.forest[q];
            if fq < 0 {
                break;
            }
            p = self.outer[fq as usize];
            fp = self.forest[p];
            self.mate[p] = q as i32;
            self.mate[q] = p as i32;
            self.expand(p, false);
            self.expand(q, false);
        }
    }

    fn destroy_blossom(&mut self, t: usize) {
        if t < self.n || (self.blocked[t] && greater(self.dual[t], 0.0)) {
            return;
        }

        let shallow_t = self.shallow[t].clone();
        for s in shallow_t {
            self.outer[s] = s;
            let deep_s = self.deep[s].clone();
            for x in deep_s {
                self.outer[x] = s;
            }
            self.destroy_blossom(s);
        }
        self.active[t] = false;
        self.blocked[t] = false;
        self.free_indices.push(t);
        self.mate[t] = -1;
    }

    fn retrieve_matching(&mut self) -> Vec<usize> {
        // Expand all active blossoms
        for i in 0..2 * self.n {
            if self.active[i] && self.mate[i] != -1 && self.outer[i] == i {
                self.expand(i, true);
            }
        }

        let mut matching = Vec::new();
        for i in 0..self.m {
            let u = self.edges[i].u;
            let v = self.edges[i].v;
            if u < self.n && v < self.n && self.mate[u] == v as i32 {
                matching.push(i);
            }
        }
        matching
    }

    /// Computes maximum cardinality matching on the graph.
    pub fn max_cardinality(&mut self, graph: &Graph) -> MatchingResult {
        assert_eq!(graph.n, self.n, "solver called with a different graph size");
        assert_eq!(
            graph.edges, self.edges,
            "solver called with different edges"
        );
        let start = std::time::Instant::now();
        self.clear_all();

        // Keep growing alternating paths until no augmenting path is found
        let mut max_passes = self.n;
        while max_passes > 0 {
            max_passes -= 1;
            let found_path = self.grow();
            if !found_path {
                break;
            }
        }

        let edges = self.retrieve_matching();
        let augmentations = edges.len();
        let total_weight: f64 = edges.iter().map(|&i| self.edge_weights[i]).sum();
        let is_perfect = edges.len() * 2 == self.n;

        MatchingResult {
            edges,
            total_weight,
            is_perfect,
            metadata: MatchingMetadata {
                iterations: self.iterations,
                augmentations,
                blossoms_created: self.blossoms_created,
                blossoms_expanded: self.blossoms_expanded,
                dual_updates: self.dual_updates,
                runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
        }
    }

    /// Solves minimum weight perfect matching on `graph`.
    pub fn min_weight_perfect(&mut self, graph: &Graph) -> Result<MatchingResult, String> {
        crate::algorithms::minimum_weight_perfect_matching(graph)
    }

    /// Algorithm name
    pub fn name(&self) -> &'static str {
        "edmonds"
    }
}
