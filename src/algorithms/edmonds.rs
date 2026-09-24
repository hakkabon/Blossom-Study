//! Edmonds' Blossom Algorithm for Maximum Cardinality and Minimum Weight Perfect Matching
//!
//! Based on Gerards (1995) "Matching" chapter in Network Models,
//! and the C++ implementation from Minimum-Cost-Perfect-Matching.

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::{Edge, Graph};
use std::collections::VecDeque;

const EVEN: i32 = 2;
const ODD: i32 = 1;
const UNLABELED: i32 = 0;
const EPS: f64 = 1e-9;

fn greater(a: f64, b: f64) -> bool {
    a - b > EPS
}

/// Edmonds' Blossom Algorithm implementation
pub struct EdmondsAlgorithm {
    // Graph reference
    n: usize,
    m: usize,
    edges: Vec<Edge>,
    adj_list: Vec<Vec<usize>>,
    adj_mat: Vec<Vec<bool>>,
    edge_index: Vec<Vec<usize>>,
    edge_weights: Vec<f64>,

    // Blossom tracking
    outer: Vec<usize>,        // outermost blossom containing vertex
    deep: Vec<Vec<usize>>,    // all original vertices inside blossom
    shallow: Vec<Vec<usize>>, // immediate children of blossom
    tip: Vec<usize>,          // tip of blossom
    active: Vec<bool>,        // whether blossom is active

    // Forest/alternating tree
    label: Vec<i32>,  // EVEN, ODD, UNLABELED
    forest: Vec<i32>, // parent in alternating forest
    root: Vec<usize>, // root of tree

    // Matching
    mate: Vec<i32>,      // matched vertex
    mate_edge: Vec<i32>, // matched edge index

    // Dual variables (for weighted matching)
    dual: Vec<f64>,  // dual variables for blossoms
    slack: Vec<f64>, // slack for edges

    // Blocked blossoms (dual > 0)
    blocked: Vec<bool>,

    // Free blossom index pool
    free_indices: Vec<usize>,

    // BFS queue
    queue: VecDeque<usize>,

    // Visited tracking
    visited: Vec<bool>,

    // Metadata
    iterations: usize,
    blossoms_created: usize,
    blossoms_expanded: usize,
    dual_updates: usize,
}

impl EdmondsAlgorithm {
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        let m = graph.edges.len();
        let max_vertices = 2 * n;

        // Build adjacency structures
        let mut adj_list = vec![Vec::new(); n];
        let mut adj_mat = vec![vec![false; n]; n];
        let mut edge_index = vec![vec![usize::MAX; n]; n];
        let mut edge_weights = vec![0.0; m];

        for (idx, e) in graph.edges.iter().enumerate() {
            adj_list[e.u].push(idx);
            adj_list[e.v].push(idx);
            adj_mat[e.u][e.v] = true;
            adj_mat[e.v][e.u] = true;
            edge_index[e.u][e.v] = idx;
            edge_index[e.v][e.u] = idx;
            edge_weights[idx] = e.weight.unwrap_or(1.0);
        }

        Self {
            n,
            m,
            edges: graph.edges.clone(),
            adj_list,
            adj_mat,
            edge_index,
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
    }

    fn get_free_index(&mut self) -> usize {
        if let Some(idx) = self.free_indices.pop() {
            idx
        } else {
            // Expand arrays if needed
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

    fn is_edge_blocked_idx(&self, idx: usize) -> bool {
        greater(self.slack[idx], 0.0)
    }

    fn is_edge_blocked(&self, u: usize, v: usize) -> bool {
        let idx = self.edge_index[u][v];
        if idx != usize::MAX {
            self.is_edge_blocked_idx(idx)
        } else {
            true
        }
    }

    fn is_adjacent(&self, u: usize, v: usize) -> bool {
        self.adj_mat[u][v] && !self.is_edge_blocked(u, v)
    }

    fn grow(&mut self) -> bool {
        self.reset_state();
        let mut iteration = 0;
        const MAX_ITER: usize = 1000;

        while let Some(v) = self.queue.pop_front() {
            if iteration > MAX_ITER {
                break;
            }
            iteration += 1;
            let w = self.outer[v];

            for &di in &self.deep[w].clone() {
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
                        self.forest[ou] = w as i32;
                        self.label[ou] = ODD;
                        self.root[ou] = self.root[w];
                        self.forest[self.outer[vm as usize]] = ou as i32;
                        self.label[self.outer[vm as usize]] = EVEN;
                        self.root[self.outer[vm as usize]] = self.root[w];

                        if !self.visited[self.outer[vm as usize]] {
                            self.queue.push_back(vm as usize);
                            self.visited[self.outer[vm as usize]] = true;
                        }
                    } else if self.root[ou] != self.root[w] {
                        self.augment(w, u);
                        self.reset_state();
                        break;
                    } else if w != u {
                        let _ = self.blossom(w, u);
                        break;
                    }
                }
            }
        }

        let mut perfect = true;
        for i in 0..self.n {
            if self.mate[self.outer[i]] == -1 {
                perfect = false;
                break;
            }
        }
        perfect
    }

