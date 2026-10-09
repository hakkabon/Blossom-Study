//! Cherry-tree primal-dual minimum-weight perfect matching.
//!
//! This module is the repaired successor to the earlier Cherry-Tree prototypes.
//! The original implementation had useful pedagogical decomposition, but its
//! contracted state retained only an unordered set of vertices.  That was not
//! sufficient to route augmenting paths through a blossom or to restore an
//! alternating forest when a blossom expanded.
//!
//! The reconstruction keeps the Cherry-Tree terminology while making every
//! correctness-critical relationship explicit:
//!
//! - a cherry tree is an alternating tree rooted at an exposed vertex;
//! - an Outer--Outer edge in one tree creates an odd cherry blossom;
//! - each blossom stores its ordered immediate children, including nesting;
//! - boundary edges retain their original endpoints inside contracted objects;
//! - dual changes create tight edges, and zero-dual Inner blossoms expand; and
//! - augmentation is lifted recursively through every contracted blossom.
//!
//! The implementation is self-contained.  It does not call another solver in
//! this crate.  Like the other exact solvers, its adapter converts MWPM into a
//! cardinality-dominating positive-integer maximum-weight problem.  Exact i128
//! scaling avoids floating-point comparisons inside the primal-dual kernel.
//!
//! The event rules follow the weighted Edmonds/Galil primal-dual framework.
//! See Z. Galil, “Efficient Algorithms for Finding Maximum Matching,”
//! *ACM Computing Surveys* 18(1), 1986, DOI: 10.1145/6462.6502.

use std::collections::VecDeque;
use std::time::Instant;

use super::mwpm::{FilteredEdge, WeightScaler, filter_edges};
use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

const NONE: usize = 0;

/// An edge between two current top-level objects.
///
/// `u` and `v` always remain *original* vertices, even when this edge is stored
/// in a matrix cell belonging to a contracted blossom.  Keeping the real
/// endpoints is what lets augmentation enter a nested blossom at the right
/// child later.
#[derive(Clone, Copy, Debug, Default)]
struct SolverEdge {
    u: usize,
    v: usize,
    weight: i128,
}

impl SolverEdge {
    fn reversed(self) -> Self {
        Self {
            u: self.v,
            v: self.u,
            weight: self.weight,
        }
    }

    fn exists(self) -> bool {
        self.weight > 0
    }
}

/// Search label of a top-level vertex or blossom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Label {
    /// Not currently part of an alternating forest.
    #[default]
    Free,
    /// Even distance from a free root (traditionally an S-object).
    Outer,
    /// Odd distance from a free root (traditionally a T-object).
    Inner,
}

#[derive(Clone, Copy, Debug, Default)]
struct EngineStats {
    edge_scans: usize,
    augmentations: usize,
    blossoms_created: usize,
    blossoms_expanded: usize,
    dual_updates: usize,
}

/// Mutable state for one maximum-weight matching computation.
///
/// Original vertices are numbered `1..=n`; index zero is a sentinel.  Slots
/// `n+1..=2n` hold contracted blossoms.  A blossom slot can be reused after the
/// blossom is expanded, which bounds all arrays by `2n + 1`.
struct CherryPrimalDual {
    n: usize,
    highest_slot: usize,

    /// Best real edge between every pair of current objects.
    edge: Vec<Vec<SolverEdge>>,
    /// Matching endpoint.  For a blossom this is an original vertex outside it.
    mate: Vec<usize>,
    /// Current top-level representative for every vertex/blossom slot.
    top: Vec<usize>,
    /// Alternating-forest parent endpoint for Inner objects.
    parent: Vec<usize>,
    label: Vec<Label>,
    /// Vertex duals occupy `1..=n`; blossom duals occupy the remaining slots.
    dual: Vec<i128>,

    /// Best Outer neighbor that can make an edge incident to this object tight.
    slack_from: Vec<usize>,
    /// Timestamp markers used while finding an alternating-tree LCA.
    visit_mark: Vec<usize>,
    visit_token: usize,

