//! Hopcroft–Karp maximum-cardinality matching for bipartite graphs.
//!
//! The caller supplies the size of the left partition. Vertices `0..left_size`
//! are left vertices and `left_size..graph.n` are right vertices; the two sides
//! need not have equal sizes. Every edge must cross the declared partition.
//! Edge weights, if present, do not influence which matching is chosen.
//!
//! An augmentation increases the matching size by one. Rather than finding
//! just one path per search, Hopcroft–Karp repeatedly performs two steps:
//!
//! 1. Multi-source BFS starts at every free left vertex. It assigns layers to
//!    left vertices reached through alternating (unmatched, matched) edge
//!    pairs, and records the shortest distance to any free right vertex.
//! 2. DFS follows only edges consistent with those layers. It finds a maximal
//!    collection of vertex-disjoint shortest augmenting paths in that phase.
//!
//! The shortest augmenting path length then grows between successful phases,
//! giving `O(E * sqrt(V))` time and `O(V + E)` space. The DFS below is iterative:
//! a long alternating path cannot overflow the Rust call stack.

use std::collections::VecDeque;
use std::time::Instant;

use super::{MatchingMetadata, MatchingResult};
use crate::graph::Graph;

const UNREACHED: usize = usize::MAX;

/// A cardinality solver for an explicitly partitioned bipartite graph.
#[derive(Clone, Copy, Debug)]
pub struct HopcroftKarpSolver {
    left_size: usize,
}

impl HopcroftKarpSolver {
    pub fn new(left_size: usize) -> Self {
        Self { left_size }
    }

    pub fn solve(&self, graph: &Graph) -> Result<MatchingResult, String> {
        hopcroft_karp_maximum_cardinality_matching(graph, self.left_size)
    }
}

/// Return a maximum-cardinality matching, even if no perfect matching exists.
///
/// The result contains indices into `graph.edges`. `total_weight` follows the
/// crate-wide result convention and sums the selected edges' stored weights
/// (or one per unweighted edge); it is *not* an optimization objective here.
pub fn hopcroft_karp_maximum_cardinality_matching(
    graph: &Graph,
    left_size: usize,
) -> Result<MatchingResult, String> {
    let started = Instant::now();
    if left_size > graph.n {
        return Err(format!(
            "left partition size {left_size} exceeds graph.n={}",
            graph.n
        ));
    }
    let right_size = graph.n - left_size;

    // Each original edge is represented exactly once, from its left endpoint.
    // Keeping the edge index lets us reconstruct the caller's actual edges,
    // including parallel edges, without a separate endpoint lookup.
    let mut adjacency = vec![Vec::<(usize, usize)>::new(); left_size];
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        if edge.u >= graph.n || edge.v >= graph.n {
            return Err(format!(
                "edge {edge_index} has an endpoint outside 0..{}",
                graph.n
            ));
        }
        let u_left = edge.u < left_size;
        let v_left = edge.v < left_size;
        if u_left == v_left {
            return Err(format!(
                "edge {edge_index} ({}, {}) does not cross bipartition 0..{left_size} | {left_size}..{}",
                edge.u, edge.v, graph.n
            ));
        }
        let (left, right) = if u_left {
            (edge.u, edge.v - left_size)
        } else {
            (edge.v, edge.u - left_size)
        };
        adjacency[left].push((right, edge_index));
    }

    // `left_mate` retains the selected original edge as well as its right
    // endpoint. `right_mate` supports constant-time traversal through the
    // matched edge after BFS/DFS scans an unmatched edge.
    let mut left_mate = vec![None::<(usize, usize)>; left_size];
    let mut right_mate = vec![None::<usize>; right_size];
    let mut distance = vec![UNREACHED; left_size];
    let mut iterations = 0;
    let mut augmentations = 0;

    while let Some(shortest_distance) = build_layers(
        &adjacency,
        &left_mate,
        &right_mate,
        &mut distance,
        &mut iterations,
    ) {
        // A cursor ensures each left adjacency list is scanned at most once
        // by DFS in this phase, even if several free roots lead into it.
        let mut next_edge = vec![0; left_size];
        for root in 0..left_size {
            if left_mate[root].is_none()
                && augment_from(
                    root,
                    shortest_distance,
                    &adjacency,
                    &mut distance,
                    &mut next_edge,
                    &mut left_mate,
                    &mut right_mate,
                    &mut iterations,
                )
            {
                augmentations += 1;
            }
        }
    }

    let mut edges: Vec<usize> = left_mate
        .iter()
        .flatten()
        .map(|&(_, index)| index)
        .collect();
    edges.sort_unstable();
    let total_weight = edges
        .iter()
        .map(|&index| graph.edges[index].weight_or_one())
        .sum();
    let is_perfect = edges.len().saturating_mul(2) == graph.n;

    Ok(MatchingResult {
        edges,
        total_weight,
        is_perfect,
        metadata: MatchingMetadata {
            iterations,
            augmentations,
            blossoms_created: 0,
            blossoms_expanded: 0,
            dual_updates: 0,
            runtime_ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    })
}