    fn blossom(&mut self, u: usize, v: usize) -> usize {
        let t = self.get_free_index();
        self.blossoms_created += 1;

        // Ensure arrays are large enough for the new index
        let max_idx = t.max(self.outer.len() - 1);
        let mut is_in_path = vec![false; max_idx + 1];
        let mut u_ = u;
        while u_ != usize::MAX {
            let ou = self.outer[u_];
            if ou < is_in_path.len() {
                is_in_path[ou] = true;
            }
            u_ = self.forest[ou] as usize;
        }

        let mut v_ = self.outer[v];
        while v_ < is_in_path.len() && !is_in_path[v_] {
            v_ = self.outer[self.forest[v_] as usize];
        }
        self.tip[t] = v_;

        // Build odd circuit
        let mut circuit = Vec::new();
        u_ = self.outer[u];
        circuit.push(u_);
        while u_ != self.tip[t] {
            u_ = self.outer[self.forest[u_] as usize];
            circuit.push(u_);
        }

        self.shallow[t].clear();
        self.deep[t].clear();
        for &x in &circuit {
            self.shallow[t].push(x);
        }

        v_ = self.outer[v];
        while v_ != self.tip[t] {
            self.shallow[t].push(v_);
            v_ = self.outer[self.forest[v_] as usize];
        }

        // Build deep and update outer
        for &x in &self.shallow[t].clone() {
            self.outer[x] = t;
            for &y in &self.deep[x].clone() {
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

        t
    }

    fn expand(&mut self, u: usize, expand_blocked: bool) {
        let v = self.outer[self.mate[u] as usize];

        // Find minimum index edge connecting u and v
        let mut min_idx = self.m;
        let mut p = usize::MAX;
        let mut q = usize::MAX;

        for &di in &self.deep[u] {
            for &dj in &self.deep[v] {
                if self.is_adjacent(di, dj) {
                    let idx = self.edge_index[di][dj];
                    if idx < min_idx {
                        min_idx = idx;
                        p = di;
                        q = dj;
                    }
                }
            }
        }

        self.mate[u] = q as i32;
        self.mate[v] = p as i32;
        self.mate_edge[u] = min_idx as i32;
        self.mate_edge[v] = min_idx as i32;

        if u < self.n || (self.blocked[u] && !expand_blocked) {
            return;
        }

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

        // Deactivate blossom
        for &s in &self.shallow[u] {
            self.outer[s] = s;
            for &x in &self.deep[s] {
                self.outer[x] = s;
            }
        }
        self.active[u] = false;
        self.free_indices.push(u);

        // Recursively expand
        let shallow_u = self.shallow[u].clone();
        for s in shallow_u {
            self.expand(s, expand_blocked);
        }
    }

    fn augment(&mut self, u: usize, v: usize) {
        let mut p = self.outer[u];
        let mut q = self.outer[v];
        let outv = q;
        let mut fp = self.forest[p];

        self.mate[p] = q as i32;
        self.mate[q] = p as i32;
        self.expand(p, false);
        self.expand(q, false);

        while fp != -1 {
            q = self.outer[self.forest[p] as usize];
            p = self.outer[self.forest[q] as usize];
            fp = self.forest[p];
            self.mate[p] = q as i32;
            self.mate[q] = p as i32;
            self.expand(p, false);
            self.expand(q, false);
        }

        p = outv;
        fp = self.forest[p];
        while fp != -1 {
            q = self.outer[self.forest[p] as usize];
            p = self.outer[self.forest[q] as usize];
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

        for s in self.shallow[t].clone() {
            self.outer[s] = s;
            for x in self.deep[s].clone() {
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
            if self.mate[u] == v as i32 {
                matching.push(i);
            }
        }
        matching
    }
}

impl EdmondsAlgorithm {
    pub fn name(&self) -> &'static str {
        "edmonds"
    }

    pub fn max_cardinality(&mut self, _graph: &Graph) -> MatchingResult {
        let start = std::time::Instant::now();
        self.clear_all();

        // Keep growing until no more augmenting paths found
        let mut perfect = false;
        let mut grow_calls = 0;
        while !perfect {
            grow_calls += 1;
            if grow_calls > 100 {
                break;
            }
            perfect = self.grow();
        }

        let edges = self.retrieve_matching();

        let total_weight: f64 = edges.iter().map(|&i| self.edge_weights[i]).sum();

        MatchingResult {
            edges,
            total_weight,
            is_perfect: perfect,
            metadata: MatchingMetadata {
                iterations: self.iterations,
                blossoms_created: self.blossoms_created,
                blossoms_expanded: self.blossoms_expanded,
                dual_updates: self.dual_updates,
                runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
        }
    }

    pub fn min_weight_perfect(&mut self, _graph: &Graph) -> Result<MatchingResult, String> {
        crate::algorithms::minimum_weight_perfect_matching(_graph)
    }
}
