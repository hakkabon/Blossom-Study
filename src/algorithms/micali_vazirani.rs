//! Phased implementation of Micali–Vazirani maximum-cardinality matching.
//!
//! Micali–Vazirani (MV) finds a maximum-cardinality matching in a general graph
//! in O(sqrt(V) E) time.  Like Hopcroft–Karp, it works in phases and augments
//! along a maximal collection of shortest augmenting paths.  General graphs
//! make the phase substantially harder: two shortest alternating paths can
//! meet around an odd cycle, producing a *petal*.  MV uses MIN/MAX levels,
//! bridges, double depth-first search (DDFS), and set merging to process those
//! petals without repeatedly shrinking an explicit graph.
//!
//! # Current milestone
//!
//! This module implements the first, deliberately isolated milestone:
//!
//! 1. alternating parity-level discovery from every exposed root;
//! 2. derivation of preliminary MIN/MAX levels;
//! 3. classification of predecessor edges (*props*) and bridge candidates.
//!
//! The parity search is useful and testable on its own, but it is not yet the
//! complete blossom-aware MV level assignment.  Bridge tenacity, DDFS, petal
//! creation, set merging, and phase augmentation remain explicit later phases.
//! The public matching method therefore still returns `NotImplemented` after
//! completing level search.  It never delegates to another matcher, preventing
//! conventional blossom timings from being mislabeled as MV timings.

use crate::algorithms::MatchingResult;
use crate::graph::Graph;
use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Ready,
    LevelSearch,
    BridgeProcessing,
    Augmentation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MicaliVaziraniError {
    GraphChanged,
    InvalidMatching,
    NotImplemented { phase: Phase },
}

impl fmt::Display for MicaliVaziraniError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphChanged => write!(formatter, "solver called with a different graph"),
            Self::InvalidMatching => {
                write!(formatter, "initial matching is inconsistent with the graph")
            }
            Self::NotImplemented { phase } => write!(
                formatter,
                "Micali–Vazirani phase {phase:?} is not implemented yet"
            ),
        }
    }
}

impl std::error::Error for MicaliVaziraniError {}

/// Radix bucket queue ordering candidate bridges by odd tenacity (1, 3, 5, ...).
/// Operations run in O(1) amortized time without log(n) heap overhead.
#[derive(Clone, Debug, Default)]
pub struct TenacityQueue {
    buckets: Vec<Vec<usize>>,
    min_bucket: usize,
    count: usize,
}

