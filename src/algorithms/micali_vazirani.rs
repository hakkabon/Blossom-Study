//! Experimental phased Micali–Vazirani (MV) maximum-cardinality matching.
//!
//! # Algorithmic Foundations
//!
//! The Micali–Vazirani algorithm (Micali & Vazirani, FOCS 1980; Vazirani, 1994, 2012, 2020)
//! finds a maximum-cardinality matching in a general graph in $O(\sqrt{V} E)$ time.
//! Like Hopcroft–Karp in bipartite graphs, it proceeds in discrete **phases** where each phase
//! discovers a maximal collection of vertex-disjoint shortest augmenting paths.
//!
//! In general graphs, shortest alternating paths can meet around an odd cycle, producing
//! blossoms ("petals"). MV processes petals without repeatedly contracting explicit graphs by
//! maintaining:
//!
//! 1. **Dual Parity Levels (MIN/MAX levels)**:
//!    - `even_level[v]`: shortest even alternating walk from an exposed root to $v$.
//!    - `odd_level[v]`: shortest odd alternating walk from an exposed root to $v$.
//!    - `min_level[v] = min(even_level[v], odd_level[v])`.
//!    - `max_level[v] = tenacity(petal) - min_level[v]`: discovered dynamically when $v$
//!      becomes part of an odd cycle (petal).
//!
//! 2. **Directed Predecessors ("Props")**:
//!    Edges $(u, v)$ in the layered alternating graph where $\text{level}(v) = \text{level}(u) + 1$.
//!
//! 3. **Tenacity-Ordered Bridge Radix Queue (`TenacityQueue`)**:
//!    Unmatched edges between two reachable even states have odd tenacity:
//!    $$\text{tenacity}(e) = \text{even\_level}(u) + \text{even\_level}(v) + 1$$
//!    Bridges are processed strictly in non-decreasing tenacity order in $O(1)$ amortized time.
//!
//! 4. **Double Depth-First Search (DDFS)**:
//!    Backward traversal along predecessor edges from bridge endpoints proposes a petal.
//!    Its certificate is checked against actual graph edges before contraction. The current
//!    implementation also enumerates alternating paths/cycles when prop reconstruction is
//!    insufficient. This makes it a correctness-first experiment, **not** an implementation
//!    with MV's proven $O(E)$ per-phase bound.
//!
//! 5. **Interleaved Search & Dynamic Enqueuing**:
//!    As petals form at tenacity $T$, vertices receiving `max_level` are re-enqueued to propagate
//!    further search through the blossom at level $\text{max\_level} + 1$.
//!
//! 6. **Maximal Disjoint Augmentation & Petal Unrolling**:
//!    Augmenting bridges of bottleneck tenacity $L = t_{\min}$ are unrolled into vertex-disjoint
//!    augmenting paths, flipping matched edges in $O(L)$ time.
//!
//! 7. **Termination (Berge's Lemma)**:
//!    The phase's failure to find a bridge is not yet a proof of optimality here: nested-petal
//!    discovery is incomplete. An independent exact alternating-path search checks the claim
//!    and returns an error if the phased search missed a path. It is exponential in general.

use crate::algorithms::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;
use std::collections::VecDeque;
use std::fmt;
use std::time::Instant;

/// Execution phase of the Micali–Vazirani state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Ready,
    LevelSearch,
    BridgeProcessing,
    Augmentation,
    Finished,
}

/// Errors produced during Micali–Vazirani execution or validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MicaliVaziraniError {
    GraphChanged,
    InvalidMatching,
    NotImplemented {
        phase: Phase,
    },
    /// A shorter augmenting path disproves the proposed phase bottleneck.
    NonShortestAugmentation {
        proposed: usize,
        actual: usize,
    },
    /// An augmenting path remains although the phased search declared completion.
    UncertifiedOptimality {
        path_length: usize,
    },
    /// A claimed bottleneck yielded no certified path to augment.
    MissingAugmentation {
        tenacity: usize,
    },
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
            Self::NonShortestAugmentation { proposed, actual } => write!(
                formatter,
                "proposed augmentation length {proposed} is not shortest; found length {actual}"
            ),
            Self::UncertifiedOptimality { path_length } => write!(
                formatter,
                "phased search missed an augmenting path of length {path_length}"
            ),
            Self::MissingAugmentation { tenacity } => write!(
                formatter,
                "no certified augmentation for claimed bottleneck tenacity {tenacity}"
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
    pub fn peek_min(&mut self) -> Option<(usize, usize)> {
        while self.min_bucket < self.buckets.len() {
            if let Some(&edge_index) = self.buckets[self.min_bucket].last() {
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

    #[inline]
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.min_bucket = 0;
        self.count = 0;
    }
}

/// Identifies which search frontier visited a vertex during DDFS lockstep traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitSource {
    None,
    SearchLeft,
    SearchRight,
}

/// Timestamped marker for DDFS traversal avoiding O(V) array clearing.
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

/// Represents an odd cycle (blossom) contracted dynamically in the alternating forest.
#[derive(Clone, Debug)]
pub struct Petal {
    pub id: usize,
    pub base: usize,
    pub bridge_index: usize,
    pub tenacity: usize,
    pub cycle_vertices: Vec<usize>,
    pub path_u: Vec<usize>,
    pub path_v: Vec<usize>,
}

/// Result of a Double Depth-First Search (DDFS) step on a candidate bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DdfsResult {
    /// An odd cycle meeting at a common lowest common ancestor base.
    Petal {
        base: usize,
        cycle_vertices: Vec<usize>,
        path_u: Vec<usize>,
        path_v: Vec<usize>,
    },
    /// An augmenting path connecting two distinct exposed roots.
    AugmentingPath {
        bridge_edge: usize,
        root_left: usize,
        root_right: usize,
        left_path: Vec<usize>,
        right_path: Vec<usize>,
    },
    /// Redundant bridge whose endpoints already share the same petal base.
    Redundant,
}