    /// Ordered odd cycle of immediate children for each blossom.
    children: Vec<Vec<usize>>,
    /// For blossom `b` and original vertex `v`, the immediate child of `b`
    /// containing `v`.  This avoids repeatedly walking a nested blossom tree.
    child_containing: Vec<Vec<usize>>,

    queue: VecDeque<usize>,
    stats: EngineStats,
}

impl CherryPrimalDual {
    fn new(n: usize) -> Self {
        let capacity = 2 * n + 1;
        let mut top = vec![NONE; capacity];
        for (vertex, representative) in top.iter_mut().enumerate().take(n + 1).skip(1) {
            *representative = vertex;
        }
        Self {
            n,
            highest_slot: n,
            edge: vec![vec![SolverEdge::default(); capacity]; capacity],
            mate: vec![NONE; capacity],
            top,
            parent: vec![NONE; capacity],
            label: vec![Label::Free; capacity],
            dual: vec![0; capacity],
            slack_from: vec![NONE; capacity],
            visit_mark: vec![0; capacity],
            visit_token: 0,
            children: vec![Vec::new(); capacity],
            child_containing: vec![vec![NONE; n + 1]; capacity],
            queue: VecDeque::new(),
            stats: EngineStats::default(),
        }
    }

    fn add_edge(&mut self, u: usize, v: usize, weight: i128) {
        debug_assert!((1..=self.n).contains(&u));
        debug_assert!((1..=self.n).contains(&v));
        debug_assert_ne!(u, v);
        debug_assert!(weight > 0);

        // Parallel edges have already been filtered, but retaining the maximum
        // here makes the core safe to use independently of the public adapter.
        if !self.edge[u][v].exists() || weight > self.edge[u][v].weight {
            self.edge[u][v] = SolverEdge { u, v, weight };
            self.edge[v][u] = self.edge[u][v].reversed();
        }
    }

    /// Reduced cost (twice the usual slack) of an edge.
    ///
    /// The factor of two keeps half-integral dual changes exact.  Tight edges
    /// have delta zero and are the only edges traversed by the primal search.
    fn edge_delta(&self, edge: SolverEdge) -> i128 {
        self.dual[edge.u] + self.dual[edge.v] - 2 * edge.weight
    }

    fn set_top_recursive(&mut self, object: usize, representative: usize) {
        self.top[object] = representative;
        if object > self.n {
            let nested = self.children[object].clone();
            for child in nested {
                self.set_top_recursive(child, representative);
            }
        }
    }

    /// Queue every original vertex represented by an Outer object.
    fn queue_object(&mut self, object: usize) {
        if object <= self.n {
            self.queue.push_back(object);
        } else {
            let nested = self.children[object].clone();
            for child in nested {
                self.queue_object(child);
            }
        }
    }

    fn update_slack(&mut self, outer_vertex: usize, object: usize) {
        let old = self.slack_from[object];
        if old == NONE
            || self.edge_delta(self.edge[outer_vertex][object])
                < self.edge_delta(self.edge[old][object])
        {
            self.slack_from[object] = outer_vertex;
        }
    }

    fn recompute_slack(&mut self, object: usize) {
        self.slack_from[object] = NONE;
        for vertex in 1..=self.n {
            if self.edge[vertex][object].exists()
                && self.top[vertex] != object
                && self.label[self.top[vertex]] == Label::Outer
            {
                self.update_slack(vertex, object);
            }
        }
    }

    /// Return the common base of two Outer objects, or zero for distinct trees.
    fn lowest_common_base(&mut self, mut left: usize, mut right: usize) -> usize {
        self.visit_token = self.visit_token.wrapping_add(1);
        if self.visit_token == 0 {
            self.visit_mark.fill(0);
            self.visit_token = 1;
        }

        loop {
            if left != NONE {
                left = self.top[left];
                if self.visit_mark[left] == self.visit_token {
                    return left;
                }
                self.visit_mark[left] = self.visit_token;
                left = self.mate[left];
                if left != NONE {
                    let inner = self.top[left];
                    left = self.top[self.parent[inner]];
                }
            }
            std::mem::swap(&mut left, &mut right);
            if left == NONE && right == NONE {
                return NONE;
            }
        }
    }

