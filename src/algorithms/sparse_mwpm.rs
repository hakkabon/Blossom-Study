//! Experimental sparse event-driven Minimum-Weight Perfect Matching (MWPM)
//!
//! This module provides an alternative, sub-cubic implementation of the
//! Primal-Dual Blossom Algorithm for Minimum-Weight Perfect Matching on general graphs.
//!
//! The implementation is designed to avoid dense matrices by using adjacency
//! lists and component-level incident-edge maps.  Its intended event-driven
//! work bound is `O(VE)`, but this educational version has not yet been given a
//! formal amortized-complexity proof.  In particular, contraction rebuilds maps
//! and should be benchmarked before describing the implementation as strictly
//! sub-cubic or strictly `O(V + E)` in all intermediate states.

use crate::algorithms::mwpm::{WeightScaler, filter_edges};
use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// An internal validated and scaled undirected edge for sparse primal-dual matching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SparseEdge {
    pub u: u32,
    pub v: u32,
    pub original_index: usize,
    pub weight: i128,
}

impl SparseEdge {
    /// Construct a 1-indexed sparse edge, rejecting endpoints that cannot be
    /// represented by the solver's `u32` vertex identifiers.
    #[inline]
    pub fn new(u: usize, v: usize, original_index: usize, weight: i128) -> Result<Self, String> {
        if u == 0 || v == 0 {
            return Err("sparse edge endpoints must use 1-based indices".to_string());
        }
        Ok(Self {
            u: u32::try_from(u).map_err(|_| format!("sparse edge endpoint {u} exceeds u32"))?,
            v: u32::try_from(v).map_err(|_| format!("sparse edge endpoint {v} exceeds u32"))?,
            original_index,
            weight,
        })
    }

    #[inline]
    pub fn reversed(self) -> Self {
        Self {
            u: self.v,
            v: self.u,
            original_index: self.original_index,
            weight: self.weight,
        }
    }
}

/// Sparse Primal-Dual Blossom solver state.
pub struct SparseBlossomSolver {
    n: usize,
    n_x: usize,
    adj: Vec<Vec<SparseEdge>>,
    lab: Vec<i128>,
    mate: Vec<usize>,
    slack: Vec<usize>,
    slack_edge: Vec<Option<SparseEdge>>,
    st: Vec<usize>,
    pa: Vec<usize>,
    s: Vec<i8>,
    vis: Vec<usize>,
    flower: Vec<Vec<usize>>,
    parent: Vec<usize>,
    best_edge: Vec<HashMap<usize, SparseEdge>>,
    q: VecDeque<usize>,
    t: usize,
    pub iterations: usize,
    pub blossoms_created: usize,
    pub blossoms_expanded: usize,
    pub dual_updates: usize,
}

impl SparseBlossomSolver {
    /// Creates a new solver instance with sparse adjacency lists.
    pub fn new(n: usize, adj: Vec<Vec<SparseEdge>>) -> Self {
        let sz = 2 * n + 1;
        Self {
            n,
            n_x: n,
            adj,
            lab: vec![0; sz],
            mate: vec![0; sz],
            slack: vec![0; sz],
            slack_edge: vec![None; sz],
            st: (0..sz).collect(),
            pa: vec![0; sz],
            s: vec![-1; sz],
            vis: vec![0; sz],
            flower: vec![Vec::new(); sz],
            parent: vec![0; sz],
            best_edge: (0..sz).map(|_| HashMap::new()).collect(),
            q: VecDeque::new(),
            t: 0,
            iterations: 0,
            blossoms_created: 0,
            blossoms_expanded: 0,
            dual_updates: 0,
        }
    }

    #[inline]
    fn get_edge(&self, u: usize, v: usize) -> Option<SparseEdge> {
        self.best_edge.get(u).and_then(|m| m.get(&v)).copied()
    }

    #[inline]
    fn set_edge(&mut self, u: usize, v: usize, e: SparseEdge) {
        self.best_edge[u].insert(v, e);
        self.best_edge[v].insert(u, e.reversed());
    }

    #[inline]
    fn e_delta(&self, e: SparseEdge) -> i128 {
        self.lab[e.u as usize] + self.lab[e.v as usize] - 2 * e.weight
    }

