//! Graph representations for matching algorithms

use serde::{Deserialize, Deserializer, Serialize};
use std::io;

fn dimacs_error(line_number: usize, message: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("line {line_number}: {message}"),
    )
}

/// Edge with optional weight
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub weight: Option<f64>,
}

impl Edge {
    pub fn new(u: usize, v: usize, weight: Option<f64>) -> Self {
        Self { u, v, weight }
    }

    #[inline]
    pub fn weight_or_one(&self) -> f64 {
        self.weight.unwrap_or(1.0)
    }
}

/// Graph representation supporting both dense and sparse formats
#[derive(Clone, Debug, Serialize)]
pub struct Graph {
    pub n: usize,
    pub edges: Vec<Edge>,
    // Dense adjacency matrix for O(1) edge lookups
    #[serde(skip)]
    pub adj_mat: Option<Vec<Vec<bool>>>,
    // Adjacency lists for sparse iteration
    #[serde(skip)]
    pub adj_list: Vec<Vec<usize>>,
    // Edge index lookup: (u, v) -> edge index
    #[serde(skip)]
    pub edge_index: Option<Vec<Vec<usize>>>,
}

impl<'de> Deserialize<'de> for Graph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoredGraph {
            n: usize,
            edges: Vec<Edge>,
        }

        let stored = StoredGraph::deserialize(deserializer)?;
        Graph::try_from_edges(stored.n, stored.edges).map_err(serde::de::Error::custom)
    }
}

impl Graph {
    /// Create from an edge list using sparse derived structures.
    ///
    /// Panics if an edge endpoint is outside `0..n`; use [`Self::try_from_edges`]
    /// when graph dimensions or edges come from untrusted input.
    pub fn from_edges(n: usize, edges: Vec<Edge>) -> Self {
        Self::try_from_edges(n, edges).expect("graph edges must reference existing vertices")
    }

    /// Create from edge list without dense matrix (for large graphs).
    ///
    /// Panics if an edge endpoint is outside `0..n`; use
    /// [`Self::try_from_edges_sparse`] for fallible construction.
    pub fn from_edges_sparse(n: usize, edges: Vec<Edge>) -> Self {
        Self::try_from_edges_sparse(n, edges).expect("graph edges must reference existing vertices")
    }

    /// Create a graph after verifying that every edge endpoint is in `0..n`.
    pub fn try_from_edges(n: usize, edges: Vec<Edge>) -> Result<Self, String> {
        Self::try_from_edges_sparse(n, edges)
    }

    /// Create a sparse graph after verifying that every edge endpoint is in `0..n`.
    pub fn try_from_edges_sparse(n: usize, edges: Vec<Edge>) -> Result<Self, String> {
        for (idx, e) in edges.iter().enumerate() {
            if e.u >= n || e.v >= n {
                return Err(format!(
                    "edge {idx} has endpoint outside 0..{n} (u={}, v={})",
                    e.u, e.v
                ));
            }
        }

        let mut adj_list = Vec::new();
        adj_list.try_reserve_exact(n).map_err(|error| {
            format!("cannot allocate adjacency lists for {n} vertices: {error}")
        })?;
        adj_list.resize_with(n, Vec::new);
        for (idx, e) in edges.iter().enumerate() {
            adj_list[e.u].push(idx);
            adj_list[e.v].push(idx);
        }
        Ok(Self {
            n,
            edges,
            adj_mat: None,
            adj_list,
            edge_index: None,
        })
    }

    /// Validate endpoints in the public edge list.
    ///
    /// Graph fields remain public for compatibility, so callers that mutate
    /// them directly can invalidate a previously constructed graph.
    pub fn validate(&self) -> Result<(), String> {
        for (idx, edge) in self.edges.iter().enumerate() {
            if edge.u >= self.n || edge.v >= self.n {
                return Err(format!(
                    "edge {idx} has endpoint outside 0..{} (u={}, v={})",
                    self.n, edge.u, edge.v
                ));
            }
        }
        Ok(())
    }

    pub fn edge(&self, idx: usize) -> &Edge {
        &self.edges[idx]
    }

    pub fn edge_index(&self, u: usize, v: usize) -> Option<usize> {
        if u >= self.n || v >= self.n {
            return None;
        }
        if let Some(index) = &self.edge_index {
            return (index[u][v] != usize::MAX).then_some(index[u][v]);
        }
        self.adj_list[u].iter().copied().find(|&index| {
            let edge = &self.edges[index];
            (edge.u == u && edge.v == v) || (edge.u == v && edge.v == u)
        })
    }