    fn unused_blossom_slot(&mut self) -> usize {
        for slot in (self.n + 1)..=self.highest_slot {
            if self.top[slot] == NONE {
                return slot;
            }
        }
        self.highest_slot += 1;
        debug_assert!(self.highest_slot < self.top.len());
        self.highest_slot
    }

    fn record_immediate_children(&mut self, blossom: usize) {
        self.child_containing[blossom].fill(NONE);
        let immediate = self.children[blossom].clone();
        for child in immediate {
            if child <= self.n {
                self.child_containing[blossom][child] = child;
            } else {
                for vertex in 1..=self.n {
                    if self.child_containing[child][vertex] != NONE {
                        self.child_containing[blossom][vertex] = child;
                    }
                }
            }
        }
    }

    /// Contract the odd alternating cycle formed by an Outer–Outer edge.
    /// Contract the odd cycle closed by two Outer vertices of one cherry tree.
    fn contract_cherry_blossom(&mut self, left: usize, base: usize, right: usize) {
        let blossom = self.unused_blossom_slot();
        self.stats.blossoms_created += 1;
        self.dual[blossom] = 0;
        self.label[blossom] = Label::Outer;
        self.mate[blossom] = self.mate[base];
        self.parent[blossom] = self.parent[base];
        self.children[blossom].clear();
        self.children[blossom].push(base);

        // Each side contributes alternating pairs (Outer object, Inner object).
        // Reversing the left arm makes the final vector a cyclic order whose
        // even positions are compatible with the current matching.
        let mut arm = Vec::new();
        let mut cursor = left;
        while cursor != base {
            arm.push(cursor);
            let inner = self.top[self.mate[cursor]];
            arm.push(inner);
            self.queue_object(inner);
            cursor = self.top[self.parent[inner]];
        }
        arm.reverse();
        self.children[blossom].extend(arm);

        cursor = right;
        while cursor != base {
            self.children[blossom].push(cursor);
            let inner = self.top[self.mate[cursor]];
            self.children[blossom].push(inner);
            self.queue_object(inner);
            cursor = self.top[self.parent[inner]];
        }

        self.record_immediate_children(blossom);
        self.set_top_recursive(blossom, blossom);

        // A contracted blossom exposes, for each outside object, the boundary
        // edge with minimum reduced cost.  Its stored endpoints still identify
        // the original vertices at which augmentation must enter and leave.
        for object in 1..=self.highest_slot {
            self.edge[blossom][object] = SolverEdge::default();
            self.edge[object][blossom] = SolverEdge::default();
        }
        let immediate = self.children[blossom].clone();
        for child in immediate {
            for object in 1..=self.highest_slot {
                let candidate = self.edge[child][object];
                if candidate.exists()
                    && (!self.edge[blossom][object].exists()
                        || self.edge_delta(candidate) < self.edge_delta(self.edge[blossom][object]))
                {
                    self.edge[blossom][object] = candidate;
                    self.edge[object][blossom] = candidate.reversed();
                }
            }
        }
        self.recompute_slack(blossom);
    }

    /// Locate an entry child and orient the odd cycle so it has even index.
    fn orient_blossom_at(&mut self, blossom: usize, entry_child: usize) -> usize {
        let position = self.children[blossom]
            .iter()
            .position(|&child| child == entry_child)
            .expect("entry endpoint must belong to blossom");
        if position % 2 == 0 {
            position
        } else {
            self.children[blossom][1..].reverse();
            self.children[blossom].len() - position
        }
    }

    /// Install a matched edge, recursively repairing the matching inside a
    /// contracted blossom along the even alternating route to its entry point.
    fn set_match(&mut self, object: usize, other: usize) {
        let boundary = self.edge[object][other];
        self.mate[object] = boundary.v;
        if object <= self.n {
            return;
        }

        let entry_child = self.child_containing[object][boundary.u];
        let position = self.orient_blossom_at(object, entry_child);
        let cycle = self.children[object].clone();
        // Visit both directions of every internal matched pair. `set_match`
        // intentionally updates one endpoint at a time because the two sides
        // of the *external* augmenting edge are handled by separate tree walks.
        for index in 0..position {
            self.set_match(cycle[index], cycle[index ^ 1]);
        }
        self.set_match(entry_child, other);
        self.children[object].rotate_left(position);
    }

