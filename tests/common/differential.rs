//! Failure reduction and recording for deterministic differential tests.

use blossom_study::graph::{Edge, Graph};
use serde_json::json;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    pub seed: u64,
    pub n: usize,
    pub edges: Vec<Edge>,
}

impl Case {
    fn graph(&self) -> Graph {
        Graph::from_edges(self.n, self.edges.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Failure {
    pub code: String,
    pub detail: String,
}

impl Failure {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

/// Remove edges until no single remaining edge can be removed while keeping
/// the same failure code. Vertex count and edge weights stay fixed, so the
/// artifact is one-edge-minimal for this predicate, not globally minimal.
fn minimize(
    case: &Case,
    failure_code: &str,
    check: &impl Fn(&Graph) -> Result<(), Failure>,
) -> Case {
    let mut reduced = case.clone();
    let mut index = 0;
    while index < reduced.edges.len() {
        let mut candidate = reduced.clone();
        candidate.edges.remove(index);
        let same_failure = evaluate(&candidate.graph(), check)
            .err()
            .is_some_and(|failure| failure.code == failure_code);
        if same_failure {
            reduced = candidate;
            index = 0;
        } else {
            index += 1;
        }
    }
    reduced
}

fn evaluate(graph: &Graph, check: &impl Fn(&Graph) -> Result<(), Failure>) -> Result<(), Failure> {
    catch_unwind(AssertUnwindSafe(|| check(graph))).unwrap_or_else(|payload| {
        let detail = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_string())
            })
            .unwrap_or_else(|| "non-string panic".to_string());
        Err(Failure::new("panic", detail))
    })
}

pub fn assert_consistent(suite: &str, case: &Case, check: impl Fn(&Graph) -> Result<(), Failure>) {
    let Err(original) = evaluate(&case.graph(), &check) else {
        return;
    };
    let reduced = minimize(case, &original.code, &check);
    let minimized = evaluate(&reduced.graph(), &check).expect_err("reducer must preserve failure");
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/differential-failures");
    std::fs::create_dir_all(&directory).expect("create differential failure directory");
    let path = directory.join(format!("{suite}-{}.json", case.name));
    let artifact = json!({
        "suite": suite,
        "case": case.name,
        "seed": case.seed,
        "failure_code": original.code,
        "original_detail": original.detail,
        "minimized_detail": minimized.detail,
        "original_edge_count": case.edges.len(),
        "minimized_edge_count": reduced.edges.len(),
        "reduction": "one-edge-minimal for the same failure code; vertex count fixed",
        "n": reduced.n,
        "edges": reduced.edges,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&artifact).unwrap())
        .expect("record minimized differential failure");
    panic!(
        "{suite}/{} failed ({}); minimized graph recorded at {}",
        case.name,
        original.code,
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_keeps_only_edges_needed_for_the_same_failure() {
        let case = Case {
            name: "reducer-self-test".into(),
            seed: 0,
            n: 6,
            edges: vec![
                Edge::new(0, 1, None),
                Edge::new(0, 2, None),
                Edge::new(2, 3, None),
                Edge::new(4, 5, None),
            ],
        };
        let check = |graph: &Graph| {
            if graph.edge_index(0, 1).is_some() && graph.edge_index(2, 3).is_some() {
                Err(Failure::new("both-edges", "synthetic failure"))
            } else {
                Ok(())
            }
        };
        let reduced = minimize(&case, "both-edges", &check);
        assert_eq!(reduced.edges.len(), 2);
        assert!(reduced.graph().edge_index(0, 1).is_some());
        assert!(reduced.graph().edge_index(2, 3).is_some());
    }

    #[test]
    fn recorder_writes_a_replayable_minimized_graph() {
        let case = Case {
            name: "recorder-self-test".into(),
            seed: 7,
            n: 4,
            edges: vec![Edge::new(0, 1, Some(2.0)), Edge::new(2, 3, Some(3.0))],
        };
        let check = |graph: &Graph| {
            if graph.edge_index(0, 1).is_some() {
                Err(Failure::new("synthetic", "edge 0--1 remains"))
            } else {
                Ok(())
            }
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/differential-failures/self-test-recorder-self-test.json");
        let failed = catch_unwind(AssertUnwindSafe(|| {
            assert_consistent("self-test", &case, check)
        }));
        assert!(failed.is_err());
        let artifact: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(artifact["failure_code"], "synthetic");
        assert_eq!(artifact["minimized_edge_count"], 1);
        let graph = Graph::from_json_file(path.to_str().unwrap()).unwrap();
        assert_eq!(graph.n, 4);
        assert_eq!(graph.edges[0], Edge::new(0, 1, Some(2.0)));
        std::fs::remove_file(path).unwrap();
    }
}
