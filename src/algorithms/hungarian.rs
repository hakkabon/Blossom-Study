//! Hungarian-style primal-dual solver for bipartite MWPM (assignment).
//!
//! This module deliberately lives beside, rather than inside, the general-
//! graph blossom solvers.  Bipartite matching has a simpler linear-programming
//! dual: it needs one potential per vertex, but no dual variables for odd sets.
//! The contrast makes the Hungarian method a useful educational baseline for
//! measuring the bookkeeping cost of general-graph blossoms.
//!
//! # Input convention
//!
//! [`HungarianSolver::new`] receives the size `k` of the left partition.  The
//! graph must contain exactly `2k` vertices:
//!
//! ```text
//! left  = 0 .. k
//! right = k .. 2k
//! ```
//!
//! Every usable edge must cross this partition.  The graph may be sparse and
//! may contain parallel edges; the common filtering policy retains the cheapest
//! parallel edge.  A missing matrix entry is kept as `None`, not represented by
//! a dangerously large synthetic cost.
//!
//! # Primal-dual view
//!
//! For a cost matrix `c[i][j]`, the assignment LP has row and column dual
//! potentials.  The solver grows one alternating tree at a time.  `min_slack[j]`
//! records the cheapest reduced-cost way to reach an unused right vertex.  A
//! dual update subtracts the smallest such slack, making at least one new edge
//! tight.  Once the tree reaches a free right vertex, `previous_right` encodes
//! the augmenting path and the assignment is flipped along it.
//!
//! This shortest-augmenting-path presentation is a standard O(k^3) form of the
//! Hungarian method.  Exact binary scaling converts finite `f64` costs to
//! `i128`, so all reduced-cost comparisons are exact.

use std::time::Instant;

use super::mwpm::{WeightScaler, filter_edges};
use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

/// Bipartite assignment solver using contiguous left and right partitions.
#[derive(Clone, Copy, Debug)]
pub struct HungarianSolver {
    left_size: usize,
}

impl HungarianSolver {
    /// Construct a solver for `left_size + left_size` vertices.
    pub fn new(left_size: usize) -> Self {
        Self { left_size }
    }

    /// Find a minimum-weight perfect matching across the declared partition.
    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        hungarian_minimum_weight_perfect_matching(graph, self.left_size)
    }
}

/// Solve balanced bipartite MWPM with the partition split at `left_size`.
pub fn hungarian_minimum_weight_perfect_matching(
    graph: &Graph,
    left_size: usize,
) -> Result<MatchingResult, String> {
    let started = Instant::now();
    let expected_vertices = left_size
        .checked_mul(2)
        .ok_or_else(|| "bipartition size overflowed usize".to_string())?;
    if graph.n != expected_vertices {
        return Err(format!(
            "Hungarian MWPM requires two equal partitions: left_size={left_size}, graph.n={}",
            graph.n
        ));
    }
    if left_size == 0 {
        return Ok(MatchingResult {
            edges: Vec::new(),
            total_weight: 0.0,
            is_perfect: true,
            metadata: MatchingMetadata::default(),
        });
    }

    let filtered = filter_edges(graph)?;

    // Each cell stores both its exact scaled cost and the corresponding index
    // in the caller's graph.  Retaining the original index avoids a second
    // endpoint lookup after the assignment has been recovered.
    let mut matrix = vec![vec![None; left_size]; left_size];
    let weights: Vec<f64> = filtered.iter().map(|edge| edge.weight).collect();
    let scaler = WeightScaler::new(&weights);
    let mut scaled = Vec::with_capacity(filtered.len());
    for edge in &filtered {
        let u_is_left = edge.u < left_size;
        let v_is_left = edge.v < left_size;
        if u_is_left == v_is_left {
            return Err(format!(
                "edge ({}, {}) does not cross the declared bipartition 0..{left_size} | {left_size}..{}",
                edge.u, edge.v, graph.n
            ));
        }

        let (left, right_vertex) = if u_is_left {
            (edge.u, edge.v)
        } else {
            (edge.v, edge.u)
        };
        let right = right_vertex - left_size;
        scaled.push((
            left,
            right,
            edge.original_index,
            scaler.to_integer(edge.weight)?,
        ));
    }

    if scaled.is_empty() {
        return Err("bipartite graph has no crossing edges".to_string());
    }

    // Adding the same constant to every available edge does not change the
    // optimum because every perfect assignment contains exactly k edges.  The
    // normalization makes all costs non-negative and lets the dual search start
    // from zero potentials without violating reduced-cost feasibility.
    let minimum = scaled
        .iter()
        .map(|entry| entry.3)
        .min()
        .expect("non-empty scaled edge list");
    let maximum = scaled
        .iter()
        .map(|entry| entry.3)
        .max()
        .expect("non-empty scaled edge list");
    let range = maximum
        .checked_sub(minimum)
        .ok_or_else(|| "bipartite cost range overflowed i128".to_string())?;
    let potential_bound = left_size
        .checked_add(1)
        .and_then(|size| i128::try_from(size).ok())
        .and_then(|size| range.checked_mul(size))
        .ok_or_else(|| "bipartite costs leave insufficient i128 dual headroom".to_string())?;
    if potential_bound > i128::MAX / 4 {
        return Err("bipartite costs leave insufficient i128 dual headroom".to_string());
    }

    for (left, right, original_index, cost) in scaled {
        let normalized = cost
            .checked_sub(minimum)
            .ok_or_else(|| "bipartite cost normalization underflowed".to_string())?;
        matrix[left][right] = Some((normalized, original_index));
    }

    let (assignment, iterations, dual_updates) = solve_assignment(&matrix)?;
    let mut matched_edges = Vec::with_capacity(left_size);
    for (left, right) in assignment.into_iter().enumerate() {
        let (_, original_index) = matrix[left][right].ok_or_else(|| {
            format!("internal error: assignment selected missing cell ({left}, {right})")
        })?;
        matched_edges.push(original_index);
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
            iterations,
            augmentations: left_size,
            blossoms_created: 0,
            blossoms_expanded: 0,
            dual_updates,
            runtime_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    })
}