/// Complete state for the phased Micali–Vazirani maximum-cardinality matching algorithm.
#[derive(Clone, Debug)]
pub struct MicaliVazirani {
    vertex_count: usize,
    /// Canonical endpoint sequence used to verify graph identity across phases.
    edge_signature: Vec<(usize, usize)>,
    phase: Phase,
    mate: Vec<Option<usize>>,
    /// Shortest even-length alternating walk from an exposed root.
    even_level: Vec<Option<usize>>,
    /// Shortest odd-length alternating walk from an exposed root.
    odd_level: Vec<Option<usize>>,
    min_level: Vec<Option<usize>>,
    max_level: Vec<Option<usize>>,
    /// Directed predecessor edges entering a vertex in the alternating layered DAG.
    predecessor: Vec<Vec<usize>>,
    /// Disjoint-set union tracking blossom bases (super-vertices).
    petal_base: Vec<usize>,
    /// Bridges collected during level search.
    bridges: Vec<usize>,
    level_search_scans: usize,
    ddfs_epoch: u32,
    ddfs_markers: Vec<DdfsMarker>,
    /// Candidate augmenting bridges belonging to the bottleneck tenacity of the current phase.
    augmenting_bridges: Vec<usize>,
    bottleneck_tenacity: Option<usize>,

    /// Contracted petals formed in the current phase.
    petals: Vec<Petal>,
    /// Maps each vertex to the petal that assigned its MAX level (if any).
    petal_by_vertex: Vec<Option<usize>>,
    /// Level queues for interleaved BFS traversal.
    level_queues: Vec<VecDeque<usize>>,
    /// Vertices committed to augmenting paths in the current phase.
    committed_in_aug: Vec<bool>,
    /// Destructive DDFS predecessor edge scan pointer per vertex (preserves prop edges for augmentation).
    ddfs_pred_ptr: Vec<usize>,

    /// Aggregate metrics across all phases.
    iterations: usize,
    total_augmentations: usize,
    total_blossoms_created: usize,
}