    fn update_slack(&mut self, e: SparseEdge, x: usize) {
        let cur_slack = self.slack_edge[x]
            .map(|old| self.e_delta(old))
            .unwrap_or(i128::MAX);
        let new_slack = self.e_delta(e);
        if self.slack[x] == 0 || new_slack < cur_slack {
            self.slack[x] = e.u as usize;
            self.slack_edge[x] = Some(e);
        }
    }

    fn set_slack(&mut self, x: usize) {
        self.slack[x] = 0;
        self.slack_edge[x] = None;
        let incident_edges: Vec<SparseEdge> = self.best_edge[x].values().copied().collect();
        for e in incident_edges {
            let u = e.u as usize;
            let v = e.v as usize;
            let tree_endpoint = if self.s[self.st[u]] == 0 && self.st[u] != x {
                Some(e)
            } else if self.s[self.st[v]] == 0 && self.st[v] != x {
                Some(e.reversed())
            } else {
                None
            };
            if let Some(edge) = tree_endpoint
                && edge.weight > 0
            {
                self.update_slack(edge, x);
            }
        }
    }

    fn q_push(&mut self, x: usize) {
        if x <= self.n {
            self.q.push_back(x);
        } else {
            let mut i = 0;
            while i < self.flower[x].len() {
                let c = self.flower[x][i];
                self.q_push(c);
                i += 1;
            }
        }
    }

    fn set_st(&mut self, x: usize, b: usize) {
        self.st[x] = b;
        if x > self.n {
            let mut i = 0;
            while i < self.flower[x].len() {
                let c = self.flower[x][i];
                self.set_st(c, b);
                i += 1;
            }
        }
    }

    fn child_of(&self, mut node: usize, b: usize) -> usize {
        while self.parent[node] != b && self.parent[node] != 0 {
            node = self.parent[node];
        }
        node
    }

    fn get_pr(&mut self, b: usize, xr: usize) -> usize {
        let pr = self.flower[b]
            .iter()
            .position(|&x| x == xr)
            .expect("xr must be a member of blossom cycle");
        if pr % 2 == 1 {
            let len = self.flower[b].len();
            self.flower[b][1..].reverse();
            len - pr
        } else {
            pr
        }
    }

    fn set_match(&mut self, u: usize, v: usize) {
        let e = self
            .get_edge(u, v)
            .expect("connecting edge between matched components must exist");
        self.mate[u] = e.v as usize;
        if u > self.n {
            let xr = self.child_of(e.u as usize, u);
            let pr = self.get_pr(u, xr);
            let mut i = 0;
            while i < pr {
                let a = self.flower[u][i];
                let b = self.flower[u][i ^ 1];
                self.set_match(a, b);
                i += 1;
            }
            self.set_match(xr, v);
            self.flower[u].rotate_left(pr);
        }
    }

    fn augment(&mut self, mut u: usize, mut v: usize) {
        loop {
            let xnv = self.st[self.mate[u]];
            self.set_match(u, v);
            if xnv == 0 {
                return;
            }
            let next_u = self.st[self.pa[xnv]];
            self.set_match(xnv, next_u);
            u = next_u;
            v = xnv;
        }
    }

    fn get_lca(&mut self, mut u: usize, mut v: usize) -> usize {
        self.t += 1;
        loop {
            if u == 0 && v == 0 {
                return 0;
            }
            if u != 0 {
                if self.vis[u] == self.t {
                    return u;
                }
                self.vis[u] = self.t;
                let m = self.st[self.mate[u]];
                u = if m != 0 { self.st[self.pa[m]] } else { 0 };
            }
            std::mem::swap(&mut u, &mut v);
        }
    }