impl TenacityQueue {
    pub fn new(max_vertices: usize) -> Self {
        Self {
            buckets: vec![Vec::new(); max_vertices + 1],
            min_bucket: 0,
            count: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, edge_index: usize, tenacity: usize) {
        let bucket_idx = tenacity / 2;
        if bucket_idx < self.buckets.len() {
            self.buckets[bucket_idx].push(edge_index);
            self.count += 1;
            if bucket_idx < self.min_bucket {
                self.min_bucket = bucket_idx;
            }
        }
    }

    #[inline]
    pub fn pop_min(&mut self) -> Option<(usize, usize)> {
        while self.min_bucket < self.buckets.len() {
            if let Some(edge_index) = self.buckets[self.min_bucket].pop() {
                self.count -= 1;
                let tenacity = 2 * self.min_bucket + 1;
                return Some((edge_index, tenacity));
            }
            self.min_bucket += 1;
        }
        None
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitSource {
    None,
    SearchLeft,
    SearchRight,
}

#[derive(Clone, Copy, Debug)]
pub struct DdfsMarker {
    pub epoch: u32,
    pub source: VisitSource,
}

impl Default for DdfsMarker {
    fn default() -> Self {
        Self {
            epoch: 0,
            source: VisitSource::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DdfsResult {
    /// A petal (blossom) formed by an odd cycle meeting at a common base.
    Petal {
        base: usize,
        cycle_vertices: Vec<usize>,
    },
    /// An augmenting path connecting two distinct exposed roots.
    AugmentingPath {
        bridge_edge: usize,
        root_left: usize,
        root_right: usize,
        left_path: Vec<usize>,
        right_path: Vec<usize>,
    },
    /// Redundant bridge whose endpoints already belong to the same petal.
    Redundant,
}

/// Persistent state for the phase-based Micali–Vazirani implementation.
#[derive(Clone, Debug)]
pub struct MicaliVazirani {
    vertex_count: usize,
    /// Canonical endpoint sequence used to reject a different graph even when
    /// it happens to have the same number of vertices and edges.
    edge_signature: Vec<(usize, usize)>,
    phase: Phase,
    mate: Vec<Option<usize>>,
    /// Shortest even-length alternating walk from an exposed root.
    even_level: Vec<Option<usize>>,
    /// Shortest odd-length alternating walk from an exposed root.
    odd_level: Vec<Option<usize>>,
    min_level: Vec<Option<usize>>,
    max_level: Vec<Option<usize>>,
    /// Edge indices that can be the final edge of a current MIN-level path.
    predecessor: Vec<Vec<usize>>,
    petal_base: Vec<usize>,
    /// Edges reachable by the parity search but not classified as props.
    /// Full MV orders these by tenacity before DDFS processes them.
    bridges: Vec<usize>,
    level_search_scans: usize,
    ddfs_epoch: u32,
    ddfs_markers: Vec<DdfsMarker>,
    augmenting_bridges: Vec<usize>,
    bottleneck_tenacity: Option<usize>,
}

impl MicaliVazirani {
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        Self {
            vertex_count: n,
            edge_signature: edge_signature(graph),
            phase: Phase::Ready,
            mate: vec![None; n],
            even_level: vec![None; n],
            odd_level: vec![None; n],
            min_level: vec![None; n],
            max_level: vec![None; n],
            predecessor: vec![Vec::new(); n],
            petal_base: (0..n).collect(),
            bridges: Vec::new(),
            level_search_scans: 0,
            ddfs_epoch: 0,
            ddfs_markers: vec![DdfsMarker::default(); n],
            augmenting_bridges: Vec::new(),
            bottleneck_tenacity: None,
        }
    }

    /// Construct a solver initialized with a verified valid partial matching.
    pub fn with_matching(
        graph: &Graph,
        mate: Vec<Option<usize>>,
    ) -> Result<Self, MicaliVaziraniError> {
        let n = graph.n;
        if mate.len() != n {
            return Err(MicaliVaziraniError::InvalidMatching);
        }
        for (u, &m) in mate.iter().enumerate() {
            if let Some(v) = m {
                if v >= n || v == u || mate[v] != Some(u) {
                    return Err(MicaliVaziraniError::InvalidMatching);
                }
                let edge_exists = graph.adj_edges(u).iter().any(|&e_idx| {
                    let edge = &graph.edges[e_idx];
                    (edge.u == u && edge.v == v) || (edge.u == v && edge.v == u)
                });
                if !edge_exists {
                    return Err(MicaliVaziraniError::InvalidMatching);
                }
            }
        }
        let mut solver = Self::new(graph);
        solver.mate = mate;
        Ok(solver)
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn mate(&self) -> &[Option<usize>] {
        &self.mate
    }

    /// Preliminary MIN levels produced by the current phase-1 milestone.
    pub fn min_level(&self) -> &[Option<usize>] {
        &self.min_level
    }

    /// A MAX level is available once both parity levels have been discovered.
    pub fn max_level(&self) -> &[Option<usize>] {
        &self.max_level
    }

    pub fn even_level(&self) -> &[Option<usize>] {
        &self.even_level
    }

    pub fn odd_level(&self) -> &[Option<usize>] {
        &self.odd_level
    }

    pub fn predecessor_edges(&self, vertex: usize) -> Option<&[usize]> {
        self.predecessor.get(vertex).map(Vec::as_slice)
    }

    pub fn bridge_candidates(&self) -> &[usize] {
        &self.bridges
    }

    /// Check if a given edge is currently classified as a bridge.
    pub fn is_bridge(&self, edge_index: usize) -> bool {
        self.bridges.contains(&edge_index)
    }

    /// Check if `edge_index` is an incoming predecessor (prop) edge for `vertex`.
    pub fn is_predecessor_edge(&self, vertex: usize, edge_index: usize) -> bool {
        self.predecessor
            .get(vertex)
            .map(|props| props.contains(&edge_index))
            .unwrap_or(false)
    }

    /// Compute the tenacity of an edge if both endpoints have discovered even levels.
    /// tenacity(e) = even_level(u) + even_level(v) + 1.
    pub fn bridge_tenacity(&self, graph: &Graph, edge_index: usize) -> Option<usize> {
        let edge = graph.edges.get(edge_index)?;
        let u_even = self.even_level.get(edge.u)?.as_ref()?;
        let v_even = self.even_level.get(edge.v)?.as_ref()?;
        Some(*u_even + *v_even + 1)
    }

    pub fn level_search_scans(&self) -> usize {
        self.level_search_scans
    }

    pub fn augmenting_bridges(&self) -> &[usize] {
        &self.augmenting_bridges
    }

    pub fn bottleneck_tenacity(&self) -> Option<usize> {
        self.bottleneck_tenacity
    }

    pub fn petal_base_of(&mut self, vertex: usize) -> usize {
        self.find_petal_base(vertex)
    }

    pub fn max_cardinality_matching(
        &mut self,
        graph: &Graph,
    ) -> Result<MatchingResult, MicaliVaziraniError> {
        self.run_level_search_phase(graph)?;
        self.run_bridge_processing_phase(graph)?;
        Err(MicaliVaziraniError::NotImplemented { phase: self.phase })
    }

    /// Process discovered bridges in non-decreasing order of tenacity via TenacityQueue
    /// and Double Depth-First Search (DDFS), finding petals and bottleneck augmenting paths.
    pub fn run_bridge_processing_phase(
        &mut self,
        graph: &Graph,
    ) -> Result<Option<usize>, MicaliVaziraniError> {
        self.validate_graph(graph)?;
        if self.phase == Phase::Ready {
            self.run_level_search_phase(graph)?;
        }

        let mut queue = TenacityQueue::new(graph.n);
        for &edge_idx in &self.bridges {
            if let Some(tenacity) = self.bridge_tenacity(graph, edge_idx) {
                queue.push(edge_idx, tenacity);
            }
        }

        while let Some((edge_idx, tenacity)) = queue.pop_min() {
            if let Some(min_aug_tenacity) = self.bottleneck_tenacity {
                if tenacity > min_aug_tenacity {
                    break;
                }
            }

            match self.ddfs(graph, edge_idx, tenacity) {
                DdfsResult::Petal { .. } => {}
                DdfsResult::AugmentingPath { bridge_edge, .. } => {
                    if self.bottleneck_tenacity.is_none() {
                        self.bottleneck_tenacity = Some(tenacity);
                    }
                    self.augmenting_bridges.push(bridge_edge);
                }
                DdfsResult::Redundant => {}
            }
        }

        self.phase = Phase::Augmentation;
        Ok(self.bottleneck_tenacity)
    }

    /// Execute lockstep Double Depth-First Search for a bridge edge (u, v).
    /// Predecessors are popped destructively to ensure linear per-phase complexity.
    pub fn ddfs(&mut self, graph: &Graph, bridge_index: usize, tenacity: usize) -> DdfsResult {
        let edge = &graph.edges[bridge_index];
        let mut curr_u = self.find_petal_base(edge.u);
        let mut curr_v = self.find_petal_base(edge.v);

        if curr_u == curr_v {
            return DdfsResult::Redundant;
        }

        self.ddfs_epoch += 1;
        let epoch = self.ddfs_epoch;

        self.ddfs_markers[curr_u] = DdfsMarker {
            epoch,
            source: VisitSource::SearchLeft,
        };
        self.ddfs_markers[curr_v] = DdfsMarker {
            epoch,
            source: VisitSource::SearchRight,
        };

        let mut path_u = vec![curr_u];
        let mut path_v = vec![curr_v];

        let mut u_active = true;
        let mut v_active = true;

        loop {
            if u_active {
                if let Some(pred) = self.advance_ddfs_step(graph, curr_u) {
                    let base_pred = self.find_petal_base(pred);
                    path_u.push(base_pred);

                    if self.ddfs_markers[base_pred].epoch == epoch {
                        if self.ddfs_markers[base_pred].source == VisitSource::SearchRight {
                            let mut cycle_vertices = path_u;
                            cycle_vertices.extend(path_v);
                            self.union_petal(base_pred, &cycle_vertices);
                            self.assign_petal_max_levels(&cycle_vertices, tenacity);
                            return DdfsResult::Petal {
                                base: base_pred,
                                cycle_vertices,
                            };
                        }
                    } else {
                        self.ddfs_markers[base_pred] = DdfsMarker {
                            epoch,
                            source: VisitSource::SearchLeft,
                        };
                        curr_u = base_pred;
                    }
                } else {
                    u_active = false;
                }
            }

            if v_active {
                if let Some(pred) = self.advance_ddfs_step(graph, curr_v) {
                    let base_pred = self.find_petal_base(pred);
                    path_v.push(base_pred);

                    if self.ddfs_markers[base_pred].epoch == epoch {
                        if self.ddfs_markers[base_pred].source == VisitSource::SearchLeft {
                            let mut cycle_vertices = path_u;
                            cycle_vertices.extend(path_v);
                            self.union_petal(base_pred, &cycle_vertices);
                            self.assign_petal_max_levels(&cycle_vertices, tenacity);
                            return DdfsResult::Petal {
                                base: base_pred,
                                cycle_vertices,
                            };
                        }
                    } else {
                        self.ddfs_markers[base_pred] = DdfsMarker {
                            epoch,
                            source: VisitSource::SearchRight,
                        };
                        curr_v = base_pred;
                    }
                } else {
                    v_active = false;
                }
            }

            if !u_active && !v_active {
                return DdfsResult::AugmentingPath {
                    bridge_edge: bridge_index,
                    root_left: curr_u,
                    root_right: curr_v,
                    left_path: path_u,
                    right_path: path_v,
                };
            }
        }
    }

    fn advance_ddfs_step(&mut self, _graph: &Graph, vertex: usize) -> Option<usize> {
        while let Some(edge_idx) = self.predecessor[vertex].pop() {
            let edge = &_graph.edges[edge_idx];
            let neighbor = if edge.u == vertex { edge.v } else { edge.u };
            return Some(neighbor);
        }
        None
    }

    fn assign_petal_max_levels(&mut self, vertices: &[usize], tenacity: usize) {
        for &v in vertices {
            if self.max_level[v].is_none() {
                if let Some(min_lvl) = self.min_level[v] {
                    if tenacity >= min_lvl {
                        self.max_level[v] = Some(tenacity - min_lvl);
                    }
                }
            }
        }
    }

    pub fn find_petal_base(&mut self, mut v: usize) -> usize {
        let mut root = v;
        while root != self.petal_base[root] {
            root = self.petal_base[root];
        }
        while v != root {
            let next = self.petal_base[v];
            self.petal_base[v] = root;
            v = next;
        }
        root
    }

    pub fn union_petal(&mut self, base: usize, cycle_vertices: &[usize]) {
        let root_base = self.find_petal_base(base);
        for &v in cycle_vertices {
            let root_v = self.find_petal_base(v);
            self.petal_base[root_v] = root_base;
        }
    }

    /// Execute the currently implemented MV milestone without claiming that a
    /// maximum matching has been produced.  This method exists so Criterion can
    /// track phase-1 development separately from completed cardinality solvers.
    pub fn run_level_search_phase(&mut self, graph: &Graph) -> Result<(), MicaliVaziraniError> {
        self.validate_graph(graph)?;
        self.begin_phase();

        // A state is (vertex, parity).  Parity zero denotes an even-length
        // alternating walk, parity one an odd-length walk.  Keeping both is
        // essential: an odd cycle can make the same vertex reachable at both
        // parities, which is precisely where later petal logic becomes needed.
        let mut queue = VecDeque::new();
        for vertex in 0..self.vertex_count {
            if self.mate[vertex].is_none() {
                self.even_level[vertex] = Some(0);
                queue.push_back((vertex, false));
            }
        }

        while let Some((vertex, odd)) = queue.pop_front() {
            let level = if odd {
                self.odd_level[vertex].expect("queued odd state has a level")
            } else {
                self.even_level[vertex].expect("queued even state has a level")
            };

            for &edge_index in graph.adj_edges(vertex) {
                self.level_search_scans += 1;
                let edge = &graph.edges[edge_index];
                let neighbor = if edge.u == vertex { edge.v } else { edge.u };

                // Alternating paths leave an even state on an unmatched edge
                // and leave an odd state on the unique matched edge.
                let edge_is_matched = self.mate[vertex] == Some(neighbor);
                if edge_is_matched != odd {
                    continue;
                }

                let candidate = level + 1;
                let destination = if odd {
                    &mut self.even_level[neighbor]
                } else {
                    &mut self.odd_level[neighbor]
                };
                if destination.is_none() {
                    *destination = Some(candidate);
                    queue.push_back((neighbor, !odd));
                }
            }
        }

        self.derive_levels_and_edge_classes(graph);
        self.phase = Phase::BridgeProcessing;
        Ok(())
    }

    fn validate_graph(&self, graph: &Graph) -> Result<(), MicaliVaziraniError> {
        if graph.n != self.vertex_count || edge_signature(graph) != self.edge_signature {
            return Err(MicaliVaziraniError::GraphChanged);
        }
        Ok(())
    }

    fn begin_phase(&mut self) {
        self.phase = Phase::LevelSearch;
        self.even_level.fill(None);
        self.odd_level.fill(None);
        self.min_level.fill(None);
        self.max_level.fill(None);
        self.predecessor.iter_mut().for_each(Vec::clear);
        self.petal_base
            .iter_mut()
            .enumerate()
            .for_each(|(v, base)| *base = v);
        self.bridges.clear();
        self.level_search_scans = 0;
        self.ddfs_epoch = 0;
        self.ddfs_markers.fill(DdfsMarker::default());
        self.augmenting_bridges.clear();
        self.bottleneck_tenacity = None;
    }

    fn derive_levels_and_edge_classes(&mut self, graph: &Graph) {
        for vertex in 0..self.vertex_count {
            self.min_level[vertex] = match (self.even_level[vertex], self.odd_level[vertex]) {
                (Some(even), Some(odd)) => Some(even.min(odd)),
                (Some(level), None) | (None, Some(level)) => Some(level),
                (None, None) => None,
            };
            self.max_level[vertex] = match (self.even_level[vertex], self.odd_level[vertex]) {
                (Some(even), Some(odd)) => Some(even.max(odd)),
                _ => None,
            };
        }

        let mut is_prop = vec![false; graph.edges.len()];
        for (edge_index, edge) in graph.edges.iter().enumerate() {
            let matched = self.mate[edge.u] == Some(edge.v);
            self.classify_prop_direction(edge_index, edge.u, edge.v, matched, &mut is_prop);
            self.classify_prop_direction(edge_index, edge.v, edge.u, matched, &mut is_prop);
        }

        // A bridge is the part of the layered alternating graph that is not a
        // predecessor edge.  Later MV phases refine these candidates by
        // tenacity and feed them to DDFS; merely collecting them is not enough
        // to create petals or augment safely.
        self.bridges.extend(
            graph
                .edges
                .iter()
                .enumerate()
                .filter(|(index, edge)| {
                    !is_prop[*index]
                        && self.min_level[edge.u].is_some()
                        && self.min_level[edge.v].is_some()
                })
                .map(|(index, _)| index),
        );
    }

    fn classify_prop_direction(
        &mut self,
        edge_index: usize,
        from: usize,
        to: usize,
        edge_is_matched: bool,
        is_prop: &mut [bool],
    ) {
        let source_level = if edge_is_matched {
            self.odd_level[from]
        } else {
            self.even_level[from]
        };
        let Some(source_level) = source_level else {
            return;
        };
        if self.min_level[to] == Some(source_level + 1) {
            self.predecessor[to].push(edge_index);
            is_prop[edge_index] = true;
        }
    }
}

fn edge_signature(graph: &Graph) -> Vec<(usize, usize)> {
    graph
        .edges
        .iter()
        .map(|edge| (edge.u.min(edge.v), edge.u.max(edge.v)))
        .collect()
}
