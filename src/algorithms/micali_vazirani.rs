//! Scaffold for the Micali–Vazirani maximum-cardinality matching algorithm.
//!
//! The implementation is intentionally not wired to a fallback solver: until
//! MIN/MAX levels, bridges, petals, and augmentation are complete, calling the
//! solver returns [`MicaliVaziraniError::NotImplemented`]. This prevents
//! benchmark results from being mislabeled as Micali–Vazirani results.

use crate::algorithms::MatchingResult;
use crate::graph::Graph;
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
                "Micali–Vazirani is a scaffold; phase {phase:?} is not implemented yet"
            ),
        }
    }
}

impl std::error::Error for MicaliVaziraniError {}

/// State reserved by the phase-based Micali–Vazirani implementation.
#[derive(Clone, Debug)]
pub struct MicaliVazirani {
    vertex_count: usize,
    edge_count: usize,
    phase: Phase,
    mate: Vec<Option<usize>>,
    min_level: Vec<Option<usize>>,
    max_level: Vec<Option<usize>>,
    predecessor: Vec<Vec<usize>>,
    petal_base: Vec<usize>,
    bridges: Vec<usize>,
}

impl MicaliVazirani {
    pub fn new(graph: &Graph) -> Self {
        let n = graph.n;
        Self {
            vertex_count: n,
            edge_count: graph.edges.len(),
            phase: Phase::Ready,
            mate: vec![None; n],
            min_level: vec![None; n],
            max_level: vec![None; n],
            predecessor: vec![Vec::new(); n],
            petal_base: (0..n).collect(),
            bridges: Vec::new(),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn mate(&self) -> &[Option<usize>] {
        &self.mate
    }

    pub fn max_cardinality_matching(
        &mut self,
        graph: &Graph,
    ) -> Result<MatchingResult, MicaliVaziraniError> {
        if graph.n != self.vertex_count || graph.edges.len() != self.edge_count {
            return Err(MicaliVaziraniError::GraphChanged);
        }

        self.begin_phase();
        Err(MicaliVaziraniError::NotImplemented { phase: self.phase })
    }

    fn begin_phase(&mut self) {
        self.phase = Phase::LevelSearch;
        self.min_level.fill(None);
        self.max_level.fill(None);
        self.predecessor.iter_mut().for_each(Vec::clear);
        self.petal_base
            .iter_mut()
            .enumerate()
            .for_each(|(v, base)| *base = v);
        self.bridges.clear();
    }
}