    fn add_blossom(&mut self, u: usize, lca: usize, v: usize) {
        let mut b = self.n + 1;
        while b <= self.n_x && self.st[b] != 0 {
            b += 1;
        }
        if b > self.n_x {
            self.n_x += 1;
        }
        self.blossoms_created += 1;
        self.lab[b] = 0;
        self.s[b] = 0;
        self.mate[b] = self.mate[lca];
        self.flower[b].clear();
        self.flower[b].push(lca);

        let mut x = u;
        while x != lca {
            let y = self.st[self.mate[x]];
            self.flower[b].push(x);
            self.flower[b].push(y);
            self.q_push(y);
            x = self.st[self.pa[y]];
        }
        self.flower[b][1..].reverse();

        let mut x = v;
        while x != lca {
            let y = self.st[self.mate[x]];
            self.flower[b].push(x);
            self.flower[b].push(y);
            self.q_push(y);
            x = self.st[self.pa[y]];
        }

        for &member in &self.flower[b] {
            self.parent[member] = b;
        }
        self.set_st(b, b);

        self.best_edge[b].clear();
        let mut mi = 0;
        while mi < self.flower[b].len() {
            let xs = self.flower[b][mi];
            let incident_edges: Vec<(usize, SparseEdge)> =
                self.best_edge[xs].iter().map(|(&x, &e)| (x, e)).collect();
            for (x, gxsx) in incident_edges {
                let gbx = self.get_edge(b, x);
                if gbx.is_none() || self.e_delta(gxsx) < self.e_delta(gbx.unwrap()) {
                    self.set_edge(b, x, gxsx);
                }
            }
            mi += 1;
        }
        self.set_slack(b);
    }

    fn expand_blossom(&mut self, b: usize) {
        self.blossoms_expanded += 1;
        let mut mi = 0;
        while mi < self.flower[b].len() {
            let m = self.flower[b][mi];
            self.set_st(m, m);
            self.parent[m] = 0;
            mi += 1;
        }

        let edge_to_pa = self
            .get_edge(b, self.pa[b])
            .expect("blossom must have incident edge to parent");
        let xr = self.child_of(edge_to_pa.u as usize, b);
        let pr = self.get_pr(b, xr);
        let mut i = 0;
        while i < pr {
            let xs = self.flower[b][i];
            let xns = self.flower[b][i + 1];
            let cycle_edge = self.get_edge(xns, xs).expect("cycle edge must exist");
            self.pa[xs] = cycle_edge.u as usize;
            self.s[xs] = 1;
            self.s[xns] = 0;
            self.slack[xs] = 0;
            self.slack_edge[xs] = None;
            self.set_slack(xns);
            self.q_push(xns);
            i += 2;
        }
        self.s[xr] = 1;
        self.pa[xr] = self.pa[b];
        let mut idx = pr + 1;
        while idx < self.flower[b].len() {
            let xs = self.flower[b][idx];
            self.s[xs] = -1;
            self.set_slack(xs);
            idx += 1;
        }
        self.st[b] = 0;
    }

    fn on_found_edge(&mut self, e: SparseEdge) -> bool {
        let u = self.st[e.u as usize];
        let v = self.st[e.v as usize];
        if self.s[v] == -1 {
            self.pa[v] = e.u as usize;
            self.s[v] = 1;
            let nu = self.st[self.mate[v]];
            self.slack[v] = 0;
            self.slack_edge[v] = None;
            self.slack[nu] = 0;
            self.slack_edge[nu] = None;
            self.s[nu] = 0;
            self.q_push(nu);
        } else if self.s[v] == 0 {
            let lca = self.get_lca(u, v);
            if lca == 0 {
                self.augment(u, v);
                self.augment(v, u);
                return true;
            } else {
                self.add_blossom(u, lca, v);
            }
        }
        false
    }