/// Build the alternating BFS layers. A transition from left `u` through a
/// matched right vertex reaches its mate on the next left layer. A free right
/// vertex terminates an augmenting path, so we remember only the first (and
/// therefore shortest) terminal layer. Later layers cannot help this phase.
fn build_layers(
    adjacency: &[Vec<(usize, usize)>],
    left_mate: &[Option<(usize, usize)>],
    right_mate: &[Option<usize>],
    distance: &mut [usize],
    iterations: &mut usize,
) -> Option<usize> {
    distance.fill(UNREACHED);
    let mut queue = VecDeque::new();
    for left in 0..adjacency.len() {
        if left_mate[left].is_none() {
            distance[left] = 0;
            queue.push_back(left);
        }
    }

    let mut shortest = None::<usize>;
    while let Some(left) = queue.pop_front() {
        let next_layer = distance[left] + 1;
        if shortest.is_some_and(|found| next_layer > found) {
            break;
        }
        for &(right, _) in &adjacency[left] {
            *iterations += 1;
            match right_mate[right] {
                None => {
                    shortest = Some(next_layer);
                }
                Some(next_left)
                    if distance[next_left] == UNREACHED
                        && shortest.is_none_or(|found| next_layer < found) =>
                {
                    distance[next_left] = next_layer;
                    queue.push_back(next_left);
                }
                Some(_) => {}
            }
        }
    }
    shortest
}

/// One DFS frame records the left vertex and the edge by which its parent
/// reached the right vertex currently matched to it. On success these saved
/// edges are precisely the edges that must become matched while unwinding.
#[derive(Clone, Copy)]
struct Frame {
    left: usize,
    incoming: Option<(usize, usize)>, // (right vertex, original edge index)
}

fn augment_from(
    root: usize,
    shortest_distance: usize,
    adjacency: &[Vec<(usize, usize)>],
    distance: &mut [usize],
    next_edge: &mut [usize],
    left_mate: &mut [Option<(usize, usize)>],
    right_mate: &mut [Option<usize>],
    iterations: &mut usize,
) -> bool {
    let mut stack = vec![Frame {
        left: root,
        incoming: None,
    }];

    while let Some(frame) = stack.last().copied() {
        let left = frame.left;
        if next_edge[left] == adjacency[left].len() {
            // No shortest augmenting path continues from this left vertex.
            // The dead label prevents another root from exploring it again.
            distance[left] = UNREACHED;
            stack.pop();
            continue;
        }

        let (right, edge_index) = adjacency[left][next_edge[left]];
        next_edge[left] += 1;
        *iterations += 1;
        let next_layer = distance[left] + 1;
        match right_mate[right] {
            None if next_layer == shortest_distance => {
                // Found the shortest free-to-free path. Assign the final edge,
                // then flip the saved alternating edges back toward the root.
                left_mate[left] = Some((right, edge_index));
                right_mate[right] = Some(left);
                for index in (1..stack.len()).rev() {
                    let parent = stack[index - 1].left;
                    let (right, edge_index) = stack[index]
                        .incoming
                        .expect("every non-root frame has an incoming edge");
                    left_mate[parent] = Some((right, edge_index));
                    right_mate[right] = Some(parent);
                }
                return true;
            }
            Some(next_left) if distance[next_left] == next_layer => {
                stack.push(Frame {
                    left: next_left,
                    incoming: Some((right, edge_index)),
                });
            }
            _ => {}
        }
    }
    false
}
