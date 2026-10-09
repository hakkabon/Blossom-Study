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
    NotImplemented { phase: Phase },
}

impl fmt::Display for MicaliVaziraniError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphChanged => write!(formatter, "solver called with a different graph"),
            Self::NotImplemented { phase } => write!(
                formatter,
                "Micali–Vazirani phase {phase:?} is not implemented yet"
            ),
        }
    }
}

impl std::error::Error for MicaliVaziraniError {}

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
    /// Full MV will order these by tenacity before DDFS processes them.
    bridges: Vec<usize>,
    level_search_scans: usize,
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
        }
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

    pub fn level_search_scans(&self) -> usize {
        self.level_search_scans
    }

    pub fn max_cardinality_matching(
        &mut self,
        graph: &Graph,
    ) -> Result<MatchingResult, MicaliVaziraniError> {
        self.run_level_search_phase(graph)?;
        Err(MicaliVaziraniError::NotImplemented { phase: self.phase })
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