    /// Flip one side of an augmenting path, walking toward its free root.
    fn augment_from(&mut self, mut object: usize, mut other: usize) {
        loop {
            let previous_inner = self.top[self.mate[object]];
            self.set_match(object, other);
            if previous_inner == NONE {
                return;
            }
            let previous_outer = self.top[self.parent[previous_inner]];
            self.set_match(previous_inner, previous_outer);
            object = previous_outer;
            other = previous_inner;
        }
    }

    /// Restore an Inner cherry blossom whose dual has fallen to zero.
    fn expand_cherry_blossom(&mut self, blossom: usize) {
        self.stats.blossoms_expanded += 1;
        let cycle = self.children[blossom].clone();
        for &child in &cycle {
            self.set_top_recursive(child, child);
        }

        // An Inner blossom has a unique forest edge entering it.  The real
        // endpoint of that edge tells us which immediate child is the entry.
        let parent_object = self.top[self.parent[blossom]];
        let entry_vertex = self.edge[blossom][parent_object].u;
        let entry_child = self.child_containing[blossom][entry_vertex];
        let position = self.orient_blossom_at(blossom, entry_child);
        let cycle = self.children[blossom].clone();

        let mut index = 0;
        while index < position {
            let inner = cycle[index];
            let outer = cycle[index + 1];
            self.parent[inner] = self.edge[outer][inner].u;
            self.label[inner] = Label::Inner;
            self.label[outer] = Label::Outer;
            self.slack_from[inner] = NONE;
            self.recompute_slack(outer);
            self.queue_object(outer);
            index += 2;
        }

        self.label[entry_child] = Label::Inner;
        self.parent[entry_child] = self.parent[blossom];
        for &child in cycle.iter().skip(position + 1) {
            self.label[child] = Label::Free;
            self.recompute_slack(child);
        }

        self.top[blossom] = NONE;
        self.label[blossom] = Label::Free;
        self.slack_from[blossom] = NONE;
    }

    /// Process a newly tight edge leaving an Outer object.
    fn process_tight_edge(&mut self, boundary: SolverEdge) -> bool {
        let left = self.top[boundary.u];
        let right = self.top[boundary.v];
        if left == right {
            return false;
        }

        match self.label[right] {
            Label::Free => {
                // A Free top-level object is matched (all free vertices were
                // made roots at stage start).  Add it as Inner and expose its
                // matched partner as a new Outer object.
                self.parent[right] = boundary.u;
                self.label[right] = Label::Inner;
                let partner = self.top[self.mate[right]];
                self.slack_from[right] = NONE;
                self.slack_from[partner] = NONE;
                self.label[partner] = Label::Outer;
                self.queue_object(partner);
            }
            Label::Outer => {
                let base = self.lowest_common_base(left, right);
                if base == NONE {
                    self.augment_from(left, right);
                    self.augment_from(right, left);
                    self.stats.augmentations += 1;
                    return true;
                }
                self.contract_cherry_blossom(left, base, right);
            }
            Label::Inner => {}
        }
        false
    }