/// Hungarian shortest-augmenting-path kernel.
///
/// The arrays use the traditional one-based presentation.  Slot zero is a
/// sentinel representing the currently exposed right vertex.  `right_mate[j]`
/// is the one-based row assigned to column `j`; consequently
/// `right_mate[0]` can temporarily hold the row whose augmentation is underway.
fn solve_assignment(
    cost: &[Vec<Option<(i128, usize)>>],
) -> Result<(Vec<usize>, usize, usize), String> {
    let size = cost.len();
    let mut row_potential = vec![0_i128; size + 1];
    let mut column_potential = vec![0_i128; size + 1];
    let mut right_mate = vec![0_usize; size + 1];
    let mut previous_right = vec![0_usize; size + 1];
    let mut iterations = 0_usize;
    let mut dual_updates = 0_usize;

    for new_row in 1..=size {
        right_mate[0] = new_row;
        let mut current_right = 0_usize;
        let mut min_slack = vec![None::<i128>; size + 1];
        let mut used_right = vec![false; size + 1];

        // Grow an alternating tree until it reaches an unassigned column.
        loop {
            used_right[current_right] = true;
            let current_row = right_mate[current_right];
            let mut delta: Option<i128> = None;
            let mut next_right = 0_usize;

            for right in 1..=size {
                if used_right[right] {
                    continue;
                }
                iterations += 1;

                if let Some((edge_cost, _)) = cost[current_row - 1][right - 1] {
                    let reduced_cost = edge_cost
                        .checked_sub(row_potential[current_row])
                        .and_then(|value| value.checked_sub(column_potential[right]))
                        .ok_or_else(|| {
                            "Hungarian reduced-cost arithmetic overflowed".to_string()
                        })?;

                    if min_slack[right].is_none_or(|slack| reduced_cost < slack) {
                        min_slack[right] = Some(reduced_cost);
                        previous_right[right] = current_right;
                    }
                }

                // Even when the current row has no edge to this column, an
                // earlier row in the alternating tree may already have given
                // the column finite slack.  It must remain eligible as the next
                // tight event; otherwise sparse feasible instances can be
                // reported infeasible merely because their matrix has a hole.
                if let Some(slack) = min_slack[right]
                    && delta.is_none_or(|best| slack < best)
                {
                    delta = Some(slack);
                    next_right = right;
                }
            }

            // If no unused column is reachable, Hall's condition fails for the
            // currently grown set of rows; no perfect assignment exists.
            let delta = delta.ok_or_else(|| {
                format!(
                    "bipartite graph has no perfect matching (augmentation for left vertex {} is blocked)",
                    new_row - 1
                )
            })?;
            if delta < 0 {
                return Err(format!(
                    "internal error: negative Hungarian slack {delta} violated dual feasibility"
                ));
            }

            // Shift potentials on the alternating tree and reduce every stored
            // frontier slack by the same amount.  At least `next_right` becomes
            // tight, so each pass makes a discrete primal-dual event possible.
            for right in 0..=size {
                if used_right[right] {
                    let row = right_mate[right];
                    row_potential[row] = row_potential[row]
                        .checked_add(delta)
                        .ok_or_else(|| "Hungarian row potential overflowed".to_string())?;
                    column_potential[right] = column_potential[right]
                        .checked_sub(delta)
                        .ok_or_else(|| "Hungarian column potential overflowed".to_string())?;
                } else if let Some(slack) = min_slack[right] {
                    min_slack[right] = Some(
                        slack
                            .checked_sub(delta)
                            .ok_or_else(|| "Hungarian slack update overflowed".to_string())?,
                    );
                }
            }
            dual_updates += 1;
            current_right = next_right;

            if right_mate[current_right] == 0 {
                break;
            }
        }

        // Follow the predecessor columns back to sentinel zero.  Reassigning
        // each column flips the alternating path and increases cardinality by 1.
        loop {
            let predecessor = previous_right[current_right];
            right_mate[current_right] = right_mate[predecessor];
            current_right = predecessor;
            if current_right == 0 {
                break;
            }
        }
    }

    let mut assignment = vec![usize::MAX; size];
    for (right, &row) in right_mate.iter().enumerate().skip(1) {
        if row == 0 {
            return Err("bipartite graph has no perfect matching".to_string());
        }
        assignment[row - 1] = right - 1;
    }
    if assignment.contains(&usize::MAX) {
        return Err("internal error: Hungarian assignment is incomplete".to_string());
    }
    Ok((assignment, iterations, dual_updates))
}