    fn matching_phase(&mut self) -> bool {
        for i in 1..=self.n_x {
            self.s[i] = -1;
            self.slack[i] = 0;
            self.slack_edge[i] = None;
        }
        self.q.clear();
        for x in 1..=self.n_x {
            if self.st[x] == x && self.mate[x] == 0 {
                self.pa[x] = 0;
                self.s[x] = 0;
                self.q_push(x);
            }
        }
        if self.q.is_empty() {
            return false;
        }

        loop {
            while let Some(u) = self.q.pop_front() {
                self.iterations += 1;
                if self.s[self.st[u]] == 1 {
                    continue;
                }
                let deg = self.adj[u].len();
                for i in 0..deg {
                    let e = self.adj[u][i];
                    let v = e.v as usize;
                    if self.st[u] != self.st[v] {
                        let delta = self.e_delta(e);
                        if delta == 0 {
                            if self.on_found_edge(e) {
                                return true;
                            }
                        } else {
                            let x = self.st[v];
                            self.update_slack(e, x);
                        }
                    }
                }
            }

            let mut d = i128::MAX;
            for b in (self.n + 1)..=self.n_x {
                if self.st[b] == b && self.s[b] == 1 {
                    d = d.min(self.lab[b] / 2);
                }
            }
            for x in 1..=self.n_x {
                if self.st[x] == x
                    && let Some(edge) = self.slack_edge[x]
                {
                    let delta = self.e_delta(edge);
                    if self.s[x] == -1 {
                        d = d.min(delta);
                    } else if self.s[x] == 0 {
                        d = d.min(delta / 2);
                    }
                }
            }

            if d == i128::MAX {
                return false;
            }

            self.dual_updates += 1;
            for u in 1..=self.n {
                match self.s[self.st[u]] {
                    0 => {
                        if self.lab[u] <= d {
                            return false;
                        }
                        self.lab[u] -= d;
                    }
                    1 => self.lab[u] += d,
                    _ => {}
                }
            }
            for b in (self.n + 1)..=self.n_x {
                if self.st[b] == b {
                    if self.s[b] == 0 {
                        self.lab[b] = self.lab[b].checked_add(d * 2).expect("lab i128 overflow");
                    } else if self.s[b] == 1 {
                        self.lab[b] -= d * 2;
                    }
                }
            }

            self.q.clear();
            for x in 1..=self.n_x {
                if self.st[x] == x && self.slack_edge[x].is_some() {
                    let edge = self.slack_edge[x].unwrap();
                    if self.st[edge.u as usize] != x
                        && self.e_delta(edge) == 0
                        && self.on_found_edge(edge)
                    {
                        return true;
                    }
                }
            }
            for b in (self.n + 1)..=self.n_x {
                if self.st[b] == b && self.s[b] == 1 && self.lab[b] == 0 {
                    self.expand_blossom(b);
                }
            }
        }
    }

    /// Solves for maximum weight matching using the primal-dual blossom method.
    pub fn solve(&mut self) -> Vec<Option<usize>> {
        for u in 1..=self.n {
            self.mate[u] = 0;
        }
        self.n_x = self.n;
        for u in 0..=self.n {
            self.st[u] = u;
            self.flower[u].clear();
            self.parent[u] = 0;
        }
        for b in (self.n + 1)..(2 * self.n + 1) {
            self.st[b] = 0;
            self.flower[b].clear();
            self.parent[b] = 0;
        }

        let mut w_max = 0_i128;
        for u in 1..=self.n {
            let deg = self.adj[u].len();
            for i in 0..deg {
                let e = self.adj[u][i];
                self.set_edge(e.u as usize, e.v as usize, e);
                if e.weight > w_max {
                    w_max = e.weight;
                }
            }
        }
        for u in 1..=self.n {
            self.lab[u] = w_max;
        }

        while self.matching_phase() {}

        let mut result = vec![None; self.n];
        for u in 1..=self.n {
            if self.mate[u] != 0 {
                result[u - 1] = Some(self.mate[u] - 1);
            }
        }
        result
    }
}