    pub fn is_adjacent(&self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n {
            return false;
        }
        if let Some(am) = &self.adj_mat {
            am[u][v]
        } else {
            self.edge_index(u, v).is_some()
        }
    }

    pub fn adj_edges(&self, u: usize) -> &[usize] {
        self.adj_list.get(u).map_or(&[], Vec::as_slice)
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

    /// Parse a DIMACS edge-format string with unweighted (`e u v`) or weighted
    /// (`e u v w`) edge records. If present, the single problem record must be
    /// `p edge n m`; its declared edge count and vertex range are checked.
    /// Headerless edge lists remain supported and infer their vertex count.
    /// Malformed records return `InvalidData` errors with their line numbers.
    pub fn from_dimacs_str(s: &str) -> Result<Self, io::Error> {
        let mut problem = None;
        let mut edges = Vec::new();
        let mut edge_lines = Vec::new();

        for (line_index, raw_line) in s.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts[0] == "c" {
                continue;
            }

            match parts[0] {
                "p" => {
                    if parts.len() != 4 {
                        return Err(dimacs_error(
                            line_number,
                            "problem record must have form `p edge <vertices> <edges>`",
                        ));
                    }
                    if parts[1] != "edge" {
                        return Err(dimacs_error(
                            line_number,
                            format!("unsupported problem type `{}`", parts[1]),
                        ));
                    }
                    if problem.is_some() {
                        return Err(dimacs_error(line_number, "duplicate problem record"));
                    }
                    let vertex_count = parts[2].parse::<usize>().map_err(|_| {
                        dimacs_error(line_number, "invalid vertex count in problem record")
                    })?;
                    let edge_count = parts[3].parse::<usize>().map_err(|_| {
                        dimacs_error(line_number, "invalid edge count in problem record")
                    })?;
                    problem = Some((vertex_count, edge_count, line_number));
                }
                "e" => {
                    if parts.len() != 3 && parts.len() != 4 {
                        return Err(dimacs_error(
                            line_number,
                            "edge record must have form `e <u> <v> [weight]`",
                        ));
                    }
                    let u = parts[1]
                        .parse::<usize>()
                        .map_err(|_| dimacs_error(line_number, "invalid vertex u"))?;
                    let v = parts[2]
                        .parse::<usize>()
                        .map_err(|_| dimacs_error(line_number, "invalid vertex v"))?;
                    if u == 0 || v == 0 {
                        return Err(dimacs_error(
                            line_number,
                            "DIMACS edge vertices are 1-based and must be positive",
                        ));
                    }
                    // DIMACS uses 1-based indexing, convert to 0-based
                    let u = u - 1;
                    let v = v - 1;
                    let weight = if parts.len() == 4 {
                        let weight = parts[3]
                            .parse::<f64>()
                            .map_err(|_| dimacs_error(line_number, "invalid edge weight"))?;
                        if !weight.is_finite() {
                            return Err(dimacs_error(line_number, "edge weight must be finite"));
                        }
                        Some(weight)
                    } else {
                        None
                    };
                    edges.push(Edge::new(u, v, weight));
                    edge_lines.push(line_number);
                }
                record => {
                    return Err(dimacs_error(
                        line_number,
                        format!("unsupported record type `{record}`"),
                    ));
                }
            }
        }

        let n = if let Some((vertex_count, declared_edges, problem_line)) = problem {
            if declared_edges != edges.len() {
                return Err(dimacs_error(
                    problem_line,
                    format!(
                        "problem record declares {declared_edges} edges, but found {}",
                        edges.len()
                    ),
                ));
            }
            for (edge_index, edge) in edges.iter().enumerate() {
                if edge.u >= vertex_count || edge.v >= vertex_count {
                    return Err(dimacs_error(
                        edge_lines[edge_index],
                        format!(
                            "edge endpoint ({}, {}) is outside 1..={vertex_count}",
                            edge.u + 1,
                            edge.v + 1
                        ),
                    ));
                }
            }
            vertex_count
        } else {
            edges
                .iter()
                .map(|edge| edge.u.max(edge.v))
                .max()
                .map(|max_vertex| {
                    max_vertex.checked_add(1).ok_or_else(|| {
                        dimacs_error(edge_lines[0], "vertex index exceeds the supported range")
                    })
                })
                .transpose()?
                .unwrap_or(0)
        };

        Graph::try_from_edges(n, edges).map_err(|message| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid DIMACS graph: {message}"),
            )
        })
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