impl MicaliVazirani {
    /// Construct a new Micali–Vazirani solver initialized with an empty matching.
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
            petals: Vec::new(),
            petal_by_vertex: vec![None; n],
            level_queues: vec![VecDeque::new(); 2 * n + 4],
            committed_in_aug: vec![false; n],
            ddfs_pred_ptr: vec![0; n],
            iterations: 0,
            total_augmentations: 0,
            total_blossoms_created: 0,
        }
    }

    /// Construct a solver initialized with a pre-existing valid matching.
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

    pub fn min_level(&self) -> &[Option<usize>] {
        &self.min_level
    }

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

    pub fn is_bridge(&self, edge_index: usize) -> bool {
        self.bridges.contains(&edge_index)
    }

    pub fn is_predecessor_edge(&self, vertex: usize, edge_index: usize) -> bool {
        self.predecessor
            .get(vertex)
            .map(|props| props.contains(&edge_index))
            .unwrap_or(false)
    }

    pub fn bridge_tenacity(&self, graph: &Graph, edge_index: usize) -> Option<usize> {
        let edge = graph.edges.get(edge_index)?;
        // A bridge may itself belong to the current matching. The two cases
        // meet different alternating-path parities at their endpoints.
        let (u_level, v_level) = if self.mate[edge.u] == Some(edge.v) {
            (
                *self.odd_level.get(edge.u)?.as_ref()?,
                *self.odd_level.get(edge.v)?.as_ref()?,
            )
        } else {
            (
                *self.even_level.get(edge.u)?.as_ref()?,
                *self.even_level.get(edge.v)?.as_ref()?,
            )
        };
        u_level.checked_add(v_level)?.checked_add(1)
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

    pub fn petals(&self) -> &[Petal] {
        &self.petals
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn total_augmentations(&self) -> usize {
        self.total_augmentations
    }

    // =========================================================================
    // PRIMARY ENTRY POINT: Solve Maximum Cardinality Matching
    // =========================================================================

    /// Attempts a maximum-cardinality matching using the experimental MV phases.
    ///
    /// An independent, exact path oracle checks shortestness before every flip and
    /// optimality at termination. A failed check returns an error rather than a
    /// silently nonmaximum result. This intentionally postpones the time bound.
    pub fn max_cardinality_matching(
        &mut self,
        graph: &Graph,
    ) -> Result<MatchingResult, MicaliVaziraniError> {
        let start_time = Instant::now();
        self.validate_graph(graph)?;
        self.iterations = 0;
        self.total_augmentations = 0;
        self.total_blossoms_created = 0;

        loop {
            self.iterations += 1;
            self.begin_phase();

            // Run the interleaved level search & bridge processing phase
            let bottleneck = self.run_interleaved_search_phase(graph)?;

            // The search can still miss a bridge inside nested petals. Check
            // Berge's condition independently before calling the result maximum.
            let Some(tenacity) = bottleneck else {
                if let Some(path) = self.shortest_augmenting_path(graph, graph.n.saturating_sub(1))
                {
                    return Err(MicaliVaziraniError::UncertifiedOptimality {
                        path_length: path.len() - 1,
                    });
                }
                break;
            };

            // Augment along a maximal collection of vertex-disjoint paths of length 'tenacity'
            let paths_augmented = self.run_augmentation_phase(graph, tenacity)?;
            if paths_augmented == 0 {
                return Err(MicaliVaziraniError::MissingAugmentation { tenacity });
            }
            self.total_augmentations += paths_augmented;
            self.total_blossoms_created += self.petals.len();
        }

        self.phase = Phase::Finished;
        Ok(self.build_matching_result(graph, start_time))
    }

    /// Convenience wrapper returning the matching pair slice.
    pub fn solve(&mut self, graph: &Graph) -> Result<&[Option<usize>], MicaliVaziraniError> {
        self.max_cardinality_matching(graph)?;
        Ok(&self.mate)
    }

    // =========================================================================
    // STAGE 1 & 2: Interleaved Parity BFS and Bridge Processing
    // =========================================================================

    /// Interleaves alternating parity level discovery with tenacity-ordered bridge processing.
    ///
    /// At each level `i`:
    /// 1. Vertices in `level_queues[i]` explore incident edges (unmatched if even, matched if odd).
    /// 2. Bridges of tenacity $\le 2i + 1$ are processed by DDFS.
    /// 3. Petals formed dynamically assign MAX levels to cycle vertices:
    ///    $$\text{max\_level}[w] = \text{tenacity} - \text{min\_level}[w]$$
    ///    and re-enqueue them into `level_queues[max_level]` to propagate reachability through blossoms.
    pub fn run_interleaved_search_phase(
        &mut self,
        graph: &Graph,
    ) -> Result<Option<usize>, MicaliVaziraniError> {
        self.validate_graph(graph)?;
        self.phase = Phase::LevelSearch;

        // Initialize exposed roots at even level 0
        for vertex in 0..self.vertex_count {
            if self.mate[vertex].is_none() {
                self.even_level[vertex] = Some(0);
                self.min_level[vertex] = Some(0);
                self.level_queues[0].push_back(vertex);
            }
        }

        let mut tenacity_queue = TenacityQueue::new(self.vertex_count);
        let mut bridge_queued = vec![false; graph.edges.len()];
        let max_possible_level = 2 * self.vertex_count + 1;
        let mut current_level = 0;

        while current_level <= max_possible_level {
            // Stop early if bottleneck tenacity L is found and current level exceeds (L - 1) / 2
            if let Some(bottleneck) = self.bottleneck_tenacity {
                if 2 * current_level + 1 > bottleneck {
                    break;
                }
            }

            let is_even_level = current_level % 2 == 0;

            // 1. Process vertices queued at current_level
            while let Some(u) = self.level_queues[current_level].pop_front() {
                if is_even_level {
                    // EVEN state: scan unmatched incident edges
                    for &edge_idx in graph.adj_edges(u) {
                        self.level_search_scans += 1;
                        let edge = &graph.edges[edge_idx];
                        let v = if edge.u == u { edge.v } else { edge.u };

                        // Alternating step must be unmatched
                        if self.mate[u] == Some(v) {
                            continue;
                        }

                        // BRIDGE CHECK: In Micali–Vazirani parity search, an unmatched edge
                        // connecting two EVEN states is a candidate bridge with tenacity:
                        // tenacity = even_level(u) + even_level(v) + 1
                        if let Some(v_even) = self.even_level[v] {
                            let tenacity = current_level + v_even + 1;
                            if !bridge_queued[edge_idx] {
                                bridge_queued[edge_idx] = true;
                                tenacity_queue.push(edge_idx, tenacity);
                                self.bridges.push(edge_idx);
                            }
                        } else if self.odd_level[v].is_none() {
                            let next_lvl = current_level + 1;
                            self.odd_level[v] = Some(next_lvl);
                            if self.min_level[v].is_none() {
                                self.min_level[v] = Some(next_lvl);
                            }
                            self.predecessor[v].push(edge_idx);
                            if next_lvl < self.level_queues.len() {
                                self.level_queues[next_lvl].push_back(v);
                            }
                        } else if self.odd_level[v] == Some(current_level + 1) {
                            // Parallel prop edge at the same level
                            self.predecessor[v].push(edge_idx);
                        }
                    }
                } else {
                    // ODD state: follow the unique matched edge
                    if let Some(v) = self.mate[u] {
                        self.level_search_scans += 1;
                        let next_lvl = current_level + 1;
                        // If the mate already has a shorter MIN-level path,
                        // this matched edge is a bridge, not a predecessor.
                        // In particular, two odd-level-one vertices reached
                        // from distinct free roots can be joined this way.
                        if self.min_level[v].is_some_and(|level| level < next_lvl) {
                            if let Some(edge_idx) = graph.edge_index(u, v)
                                && let Some(tenacity) = self.bridge_tenacity(graph, edge_idx)
                            {
                                if !bridge_queued[edge_idx] {
                                    bridge_queued[edge_idx] = true;
                                    tenacity_queue.push(edge_idx, tenacity);
                                    self.bridges.push(edge_idx);
                                }
                            }
                        } else if self.even_level[v].is_none() {
                            self.even_level[v] = Some(next_lvl);
                            if self.min_level[v].is_none() {
                                self.min_level[v] = Some(next_lvl);
                            }
                            if let Some(edge_idx) = graph.edge_index(u, v) {
                                self.predecessor[v].push(edge_idx);
                            }
                            if next_lvl < self.level_queues.len() {
                                self.level_queues[next_lvl].push_back(v);
                            }
                        } else if self.even_level[v] == Some(next_lvl) {
                            if let Some(edge_idx) = graph.edge_index(u, v) {
                                self.predecessor[v].push(edge_idx);
                            }
                        }
                    }
                }
            }

            // 2. Process bridges of tenacity <= 2 * current_level + 1
            self.phase = Phase::BridgeProcessing;
            let max_tenacity_now = 2 * current_level + 1;

            while let Some((edge_idx, tenacity)) = tenacity_queue.peek_min() {
                if tenacity > max_tenacity_now {
                    break;
                }
                tenacity_queue.pop_min();

                // Stop if bridge exceeds known bottleneck tenacity
                if let Some(bottleneck) = self.bottleneck_tenacity {
                    if tenacity > bottleneck {
                        break;
                    }
                }

                match self.ddfs(graph, edge_idx, tenacity) {
                    DdfsResult::Petal {
                        base,
                        cycle_vertices,
                        path_u,
                        path_v,
                    } => {
                        let petal_id = self.petals.len();
                        let petal = Petal {
                            id: petal_id,
                            base,
                            bridge_index: edge_idx,
                            tenacity,
                            cycle_vertices: cycle_vertices.clone(),
                            path_u,
                            path_v,
                        };
                        self.petals.push(petal);

                        // Assign MAX levels and dynamically enqueue activated vertices
                        for &w in &cycle_vertices {
                            if self.max_level[w].is_none() {
                                if let Some(min_lvl) = self.min_level[w] {
                                    if tenacity >= min_lvl {
                                        let new_max = tenacity - min_lvl;
                                        self.max_level[w] = Some(new_max);
                                        self.petal_by_vertex[w] = Some(petal_id);

                                        if new_max % 2 == 0 {
                                            self.even_level[w] = Some(new_max);
                                        } else {
                                            self.odd_level[w] = Some(new_max);
                                        }

                                        // Dynamic propagation through blossom
                                        if new_max > current_level
                                            && new_max < self.level_queues.len()
                                        {
                                            self.level_queues[new_max].push_back(w);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DdfsResult::AugmentingPath { bridge_edge, .. } => {
                        if self.bottleneck_tenacity.is_none() {
                            self.bottleneck_tenacity = Some(tenacity);
                        }
                        if !self.augmenting_bridges.contains(&bridge_edge) {
                            self.augmenting_bridges.push(bridge_edge);
                        }
                    }
                    DdfsResult::Redundant => {}
                }
            }

            current_level += 1;
        }

        self.derive_levels_and_edge_classes(graph);
        Ok(self.bottleneck_tenacity)
    }

    /// Preserved milestone execution for Criterion benchmarks and phase-1 tests.
    pub fn run_level_search_phase(&mut self, graph: &Graph) -> Result<(), MicaliVaziraniError> {
        self.validate_graph(graph)?;
        self.begin_phase();

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

    /// Preserved milestone execution for bridge processing benchmarks and tests.
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
                DdfsResult::Petal { cycle_vertices, .. } => {
                    self.assign_petal_max_levels(&cycle_vertices, tenacity)
                }
                DdfsResult::AugmentingPath { bridge_edge, .. } => {
                    if self.bottleneck_tenacity.is_none() {
                        self.bottleneck_tenacity = Some(tenacity);
                    }
                    if !self.augmenting_bridges.contains(&bridge_edge) {
                        self.augmenting_bridges.push(bridge_edge);
                    }
                }
                DdfsResult::Redundant => {}
            }
        }

        self.phase = Phase::Augmentation;
        Ok(self.bottleneck_tenacity)
    }

    // =========================================================================
    // DOUBLE DEPTH-FIRST SEARCH (DDFS) & PETAL CONTRACTION
    // =========================================================================

    /// Double Depth-First Search in lockstep backwards along predecessor edges.
    ///
    /// Guarantees O(cycle) work per petal by:
    /// - Alternating steps between Left and Right search frontiers;
    /// - Skipping already-contracted petals via `find_petal_base`;
    /// - Consuming predecessor edges destructively so no edge is rescanned in the phase.
    pub fn ddfs(&mut self, graph: &Graph, bridge_index: usize, tenacity: usize) -> DdfsResult {
        // A caller-supplied tenacity is not itself a certificate. Reject an
        // edge whose currently known parity levels disagree with it before
        // mutating the petal partition or predecessor scan pointers.
        if self.bridge_tenacity(graph, bridge_index) != Some(tenacity) || tenacity % 2 == 0 {
            return DdfsResult::Redundant;
        }
        let edge = &graph.edges[bridge_index];
        let mut curr_u = self.find_petal_base(edge.u);
        let mut curr_v = self.find_petal_base(edge.v);

        if curr_u == curr_v {
            return DdfsResult::Redundant;
        }

        // A bridge can have several predecessor choices. Before contracting
        // anything, try to produce an actual simple, alternating path of the
        // prescribed tenacity through this bridge. A single arbitrary DDFS
        // predecessor branch may meet at a blossom base even though another
        // branch reaches a distinct exposed root (the four-vertex path is the
        // smallest example). This certificate search prioritizes correctness;
        // its cost is not covered by the claimed MV linear-per-phase bound.
        if let Some(path) =
            self.certified_augmenting_path(graph, bridge_index, tenacity, &vec![false; graph.n])
        {
            let (left_path, right_path) = self.path_halves_around_bridge(&path, edge.u, edge.v);
            return DdfsResult::AugmentingPath {
                bridge_edge: bridge_index,
                root_left: *left_path.last().expect("certified path has a left root"),
                root_right: *right_path.last().expect("certified path has a right root"),
                left_path,
                right_path,
            };
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
            // Step Search Left
            if u_active {
                if let Some(pred) = self.advance_ddfs_step(graph, curr_u) {
                    let base_pred = self.find_petal_base(pred);
                    path_u.push(base_pred);

                    if self.ddfs_markers[base_pred].epoch == epoch {
                        if self.ddfs_markers[base_pred].source == VisitSource::SearchRight {
                            // Cycle in circular order: base_pred down path_u to u, bridge (u, v), then up path_v to base_pred
                            let mut cycle_vertices = Vec::new();
                            for &w in path_u.iter().rev() {
                                cycle_vertices.push(w);
                            }
                            for &w in &path_v {
                                if w != base_pred {
                                    cycle_vertices.push(w);
                                }
                            }
                            if let Some(cycle_vertices) = self.certified_petal_cycle(
                                graph,
                                bridge_index,
                                tenacity,
                                base_pred,
                                cycle_vertices,
                            ) {
                                self.union_petal(base_pred, &cycle_vertices);
                                return DdfsResult::Petal {
                                    base: base_pred,
                                    cycle_vertices,
                                    path_u,
                                    path_v,
                                };
                            }
                            return DdfsResult::Redundant;
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

            // Step Search Right (alternating lockstep)
            if v_active {
                if let Some(pred) = self.advance_ddfs_step(graph, curr_v) {
                    let base_pred = self.find_petal_base(pred);
                    path_v.push(base_pred);

                    if self.ddfs_markers[base_pred].epoch == epoch {
                        if self.ddfs_markers[base_pred].source == VisitSource::SearchLeft {
                            // Cycle in circular order: base_pred down path_v to v, bridge (v, u), then up path_u to base_pred
                            let mut cycle_vertices = Vec::new();
                            for &w in path_v.iter().rev() {
                                cycle_vertices.push(w);
                            }
                            for &w in &path_u {
                                if w != base_pred {
                                    cycle_vertices.push(w);
                                }
                            }
                            if let Some(cycle_vertices) = self.certified_petal_cycle(
                                graph,
                                bridge_index,
                                tenacity,
                                base_pred,
                                cycle_vertices,
                            ) {
                                self.union_petal(base_pred, &cycle_vertices);
                                return DdfsResult::Petal {
                                    base: base_pred,
                                    cycle_vertices,
                                    path_u,
                                    path_v,
                                };
                            }
                            return DdfsResult::Redundant;
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

            // Exhausting two arbitrary predecessor branches is not by itself
            // an augmenting-path certificate. The certified search above
            // already tried every compatible branch through this bridge.
            if !u_active && !v_active {
                return DdfsResult::Redundant;
            }
        }
    }

    /// Check the explicit odd cycle before treating it as a contracted petal.
    /// In particular, a DSU representative sequence that skips internal edges
    /// of a nested blossom is *not* silently accepted as a cycle in G.
    fn valid_petal_cycle(
        &self,
        graph: &Graph,
        bridge_index: usize,
        base: usize,
        cycle: &[usize],
    ) -> bool {
        if cycle.len() < 3 || cycle.len() % 2 == 0 || !cycle.contains(&base) {
            return false;
        }
        let mut seen = vec![false; graph.n];
        for &vertex in cycle {
            if vertex >= graph.n || seen[vertex] {
                return false;
            }
            seen[vertex] = true;
        }
        let bridge = &graph.edges[bridge_index];
        let mut contains_bridge = false;
        for i in 0..cycle.len() {
            let u = cycle[i];
            let v = cycle[(i + 1) % cycle.len()];
            if graph.edge_index(u, v).is_none() {
                return false;
            }
            if (u == bridge.u && v == bridge.v) || (u == bridge.v && v == bridge.u) {
                contains_bridge = true;
            }
            // Every non-base vertex must have exactly one matched cycle edge;
            // the base may be matched outside the petal, but not within it.
            if u != base {
                let prev = cycle[(i + cycle.len() - 1) % cycle.len()];
                let matched_incident =
                    usize::from(self.mate[u] == Some(prev)) + usize::from(self.mate[u] == Some(v));
                if matched_incident != 1 {
                    return false;
                }
            } else if self.mate[u] == Some(v)
                || self.mate[u] == Some(cycle[(i + cycle.len() - 1) % cycle.len()])
            {
                return false;
            }
        }
        contains_bridge
    }

    /// A quotient-graph DDFS can jump over the interior of an existing petal.
    /// Its representative sequence then is not an original-graph cycle. To
    /// certify the contraction, lift that sequence by looking for a concrete
    /// odd alternating cycle with the same base and bridge. This is a
    /// correctness-first reconstruction and can be expensive; it is not the
    /// incremental-tree/stack set-merging procedure of linear-time MV.
    fn certified_petal_cycle(
        &self,
        graph: &Graph,
        bridge_index: usize,
        tenacity: usize,
        base: usize,
        representative_cycle: Vec<usize>,
    ) -> Option<Vec<usize>> {
        let base_distance = self.min_level[base]?;
        if base_distance % 2 != 0 {
            return None;
        }
        let cycle_len = tenacity.checked_sub(base_distance.checked_mul(2)?)?;
        if cycle_len < 3 || cycle_len % 2 == 0 || cycle_len > graph.n {
            return None;
        }
        if representative_cycle.len() == cycle_len
            && self.valid_petal_cycle(graph, bridge_index, base, &representative_cycle)
        {
            return Some(representative_cycle);
        }
        let mut seen = vec![false; graph.n];
        seen[base] = true;
        let mut path = vec![base];
        self.find_petal_cycle(
            graph,
            bridge_index,
            base,
            base,
            cycle_len,
            false,
            &mut seen,
            &mut path,
        )
    }

    fn find_petal_cycle(
        &self,
        graph: &Graph,
        bridge_index: usize,
        base: usize,
        current: usize,
        edges_remaining: usize,
        expect_matched: bool,
        seen: &mut [bool],
        path: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        let bridge = &graph.edges[bridge_index];
        for &edge_index in graph.adj_edges(current) {
            let edge = &graph.edges[edge_index];
            let next = if edge.u == current { edge.v } else { edge.u };
            if (self.mate[current] == Some(next)) != expect_matched {
                continue;
            }
            if edges_remaining == 1 {
                if next != base {
                    continue;
                }
                let contains_bridge = path.windows(2).any(|pair| {
                    (pair[0] == bridge.u && pair[1] == bridge.v)
                        || (pair[0] == bridge.v && pair[1] == bridge.u)
                }) || (current == bridge.u && base == bridge.v)
                    || (current == bridge.v && base == bridge.u);
                if contains_bridge && self.valid_petal_cycle(graph, bridge_index, base, path) {
                    return Some(path.clone());
                }
            } else if next != base && !seen[next] {
                seen[next] = true;
                path.push(next);
                if let Some(cycle) = self.find_petal_cycle(
                    graph,
                    bridge_index,
                    base,
                    next,
                    edges_remaining - 1,
                    !expect_matched,
                    seen,
                    path,
                ) {
                    return Some(cycle);
                }
                path.pop();
                seen[next] = false;
            }
        }
        None
    }

    fn advance_ddfs_step(&mut self, graph: &Graph, vertex: usize) -> Option<usize> {
        if self.ddfs_pred_ptr[vertex] < self.predecessor[vertex].len() {
            let edge_idx = self.predecessor[vertex][self.ddfs_pred_ptr[vertex]];
            self.ddfs_pred_ptr[vertex] += 1;
            let edge = &graph.edges[edge_idx];
            let neighbor = if edge.u == vertex { edge.v } else { edge.u };
            return Some(neighbor);
        }
        None
    }

    fn assign_petal_max_levels(&mut self, vertices: &[usize], tenacity: usize) {
        for &v in vertices {
            if let Some(min_lvl) = self.min_level[v] {
                if tenacity >= min_lvl {
                    let new_max = tenacity - min_lvl;
                    if self.max_level[v].is_none_or(|old| new_max > old) {
                        self.max_level[v] = Some(new_max);
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

    // =========================================================================
    // STAGE 3: Augmentation Engine & Maximal Vertex-Disjoint Paths
    // =========================================================================

    /// Stage 3: The Augmentation Engine (Finding Maximal Disjoint Paths).
    ///
    /// ## Algorithmic Purpose
    /// In the Micali–Vazirani (and Hopcroft–Karp) framework, achieving the $O(\sqrt{V} E)$ bound
    /// requires that each phase finds a **maximal collection of vertex-disjoint shortest augmenting
    /// paths** of the current bottleneck tenacity $L = t_{\min}$.
    ///
    /// ## Invariant & Bound
    /// 1. **Shortest Path Monotonicity**: Augmenting along a maximal collection of vertex-disjoint
    ///    paths of length $L$ guarantees that in the subsequent phase, the length of the shortest
    ///    augmenting path strictly increases: $L' > L$.
    /// 2. **Phase Count Bound**: Because path lengths are odd integers bounded by $V$, there are
    ///    at most $O(\sqrt{V})$ distinct phases before the matching is guaranteed to be maximum.
    /// 3. **Disjointness Invariant**: Vertices included in any augmented path are marked in
    ///    `committed_in_aug`. Any subsequent candidate bridge whose unrolled path intersects
    ///    an already committed vertex is bypassed, ensuring non-overlapping augmentations.
    pub fn run_augmentation_phase(
        &mut self,
        graph: &Graph,
        bottleneck_tenacity: usize,
    ) -> Result<usize, MicaliVaziraniError> {
        self.phase = Phase::Augmentation;
        let mut augmentations = 0;

        let candidate_bridges = self.augmenting_bridges.clone();
        for bridge_idx in candidate_bridges {
            let edge = &graph.edges[bridge_idx];
            // Skip bridges whose endpoints were already claimed by an earlier disjoint path
            if self.committed_in_aug[edge.u] || self.committed_in_aug[edge.v] {
                continue;
            }

            // Attempt to unroll a complete alternating path to distinct exposed roots
            if let Some(path) = self.certified_augmenting_path(
                graph,
                bridge_idx,
                bottleneck_tenacity,
                &self.committed_in_aug,
            ) {
                // Ensure strictly vertex-disjoint paths in this phase
                if path.iter().all(|&w| !self.committed_in_aug[w]) {
                    // A path of the queue's least bridge tenacity is only a
                    // *candidate* shortest path. Verify that no shorter
                    // augmenting path exists in the current matching before
                    // modifying any mate. This is intentionally exact and
                    // expensive while the MV layering proof is unfinished.
                    if bottleneck_tenacity > 1
                        && let Some(shorter) =
                            self.shortest_augmenting_path(graph, bottleneck_tenacity - 2)
                    {
                        return Err(MicaliVaziraniError::NonShortestAugmentation {
                            proposed: bottleneck_tenacity,
                            actual: shorter.len() - 1,
                        });
                    }
                    self.apply_augmentation(&path);
                    augmentations += 1;
                }
            }
        }

        Ok(augmentations)
    }

    /// Independent certificate oracle, used only to *reject* invalid phase
    /// claims. It never supplies a replacement augmentation to the MV search.
    /// Enumerating simple alternating paths is exponential in the worst case.
    fn shortest_augmenting_path(&self, graph: &Graph, maximum_length: usize) -> Option<Vec<usize>> {
        if maximum_length == 0 || self.mate.iter().filter(|mate| mate.is_none()).count() < 2 {
            return None;
        }
        let upper = maximum_length.min(graph.n.saturating_sub(1));
        for length in (1..=upper).step_by(2) {
            for root in 0..graph.n {
                if self.mate[root].is_some() {
                    continue;
                }
                let mut seen = vec![false; graph.n];
                seen[root] = true;
                let mut path = vec![root];
                if self.find_exact_augmenting_path(graph, root, length, false, &mut seen, &mut path)
                {
                    return Some(path);
                }
            }
        }
        None
    }

    /// `expect_matched` describes the next edge, not the parity of `current`.
    /// The visited set enforces the simple-path requirement that parity-state
    /// BFS alone cannot enforce in a general (non-bipartite) graph.
    fn find_exact_augmenting_path(
        &self,
        graph: &Graph,
        current: usize,
        remaining: usize,
        expect_matched: bool,
        seen: &mut [bool],
        path: &mut Vec<usize>,
    ) -> bool {
        if remaining == 0 {
            return self.mate[current].is_none();
        }
        for &edge_index in graph.adj_edges(current) {
            let edge = &graph.edges[edge_index];
            let next = if edge.u == current { edge.v } else { edge.u };
            if seen[next] || (self.mate[current] == Some(next)) != expect_matched {
                continue;
            }
            seen[next] = true;
            path.push(next);
            if self.find_exact_augmenting_path(
                graph,
                next,
                remaining - 1,
                !expect_matched,
                seen,
                path,
            ) {
                return true;
            }
            path.pop();
            seen[next] = false;
        }
        false
    }

    /// Reconstructs the complete unrolled alternating path connecting two exposed roots
    /// across the augmenting bridge edge.
    ///
    /// ## Parity Alignment
    /// An *unmatched* bridge joins two even alternating states, so backward
    /// traversal on either side begins with a matched edge. A *matched*
    /// bridge joins two odd states, so backward traversal begins with an
    /// unmatched edge. Either way, the two halves must lead to distinct free
    /// roots, and the spliced path is subsequently checked edge by edge.
    fn reconstruct_augmenting_path(
        &self,
        graph: &Graph,
        bridge_index: usize,
    ) -> Option<Vec<usize>> {
        let edge = &graph.edges[bridge_index];
        let (u, v) = (edge.u, edge.v);

        let mut visited_u = vec![false; self.vertex_count];
        visited_u[u] = true;
        visited_u[v] = true;

        // An unmatched bridge joins two EVEN paths; a matched bridge joins two
        // ODD paths. Backward reconstruction must begin with the corresponding
        // edge parity or it will reuse the bridge as an ordinary matched step.
        let bridge_is_matched = self.mate[u] == Some(v);
        let path_u = self.find_path_to_root(graph, u, !bridge_is_matched, &mut visited_u)?;

        // Ensure path_v is vertex-disjoint from path_u
        let mut visited_v = vec![false; self.vertex_count];
        for &w in &path_u {
            visited_v[w] = true;
        }
        visited_v[v] = true;

        let path_v = self.find_path_to_root(graph, v, !bridge_is_matched, &mut visited_v)?;

        let root_u = path_u.first()?;
        let root_v = path_v.first()?;
        if root_u == root_v || self.mate[*root_u].is_some() || self.mate[*root_v].is_some() {
            return None;
        }

        // Full path: root_u -> ... -> u -> v -> ... -> root_v
        let mut full_path = path_u;
        for &w in path_v.iter().rev() {
            full_path.push(w);
        }

        Some(full_path)
    }

    /// Accept an augmentation only if it has a concrete certificate: a simple
    /// path of the current phase's shortest length, with exposed endpoints,
    /// the requested bridge, and correctly alternating matched status.
    ///
    /// The existing prop/petal reconstruction is tried first. If it chooses
    /// two paths that meet at a base even though another predecessor choice
    /// would reach distinct roots, the second search enumerates those choices.
    /// That second search is a correctness-first measure, not a claim of MV's
    /// O(E) phase bound; replacing it with certified linear-time DDFS remains
    /// separate algorithmic work.
    fn certified_augmenting_path(
        &self,
        graph: &Graph,
        bridge_index: usize,
        tenacity: usize,
        forbidden: &[bool],
    ) -> Option<Vec<usize>> {
        if let Some(path) = self.reconstruct_augmenting_path(graph, bridge_index) {
            if self.valid_augmenting_path(graph, bridge_index, tenacity, forbidden, &path) {
                return Some(path);
            }
        }

        let bridge = graph.edges.get(bridge_index)?;
        let (u, v) = (bridge.u, bridge.v);
        let bridge_matched = self.mate[u] == Some(v);
        let (u_length, v_length) = if bridge_matched {
            (self.odd_level[u]?, self.odd_level[v]?)
        } else {
            (self.even_level[u]?, self.even_level[v]?)
        };
        if u_length.checked_add(v_length)?.checked_add(1)? != tenacity
            || tenacity >= graph.n
            || forbidden.get(u).copied().unwrap_or(true)
            || forbidden.get(v).copied().unwrap_or(true)
        {
            return None;
        }

        let mut seen_u = forbidden.to_vec();
        seen_u[u] = true;
        seen_u[v] = true;
        let mut backward_u = vec![u];
        let mut answer = None;
        self.enumerate_half_paths(
            graph,
            u,
            u_length,
            !bridge_matched,
            &mut seen_u,
            &mut backward_u,
            &mut |half_u| {
                let mut seen_v = forbidden.to_vec();
                for &vertex in half_u {
                    seen_v[vertex] = true;
                }
                seen_v[v] = true;
                let mut backward_v = vec![v];
                self.enumerate_half_paths(
                    graph,
                    v,
                    v_length,
                    !bridge_matched,
                    &mut seen_v,
                    &mut backward_v,
                    &mut |half_v| {
                        let mut path: Vec<usize> = half_u.iter().rev().copied().collect();
                        path.extend_from_slice(half_v);
                        if self.valid_augmenting_path(
                            graph,
                            bridge_index,
                            tenacity,
                            forbidden,
                            &path,
                        ) {
                            answer = Some(path);
                            true
                        } else {
                            false
                        }
                    },
                )
            },
        );
        answer
    }

    /// Enumerate simple alternating half-paths backwards from one bridge end.
    /// `remaining` counts edges to an exposed root; `expect_matched` specifies
    /// the status of the next edge while walking backwards.
    fn enumerate_half_paths<F: FnMut(&[usize]) -> bool>(
        &self,
        graph: &Graph,
        current: usize,
        remaining: usize,
        expect_matched: bool,
        seen: &mut [bool],
        backward: &mut Vec<usize>,
        on_root: &mut F,
    ) -> bool {
        if remaining == 0 {
            return self.mate[current].is_none() && on_root(backward);
        }
        for &edge_index in graph.adj_edges(current) {
            let edge = &graph.edges[edge_index];
            let next = if edge.u == current { edge.v } else { edge.u };
            if next == current || seen[next] || (self.mate[current] == Some(next)) != expect_matched
            {
                continue;
            }
            seen[next] = true;
            backward.push(next);
            if self.enumerate_half_paths(
                graph,
                next,
                remaining - 1,
                !expect_matched,
                seen,
                backward,
                on_root,
            ) {
                return true;
            }
            backward.pop();
            seen[next] = false;
        }
        false
    }

    fn valid_augmenting_path(
        &self,
        graph: &Graph,
        bridge_index: usize,
        tenacity: usize,
        forbidden: &[bool],
        path: &[usize],
    ) -> bool {
        if tenacity >= graph.n || path.len() != tenacity + 1 || path.len() < 2 || tenacity % 2 == 0
        {
            return false;
        }
        let mut seen = vec![false; graph.n];
        for &vertex in path {
            if vertex >= graph.n || forbidden[vertex] || seen[vertex] {
                return false;
            }
            seen[vertex] = true;
        }
        if self.mate[path[0]].is_some() || self.mate[*path.last().unwrap()].is_some() {
            return false;
        }
        let bridge = &graph.edges[bridge_index];
        let mut includes_bridge = false;
        for (index, pair) in path.windows(2).enumerate() {
            let (u, v) = (pair[0], pair[1]);
            if graph.edge_index(u, v).is_none() || (self.mate[u] == Some(v)) != (index % 2 == 1) {
                return false;
            }
            if (u == bridge.u && v == bridge.v) || (u == bridge.v && v == bridge.u) {
                includes_bridge = true;
            }
        }
        includes_bridge
    }

    fn path_halves_around_bridge(
        &self,
        path: &[usize],
        u: usize,
        v: usize,
    ) -> (Vec<usize>, Vec<usize>) {
        let split = path
            .windows(2)
            .position(|pair| (pair[0] == u && pair[1] == v) || (pair[0] == v && pair[1] == u))
            .expect("certified augmenting path contains its bridge");
        if path[split] == u {
            (
                path[..=split].iter().rev().copied().collect(),
                path[split + 1..].to_vec(),
            )
        } else {
            (
                path[split + 1..].to_vec(),
                path[..=split].iter().rev().copied().collect(),
            )
        }
    }

    /// Returns the alternating walk along the petal cycle from `curr` to `petal.base`.
    ///
    /// If `first_edge_matched == true`, the first edge leaving `curr` must be MATCHED.
    /// If `first_edge_matched == false`, the first edge leaving `curr` must be UNMATCHED.
    ///
    /// Because the petal is an odd alternating cycle with `base` as its unmatched vertex,
    /// exactly one direction around the cycle alternates correctly and terminates at `base`.
    fn unroll_petal_cycle_to_base(
        &self,
        petal: &Petal,
        curr: usize,
        first_edge_matched: bool,
    ) -> Option<Vec<usize>> {
        if curr == petal.base {
            return Some(vec![curr]);
        }
        let cycle = &petal.cycle_vertices;
        let n_c = cycle.len();
        if n_c < 3 {
            return None;
        }
        let curr_idx = cycle.iter().position(|&v| v == curr)?;

        let next_idx = (curr_idx + 1) % n_c;
        let prev_idx = (curr_idx + n_c - 1) % n_c;

        let next_v = cycle[next_idx];
        let prev_v = cycle[prev_idx];

        let next_matched = self.mate[curr] == Some(next_v);
        let prev_matched = self.mate[curr] == Some(prev_v);

        // Determine step direction along the cycle (+1 for forward, -1 for backward)
        let step_forward = if first_edge_matched {
            if next_matched {
                true
            } else if prev_matched {
                false
            } else {
                return None;
            }
        } else {
            if !next_matched {
                true
            } else if !prev_matched {
                false
            } else {
                return None;
            }
        };

        let mut path = vec![curr];
        let mut idx = curr_idx;
        let mut expect_matched = first_edge_matched;

        while cycle[idx] != petal.base {
            let next_i = if step_forward {
                (idx + 1) % n_c
            } else {
                (idx + n_c - 1) % n_c
            };
            let u = cycle[idx];
            let v = cycle[next_i];
            let is_matched = self.mate[u] == Some(v);
            if is_matched != expect_matched {
                return None;
            }
            path.push(v);
            idx = next_i;
            expect_matched = !expect_matched;
        }

        Some(path)
    }

    /// Backtracking search finding an alternating walk to an exposed root along tight edges.
    fn find_path_to_root(
        &self,
        graph: &Graph,
        curr: usize,
        need_even: bool,
        visited: &mut [bool],
    ) -> Option<Vec<usize>> {
        if need_even {
            if self.mate[curr].is_none() {
                // Base case: exposed root
                return Some(vec![curr]);
            }
            let partner = self.mate[curr]?;
            if visited[partner] {
                return None;
            }
            visited[partner] = true;
            let mut path = self.find_path_to_root(graph, partner, false, visited)?;
            path.push(curr);
            Some(path)
        } else {
            // Need an odd step into curr (must come via an unmatched edge).
            // Option A: follow direct predecessor props
            for &edge_idx in &self.predecessor[curr] {
                let edge = &graph.edges[edge_idx];
                let p = if edge.u == curr { edge.v } else { edge.u };

                if self.mate[curr] == Some(p) {
                    continue;
                }
                if !visited[p] && self.even_level[p].is_some() {
                    let p_level = self.even_level[p].unwrap();
                    let curr_odd = self.odd_level[curr].unwrap_or(usize::MAX);
                    if p_level < curr_odd {
                        visited[p] = true;
                        if let Some(mut path) = self.find_path_to_root(graph, p, true, visited) {
                            path.push(curr);
                            return Some(path);
                        }
                        visited[p] = false;
                    }
                }
            }

            // Also check all incident unmatched edges as fallback
            for &edge_idx in graph.adj_edges(curr) {
                let edge = &graph.edges[edge_idx];
                let p = if edge.u == curr { edge.v } else { edge.u };
                if self.mate[curr] == Some(p) {
                    continue;
                }
                if !visited[p] && self.even_level[p].is_some() {
                    let p_level = self.even_level[p].unwrap();
                    let curr_odd = self.odd_level[curr].unwrap_or(usize::MAX);
                    if p_level < curr_odd {
                        visited[p] = true;
                        if let Some(mut path) = self.find_path_to_root(graph, p, true, visited) {
                            path.push(curr);
                            return Some(path);
                        }
                        visited[p] = false;
                    }
                }
            }

            // Option B: if curr is inside a petal, route around the odd cycle via the petal bridge
            if let Some(petal_id) = self.petal_by_vertex[curr] {
                let petal = &self.petals[petal_id];
                if let Some(cycle_walk) = self.unroll_petal_cycle_to_base(petal, curr, false) {
                    let can_use = cycle_walk
                        .iter()
                        .skip(1)
                        .filter(|&&v| v != petal.base)
                        .all(|&v| !visited[v]);
                    if can_use {
                        for &v in cycle_walk.iter().skip(1).filter(|&&v| v != petal.base) {
                            visited[v] = true;
                        }
                        if let Some(mut base_path) =
                            self.find_path_to_root(graph, petal.base, true, visited)
                        {
                            // Append cycle walk in reverse (from neighbor of base back to curr)
                            for &v in cycle_walk.iter().rev().skip(1) {
                                base_path.push(v);
                            }
                            return Some(base_path);
                        }
                        for &v in cycle_walk.iter().skip(1).filter(|&&v| v != petal.base) {
                            visited[v] = false;
                        }
                    }
                }
            }

            None
        }
    }

    /// Augments the matching along a path, flipping matched and unmatched edges.
    fn apply_augmentation(&mut self, path: &[usize]) {
        for chunk in path.chunks_exact(2) {
            let u = chunk[0];
            let v = chunk[1];
            self.mate[u] = Some(v);
            self.mate[v] = Some(u);
        }
        for &w in path {
            self.committed_in_aug[w] = true;
        }
    }

    // =========================================================================
    // STATE MANAGEMENT & DERIVATION HELPERS
    // =========================================================================

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
        self.petals.clear();
        self.petal_by_vertex.fill(None);
        for q in &mut self.level_queues {
            q.clear();
        }
        self.committed_in_aug.fill(false);
        self.ddfs_pred_ptr.fill(0);
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
        self.bridges.sort_unstable();
        self.bridges.dedup();
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
            if !self.predecessor[to].contains(&edge_index) {
                self.predecessor[to].push(edge_index);
            }
            is_prop[edge_index] = true;
        }
    }

    fn build_matching_result(&self, graph: &Graph, start_time: Instant) -> MatchingResult {
        let mut edges = Vec::new();
        let mut scanned = vec![false; self.vertex_count];

        for u in 0..self.vertex_count {
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
            is_perfect: edge_count * 2 == self.vertex_count,
            metadata: MatchingMetadata {
                iterations: self.iterations,
                augmentations: self.total_augmentations,
                blossoms_created: self.total_blossoms_created,
                blossoms_expanded: 0,
                dual_updates: 0,
                runtime_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            },
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

#[cfg(test)]
mod certificate_tests {
    use super::*;
    use crate::graph::Edge;

    #[test]
    fn independent_shortest_path_check_rejects_a_longer_candidate() {
        // Two free roots have both a direct edge and a length-five alternating
        // route. The latter is valid, but must not be flipped in a shortest-
        // augmenting-path phase while the direct edge remains available.
        let graph = Graph::from_edges(
            6,
            [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (0, 5)]
                .into_iter()
                .map(|(u, v)| Edge::new(u, v, None))
                .collect(),
        );
        let solver = MicaliVazirani::with_matching(
            &graph,
            vec![None, Some(2), Some(1), Some(4), Some(3), None],
        )
        .unwrap();
        assert_eq!(solver.shortest_augmenting_path(&graph, 5), Some(vec![0, 5]));
        assert!(solver.valid_augmenting_path(&graph, 2, 5, &[false; 6], &[0, 1, 2, 3, 4, 5],));
        assert!(!solver.valid_augmenting_path(&graph, 2, 5, &[false; 6], &[0, 1, 2, 1, 4, 5],));
    }
}