    /// Run one augmenting stage of the primal-dual algorithm.
    fn grow_cherry_forest(&mut self) -> bool {
        self.label.fill(Label::Free);
        self.parent.fill(NONE);
        self.slack_from.fill(NONE);
        self.queue.clear();

        // Every free top-level object starts a separate alternating tree.  A
        // tight edge joining two such trees is immediately augmenting.
        for object in 1..=self.highest_slot {
            if self.top[object] == object && self.mate[object] == NONE {
                self.label[object] = Label::Outer;
                self.queue_object(object);
            }
        }
        if self.queue.is_empty() {
            return false;
        }

        loop {
            while let Some(vertex) = self.queue.pop_front() {
                if self.label[self.top[vertex]] != Label::Outer {
                    continue;
                }
                for other in 1..=self.n {
                    self.stats.edge_scans += 1;
                    let boundary = self.edge[vertex][other];
                    if !boundary.exists() || self.top[vertex] == self.top[other] {
                        continue;
                    }
                    if self.edge_delta(boundary) == 0 {
                        if self.process_tight_edge(boundary) {
                            return true;
                        }
                    } else {
                        self.update_slack(vertex, self.top[other]);
                    }
                }
            }

            // Select the next dual event.  Vertex duals use doubled units;
            // therefore Outer–Outer slacks and Inner-blossom duals are halved.
            let mut change = i128::MAX;
            for vertex in 1..=self.n {
                if self.label[self.top[vertex]] == Label::Outer {
                    change = change.min(self.dual[vertex]);
                }
            }
            for object in 1..=self.highest_slot {
                if self.top[object] != object {
                    continue;
                }
                if object > self.n && self.label[object] == Label::Inner {
                    change = change.min(self.dual[object] / 2);
                }
                let source = self.slack_from[object];
                if source != NONE {
                    let delta = self.edge_delta(self.edge[source][object]);
                    match self.label[object] {
                        Label::Free => change = change.min(delta),
                        Label::Outer => change = change.min(delta / 2),
                        Label::Inner => {}
                    }
                }
            }
            if change == i128::MAX || change < 0 {
                return false;
            }

            for vertex in 1..=self.n {
                match self.label[self.top[vertex]] {
                    Label::Outer => self.dual[vertex] -= change,
                    Label::Inner => self.dual[vertex] += change,
                    Label::Free => {}
                }
            }
            for blossom in (self.n + 1)..=self.highest_slot {
                if self.top[blossom] != blossom {
                    continue;
                }
                match self.label[blossom] {
                    Label::Outer => self.dual[blossom] += 2 * change,
                    Label::Inner => self.dual[blossom] -= 2 * change,
                    Label::Free => {}
                }
            }
            self.stats.dual_updates += 1;

            // Dual adjustment may make several stored best edges tight.  Each
            // one can grow a tree, create a blossom, or finish the stage.
            for object in 1..=self.highest_slot {
                let source = self.slack_from[object];
                if self.top[object] == object
                    && source != NONE
                    && self.top[source] != object
                    && self.edge_delta(self.edge[source][object]) == 0
                    && self.process_tight_edge(self.edge[source][object])
                {
                    return true;
                }
            }

            // A zero-dual Inner blossom ceases to be active in the laminar
            // dual family and must be expanded before the search continues.
            for blossom in (self.n + 1)..=self.highest_slot {
                if self.top[blossom] == blossom
                    && self.label[blossom] == Label::Inner
                    && self.dual[blossom] == 0
                {
                    self.expand_cherry_blossom(blossom);
                }
            }

            // A zero dual adjustment with no primal event would repeat the same
            // state forever.  It also certifies that this stage cannot augment.
            if change == 0 && self.queue.is_empty() {
                return false;
            }
        }
    }

    fn solve(mut self) -> (Vec<Option<usize>>, EngineStats) {
        let mut maximum_weight = 0;
        for u in 1..=self.n {
            for v in (u + 1)..=self.n {
                maximum_weight = maximum_weight.max(self.edge[u][v].weight);
            }
        }
        for vertex in 1..=self.n {
            self.dual[vertex] = maximum_weight;
        }

        while self.grow_cherry_forest() {}

        let mut result = vec![None; self.n];
        for vertex in 1..=self.n {
            if self.mate[vertex] != NONE {
                result[vertex - 1] = Some(self.mate[vertex] - 1);
            }
        }
        (result, self.stats)
    }
}

/// Self-contained repaired Cherry-Tree solver exposed by the study library.
#[derive(Clone, Copy, Debug, Default)]
pub struct CherryTreeSolver;

impl CherryTreeSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        solve_minimum_weight_perfect(graph)
    }
}

pub fn cherry_tree_minimum_weight_perfect_matching(
    graph: &Graph,
) -> Result<MatchingResult, String> {
    CherryTreeSolver::new().solve(graph)
}

