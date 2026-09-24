//! Graph representations for matching algorithms

use serde::{Deserialize, Serialize};
use std::io;

/// Edge with optional weight
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub weight: Option<f64>,
}

impl Edge {
    pub fn new(u: usize, v: usize, weight: Option<f64>) -> Self {
        Self { u, v, weight }
    }
}

/// Graph representation supporting both dense and sparse formats
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph {
    pub n: usize,
    pub edges: Vec<Edge>,
    // Dense adjacency matrix for O(1) edge lookups
    pub adj_mat: Option<Vec<Vec<bool>>>,
    // Adjacency lists for sparse iteration
    pub adj_list: Vec<Vec<usize>>,
    // Edge index lookup: (u, v) -> edge index
    pub edge_index: Option<Vec<Vec<usize>>>,
}

impl Graph {
    /// Create from edge list (sparse)
    pub fn from_edges(n: usize, edges: Vec<Edge>) -> Self {
        let mut adj_list = vec![Vec::new(); n];
        let mut adj_mat = vec![vec![false; n]; n];
        let mut edge_index = vec![vec![usize::MAX; n]; n];

        for (idx, e) in edges.iter().enumerate() {
            adj_list[e.u].push(idx);
            adj_list[e.v].push(idx);
            adj_mat[e.u][e.v] = true;
            adj_mat[e.v][e.u] = true;
            edge_index[e.u][e.v] = idx;
            edge_index[e.v][e.u] = idx;
        }

        Self {
            n,
            edges,
            adj_mat: Some(adj_mat),
            adj_list,
            edge_index: Some(edge_index),
        }
    }

    /// Create from edge list without dense matrix (for large graphs)
    pub fn from_edges_sparse(n: usize, edges: Vec<Edge>) -> Self {
        let mut adj_list = vec![Vec::new(); n];
        for (idx, e) in edges.iter().enumerate() {
            adj_list[e.u].push(idx);
            adj_list[e.v].push(idx);
        }
        Self {
            n,
            edges,
            adj_mat: None,
            adj_list,
            edge_index: None,
        }
    }

    pub fn edge(&self, idx: usize) -> &Edge {
        &self.edges[idx]
    }

    pub fn edge_index(&self, u: usize, v: usize) -> Option<usize> {
        self.edge_index
            .as_ref()
            .map(|ei| ei[u][v])
            .filter(|&i| i != usize::MAX)
    }

    pub fn is_adjacent(&self, u: usize, v: usize) -> bool {
        if let Some(am) = &self.adj_mat {
            am[u][v]
        } else {
            self.edges
                .iter()
                .any(|e| (e.u == u && e.v == v) || (e.u == v && e.v == u))
        }
    }

    pub fn adj_edges(&self, u: usize) -> &[usize] {
        &self.adj_list[u]
    }

    /// Load graph from JSON file
    pub fn from_json_file(path: &str) -> Result<Self, io::Error> {
        let content = std::fs::read_to_string(path)?;
        let graph: Self = serde_json::from_str(&content)?;
        Ok(graph)
    }

    /// Save graph to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), io::Error> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    /// Load graph from DIMACS format file
    pub fn from_dimacs_file(path: &str) -> Result<Self, io::Error> {
        let content = std::fs::read_to_string(path)?;
        Self::from_dimacs_str(&content)
    }

    /// Parse graph from DIMACS format string
    /// Supports both unweighted (e u v) and weighted (e u v w) edges
    pub fn from_dimacs_str(s: &str) -> Result<Self, io::Error> {
        let mut n = 0;
        let mut edges = Vec::new();

        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('c') {
                continue; // Skip comments and empty lines
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "p" => {
                    if parts.len() >= 4 && parts[1] == "edge" {
                        n = parts[2].parse().map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Invalid vertex count in problem line",
                            )
                        })?;
                        let _edge_count: usize = parts[3].parse().map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Invalid edge count in problem line",
                            )
                        })?;
                    }
                }
                "e" if parts.len() >= 3 => {
                    let u: usize = parts[1].parse().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "Invalid vertex u in edge line")
                    })?;
                    let v: usize = parts[2].parse().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "Invalid vertex v in edge line")
                    })?;
                    // DIMACS uses 1-based indexing, convert to 0-based
                    let u = u.saturating_sub(1);
                    let v = v.saturating_sub(1);
                    let weight = if parts.len() >= 4 {
                        Some(parts[3].parse().map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Invalid weight in edge line",
                            )
                        })?)
                    } else {
                        None
                    };
                    edges.push(Edge::new(u, v, weight));
                }
                _ => {}
            }
        }

        if n == 0 {
            n = edges
                .iter()
                .map(|e| e.u.max(e.v))
                .max()
                .map(|x| x + 1)
                .unwrap_or(0);
        }

        Ok(Graph::from_edges(n, edges))
    }

    /// Save graph to DIMACS format file
    pub fn to_dimacs_file(&self, path: &str) -> Result<(), io::Error> {
        let content = self.to_dimacs_str();
        std::fs::write(path, content)
    }

    /// Export graph to DIMACS format string
    pub fn to_dimacs_str(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("p edge {} {}\n", self.n, self.edges.len()));
        for e in &self.edges {
            if let Some(w) = e.weight {
                out.push_str(&format!("e {} {} {}\n", e.u + 1, e.v + 1, w));
            } else {
                out.push_str(&format!("e {} {}\n", e.u + 1, e.v + 1));
            }
        }
        out
    }
}

/// Generator for random test graphs
pub mod generator {
    use super::*;
    use rand::{Rng, thread_rng};

    pub fn complete_graph(n: usize, weight_range: (f64, f64)) -> Graph {
        let mut edges = Vec::with_capacity(n * (n - 1) / 2);
        let mut rng = thread_rng();
        for u in 0..n {
            for v in (u + 1)..n {
                let w = rng.gen_range(weight_range.0..weight_range.1);
                edges.push(Edge::new(u, v, Some(w)));
            }
        }
        Graph::from_edges(n, edges)
    }

    pub fn random_sparse(n: usize, m: usize, weight_range: (f64, f64)) -> Graph {
        let mut edges = Vec::with_capacity(m);
        let mut edge_set = std::collections::HashSet::new();
        let mut rng = thread_rng();
        while edges.len() < m {
            let u = rng.gen_range(0..n);
            let v = rng.gen_range(0..n);
            if u == v {
                continue;
            }
            let key = (u.min(v), u.max(v));
            if edge_set.insert(key) {
                let w = rng.gen_range(weight_range.0..weight_range.1);
                edges.push(Edge::new(u, v, Some(w)));
            }
        }
        Graph::from_edges(n, edges)
    }

    pub fn grid_graph(rows: usize, cols: usize, weight_range: (f64, f64)) -> Graph {
        let n = rows * cols;
        let mut edges = Vec::new();
        let mut rng = thread_rng();
        for r in 0..rows {
            for c in 0..cols {
                let u = r * cols + c;
                if r + 1 < rows {
                    let v = (r + 1) * cols + c;
                    let w = rng.gen_range(weight_range.0..weight_range.1);
                    edges.push(Edge::new(u, v, Some(w)));
                }
                if c + 1 < cols {
                    let v = r * cols + (c + 1);
                    let w = rng.gen_range(weight_range.0..weight_range.1);
                    edges.push(Edge::new(u, v, Some(w)));
                }
            }
        }
        Graph::from_edges(n, edges)
    }
}
