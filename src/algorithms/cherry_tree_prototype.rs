//! Corrected dense-matrix front end for the Cherry-Tree study variant.
//!
//! The original Cherry-Tree experiment stored alternating-tree predecessors
//! only on odd vertices, but attempted to find lowest common ancestors by
//! following predecessors directly from even vertices.  A same-tree collision
//! could therefore be mistaken for an augmenting path between two trees.  The
//! resulting path contained its root twice and corrupted the mate relation.
//! Its contracted state also lacked a blossom base, ordered children and the
//! real boundary endpoints needed to route an augmenting path through nested
//! blossoms.
//!
//! Those are representation errors, not isolated off-by-one bugs.  This module
//! consequently no longer maintains the incomplete DSU/bridge state machine.
//! It preserves Cherry-Tree's useful *complete distance matrix* interface, but
//! delegates the combinatorial search to the repository's audited hierarchical
//! Galil/Edmonds kernel.  That kernel explicitly records:
//!
//! - an alternating-forest parent endpoint for every inner object;
//! - a base and ordered immediate children for every contracted blossom;
//! - the original edge endpoints between consecutive blossom children;
//! - top-level containment without losing nested contraction history; and
//! - exact integral duals, including blossom dual events and expansion.
//!
//! Sharing the kernel is intentional.  It makes the Cherry benchmark measure
//! the cost of its dense-matrix representation and conversion without claiming
//! that a second copy of the same primal-dual algorithm is independent.

use super::MatchingMetadata;
use super::blossom_engine::BlossomEngine;
use crate::graph::{Edge, Graph};

/// MWPM solver accepting the dense symmetric matrix used by Cherry-Tree.
///
/// `match_pair` remains public for compatibility with the original educational
/// prototype.  Unlike that prototype, a successful `solve` call guarantees
/// that every entry is in range and reciprocal.
#[derive(Clone, Debug)]
pub struct CherryTreeSolver {
    n: usize,
    distance_matrix: Vec<Vec<f64>>,
    pub match_pair: Vec<usize>,
    metadata: MatchingMetadata,
}

impl CherryTreeSolver {
    pub fn new(n: usize, distance_matrix: Vec<Vec<f64>>) -> Self {
        Self {
            n,
            distance_matrix,
            match_pair: vec![usize::MAX; n],
            metadata: MatchingMetadata::default(),
        }
    }

    /// Solve the complete-graph MWPM instance represented by the matrix.
    ///
    /// Matrix validation happens here rather than in `new` so construction
    /// remains inexpensive and compatible with the former API.  Only the upper
    /// triangle becomes graph edges; requiring exact symmetry prevents two
    /// contradictory costs from being silently resolved by iteration order.
    pub fn solve(&mut self) -> Result<&[usize], String> {
        self.validate_matrix()?;

        let mut edges = Vec::with_capacity(self.n.saturating_mul(self.n.saturating_sub(1)) / 2);
        for u in 0..self.n {
            for v in (u + 1)..self.n {
                edges.push(Edge::new(u, v, Some(self.distance_matrix[u][v])));
            }
        }

        // The shared kernel owns all alternating-forest, contraction, dual and
        // expansion state.  Its result uses indices into this deterministic
        // upper-triangle edge list, which lets us reconstruct the mate table
        // without another pair-to-edge lookup structure.
        let graph = Graph::from_edges_sparse(self.n, edges);
        let result = BlossomEngine::new().solve(&graph)?;

        self.match_pair.fill(usize::MAX);
        for edge_index in result.edges.iter().copied() {
            let edge = graph.edges.get(edge_index).ok_or_else(|| {
                format!("Cherry-Tree kernel returned invalid edge index {edge_index}")
            })?;
            if self.match_pair[edge.u] != usize::MAX || self.match_pair[edge.v] != usize::MAX {
                return Err(format!(
                    "Cherry-Tree kernel returned overlapping edge ({}, {})",
                    edge.u, edge.v
                ));
            }
            self.match_pair[edge.u] = edge.v;
            self.match_pair[edge.v] = edge.u;
        }

        // Keep the invariant check beside the state mutation.  This is the
        // check the old core omitted when it stopped merely because no sentinel
        // remained in the table.
        for (u, &v) in self.match_pair.iter().enumerate() {
            if v >= self.n || v == u || self.match_pair[v] != u {
                return Err(format!(
                    "Cherry-Tree kernel produced a non-reciprocal mate relation at vertex {u}"
                ));
            }
        }

        self.metadata = result.metadata;
        Ok(&self.match_pair)
    }

    pub fn metadata(&self) -> &MatchingMetadata {
        &self.metadata
    }

    fn validate_matrix(&self) -> Result<(), String> {
        if !self.n.is_multiple_of(2) {
            return Err(format!(
                "Cherry-Tree requires an even vertex count (n = {})",
                self.n
            ));
        }
        if self.distance_matrix.len() != self.n {
            return Err(format!(
                "Cherry-Tree expected {} matrix rows, found {}",
                self.n,
                self.distance_matrix.len()
            ));
        }
        for (row_index, row) in self.distance_matrix.iter().enumerate() {
            if row.len() != self.n {
                return Err(format!(
                    "Cherry-Tree matrix row {row_index} has length {}, expected {}",
                    row.len(),
                    self.n
                ));
            }
        }

        for u in 0..self.n {
            for v in (u + 1)..self.n {
                let forward = self.distance_matrix[u][v];
                let reverse = self.distance_matrix[v][u];
                if !forward.is_finite() || !reverse.is_finite() {
                    return Err(format!(
                        "Cherry-Tree edge ({u}, {v}) has a non-finite matrix weight"
                    ));
                }
                if forward != reverse {
                    return Err(format!(
                        "Cherry-Tree requires a symmetric matrix: [{u}][{v}]={forward}, [{v}][{u}]={reverse}"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CherryTreeSolver;

    #[test]
    fn all_equal_k4_regression_is_reciprocal() {
        let n = 4;
        let mut matrix = vec![vec![1.0; n]; n];
        for (vertex, row) in matrix.iter_mut().enumerate() {
            row[vertex] = 0.0;
        }

        let mut solver = CherryTreeSolver::new(n, matrix);
        let mates = solver.solve().unwrap();
        for (vertex, &mate) in mates.iter().enumerate() {
            assert_eq!(mates[mate], vertex);
        }
    }

    #[test]
    fn rejects_an_asymmetric_matrix() {
        let mut solver = CherryTreeSolver::new(2, vec![vec![0.0, 1.0], vec![2.0, 0.0]]);
        assert!(solver.solve().unwrap_err().contains("symmetric matrix"));
    }
}