fn validate(graph: &Graph, edges: &[FilteredEdge]) -> Result<(), String> {
    if !graph.n.is_multiple_of(2) {
        return Err(format!(
            "a perfect matching requires an even number of vertices (graph has n = {})",
            graph.n
        ));
    }
    if graph.n == 0 {
        return Ok(());
    }
    if edges.is_empty() {
        return Err(format!(
            "graph (n = {}, m = {}) has no edges available for matching",
            graph.n,
            graph.edges.len()
        ));
    }
    let mut degree = vec![0_usize; graph.n];
    for edge in edges {
        degree[edge.u] += 1;
        degree[edge.v] += 1;
    }
    if let Some((vertex, _)) = degree.iter().enumerate().find(|(_, degree)| **degree == 0) {
        return Err(format!(
            "graph has no perfect matching: vertex {vertex} has degree 0 (isolated)"
        ));
    }
    Ok(())
}

fn solve_minimum_weight_perfect(graph: &Graph) -> Result<MatchingResult, String> {
    let started = Instant::now();
    let filtered = filter_edges(graph)?;
    validate(graph, &filtered)?;
    if graph.n == 0 {
        return Ok(MatchingResult {
            edges: Vec::new(),
            total_weight: 0.0,
            is_perfect: true,
            metadata: MatchingMetadata::default(),
        });
    }

    let weights: Vec<f64> = filtered.iter().map(|edge| edge.weight).collect();
    let scaler = WeightScaler::new(&weights);
    let scaled: Vec<i128> = weights
        .into_iter()
        .map(|weight| scaler.to_integer(weight))
        .collect::<Result<_, _>>()?;
    let (minimum, maximum) = scaled
        .iter()
        .copied()
        .fold((i128::MAX, i128::MIN), |(minimum, maximum), cost| {
            (minimum.min(cost), maximum.max(cost))
        });
    let pairs = i128::try_from(graph.n / 2)
        .map_err(|_| "vertex count exceeds i128 capacity".to_string())?;
    let bonus = maximum
        .checked_sub(minimum)
        .and_then(|range| range.checked_mul(pairs))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "edge-weight range is too large to represent in i128".to_string())?;
    if bonus > i128::MAX / 4 {
        return Err(
            "edge-weight range leaves insufficient i128 headroom for doubled dual arithmetic"
                .to_string(),
        );
    }

    let mut solver = CherryPrimalDual::new(graph.n);
    for (edge, cost) in filtered.iter().zip(scaled) {
        let shifted = cost
            .checked_sub(minimum)
            .ok_or_else(|| "edge-weight transformation underflowed".to_string())?;
        let weight = bonus
            .checked_sub(shifted)
            .ok_or_else(|| "edge-weight transformation underflowed".to_string())?;
        solver.add_edge(edge.u + 1, edge.v + 1, weight);
    }

    let (mates, stats) = solver.solve();
    let unmatched = mates.iter().filter(|mate| mate.is_none()).count();
    if unmatched != 0 {
        return Err(format!(
            "graph does not have a perfect matching: {unmatched} of {} vertices could not be matched",
            graph.n
        ));
    }

    let mut matched_edges = Vec::with_capacity(graph.n / 2);
    for (u, partner) in mates.iter().copied().enumerate() {
        let v = partner.expect("all partners verified above");
        if v >= graph.n || mates[v] != Some(u) {
            return Err(format!(
                "internal error: non-reciprocal mate relation at vertex {u} (partner {v})"
            ));
        }
        if u < v {
            let position = filtered
                .binary_search_by_key(&(u, v), |edge| (edge.u, edge.v))
                .map_err(|_| format!("internal error: matched edge ({u}, {v}) not found"))?;
            matched_edges.push(filtered[position].original_index);
        }
    }
    matched_edges.sort_unstable();
    let total_weight = matched_edges
        .iter()
        .map(|&index| graph.edges[index].weight_or_one())
        .sum();

    Ok(MatchingResult {
        edges: matched_edges,
        total_weight,
        is_perfect: true,
        metadata: MatchingMetadata {
            iterations: stats.edge_scans,
            augmentations: stats.augmentations,
            blossoms_created: stats.blossoms_created,
            blossoms_expanded: stats.blossoms_expanded,
            dual_updates: stats.dual_updates,
            runtime_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    })
}