/// Solves Minimum-Weight Perfect Matching on `graph` using the sparse sub-cubic algorithm.
pub fn sparse_minimum_weight_perfect_matching(graph: &Graph) -> Result<MatchingResult, String> {
    if !graph.n.is_multiple_of(2) {
        return Err(format!(
            "a perfect matching requires an even number of vertices (graph has n = {})",
            graph.n
        ));
    }

    if graph.n == 0 {
        return Ok(MatchingResult {
            edges: Vec::new(),
            total_weight: 0.0,
            is_perfect: true,
            metadata: MatchingMetadata::default(),
        });
    }

    u32::try_from(graph.n)
        .map_err(|_| format!("sparse solver supports at most {} vertices", u32::MAX))?;

    let start = Instant::now();

    // 1. Validate and filter edges
    let filtered_edges = filter_edges(graph)?;
    if filtered_edges.is_empty() {
        return Err(format!(
            "graph (n = {}, m = {}) has no edges available for matching",
            graph.n,
            graph.edges.len()
        ));
    }

    // Check vertex degrees: every vertex must have at least one incident edge
    let mut degree = vec![0usize; graph.n];
    for e in &filtered_edges {
        degree[e.u] += 1;
        degree[e.v] += 1;
    }
    if let Some((v, _)) = degree.iter().enumerate().find(|&(_, &d)| d == 0) {
        return Err(format!(
            "graph has no perfect matching: vertex {v} has degree 0 (isolated)"
        ));
    }

    // 2. Exact scaling to integers
    let weights: Vec<f64> = filtered_edges.iter().map(|e| e.weight).collect();
    let scaler = WeightScaler::new(&weights);

    let mut scaled_edges: Vec<(usize, usize, usize, i128)> =
        Vec::with_capacity(filtered_edges.len());
    for e in &filtered_edges {
        let scaled_cost = scaler.to_integer(e.weight)?;
        scaled_edges.push((e.u, e.v, e.original_index, scaled_cost));
    }

    let (min_cost, max_cost) = scaled_edges.iter().fold(
        (i128::MAX, i128::MIN),
        |(min_cost, max_cost), &(_, _, _, cost)| (min_cost.min(cost), max_cost.max(cost)),
    );

    let k =
        i128::try_from(graph.n / 2).map_err(|_| "vertex count exceeds i128 limit".to_string())?;

    let cardinality_bonus = max_cost
        .checked_sub(min_cost)
        .and_then(|range| range.checked_mul(k))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "edge-weight range is too large to represent in i128".to_string())?;

    // Ensure the weight arithmetic during dual updates cannot overflow i128.
    // The maximum lab value is w_max ≈ cardinality_bonus; dual deltas are at most w_max,
    // and blossom labs are doubled, so we conservatively require 4 * cardinality_bonus < i128::MAX.
    if cardinality_bonus > i128::MAX / 4 {
        return Err(
            "edge-weight range is too large for the sparse solver's i128 arithmetic".to_string(),
        );
    }

    // 3. Build sparse adjacency list (1-indexed for solver)
    let mut adj = vec![Vec::new(); graph.n + 1];
    for (u, v, orig_idx, cost) in scaled_edges {
        let weight: i128 = cardinality_bonus - (cost - min_cost);
        let e1 = SparseEdge::new(u + 1, v + 1, orig_idx, weight)?;
        let e2 = SparseEdge::new(v + 1, u + 1, orig_idx, weight)?;
        adj[u + 1].push(e1);
        adj[v + 1].push(e2);
    }

    // 4. Solve using SparseBlossomSolver
    let mut solver = SparseBlossomSolver::new(graph.n, adj);
    let mate = solver.solve();

    // Verify perfect matching
    if mate.iter().any(Option::is_none) {
        let unmatched_count = mate.iter().filter(|m| m.is_none()).count();
        return Err(format!(
            "graph does not have a perfect matching: {unmatched_count} of {} vertices could not be matched",
            graph.n
        ));
    }

    // 5. Reconstruct matched edges
    let mut matched_edges = Vec::with_capacity(graph.n / 2);
    for (u, partner) in mate.into_iter().enumerate() {
        let v = partner.expect("all partners verified");
        if u < v {
            let position = filtered_edges
                .binary_search_by_key(&(u, v), |edge| (edge.u, edge.v))
                .map_err(|_| format!("internal error: matched edge ({u}, {v}) not found"))?;
            matched_edges.push(filtered_edges[position].original_index);
        }
    }

    matched_edges.sort_unstable();

    let total_weight: f64 = matched_edges
        .iter()
        .map(|&idx| graph.edges[idx].weight_or_one())
        .sum();

    Ok(MatchingResult {
        edges: matched_edges,
        total_weight,
        is_perfect: true,
        metadata: MatchingMetadata {
            iterations: solver.iterations,
            augmentations: graph.n / 2,
            blossoms_created: solver.blossoms_created,
            blossoms_expanded: solver.blossoms_expanded,
            dual_updates: solver.dual_updates,
            runtime_ms: start.elapsed().as_secs_f64() * 1000.0,
        },
    })
}
