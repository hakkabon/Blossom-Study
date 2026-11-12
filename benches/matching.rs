use blossom_study::{
    MatchingResult,
    algorithms::{
        FastBlossomMatching, MicaliVazirani, MicaliVaziraniPhase,
        hopcroft_karp_maximum_cardinality_matching,
    },
    benchmark::{GENERAL_CARDINALITY_SOLVERS, compare_general_cardinality},
    graph::{Edge, Graph, generator},
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::fmt::Display;

#[inline]
fn require_timed_result<T, E: Display>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| {
        panic!("solver failed during timed run after successful preflight; no valid timing sample: {error}")
    })
}

fn sparse_general_cardinality_graph(n: usize) -> Graph {
    let mut edges = Vec::with_capacity(4 * n);
    for u in (0..n).step_by(2) {
        edges.push(Edge::new(u, u + 1, None));
    }
    for u in 0..n {
        for offset in [2, 3, 5] {
            let v = (u + offset) % n;
            if u < v {
                edges.push(Edge::new(u, v, None));
            }
        }
    }
    Graph::from_edges(n, edges)
}

fn cardinality_benchmarks(c: &mut Criterion) {
    for (shape, sizes) in [("dense", [20, 50, 100]), ("sparse", [20, 50, 100])] {
        let mut group = c.benchmark_group(format!("maximum-cardinality/general-{shape}"));
        for n in sizes {
            let graph = if shape == "dense" {
                generator::complete_graph(n, (1.0, 100.0))
            } else {
                sparse_general_cardinality_graph(n)
            };
            if let Err(error) = compare_general_cardinality(&graph) {
                eprintln!(
                    "[benchmark correctness failure] general-cardinality/{shape}, n={n}: {error}; timings skipped"
                );
                continue;
            }
            for solver in GENERAL_CARDINALITY_SOLVERS {
                group.bench_with_input(BenchmarkId::new(solver.name, n), &graph, |b, graph| {
                    b.iter(|| (solver.solve)(graph))
                });
            }
        }
        group.finish();
    }

    // MV is not placed beside completed solvers yet.  This group measures the
    // implemented alternating parity-level search only; its name prevents the
    // result from being mistaken for end-to-end maximum matching throughput.
    let mut development = c.benchmark_group("maximum-cardinality/mv-development-phase-1");
    for n in [20, 50, 100, 200] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        if let Err(error) = preflight_mv_level_search(&graph, None) {
            eprintln!(
                "[benchmark correctness failure] MV development level-search, complete-dense, n={n}: {error}; timing skipped"
            );
            continue;
        }
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/complete-dense", n),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let mut solver = MicaliVazirani::new(graph);
                    require_timed_result(solver.run_level_search_phase(graph))
                });
            },
        );
    }

    for side in [5, 10, 15, 20] {
        let n = side * side;
        let graph = generator::grid_graph(side, side, (1.0, 100.0));
        if let Err(error) = preflight_mv_level_search(&graph, None) {
            eprintln!(
                "[benchmark correctness failure] MV development level-search, sparse-grid, n={n}: {error}; timing skipped"
            );
            continue;
        }
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/sparse-grid", n),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let mut solver = MicaliVazirani::new(graph);
                    require_timed_result(solver.run_level_search_phase(graph))
                });
            },
        );
    }

    for n in [20, 50, 100, 200] {
        let mut edges = Vec::new();
        let matched_pairs = (n / 4) * 2;
        let mut mate = vec![None; n];
        for u in (0..matched_pairs).step_by(2) {
            edges.push(Edge::new(u, u + 1, Some(1.0)));
            mate[u] = Some(u + 1);
            mate[u + 1] = Some(u);
        }
        for u in 0..n {
            for offset in [2, 3, 5] {
                let v = (u + offset) % n;
                if u < v {
                    edges.push(Edge::new(u, v, Some(1.0)));
                }
            }
        }
        let graph = Graph::from_edges(n, edges);
        if let Err(error) = preflight_mv_level_search(&graph, Some(mate.clone())) {
            eprintln!(
                "[benchmark correctness failure] MV development level-search, planted-50pct-matched, n={n}: {error}; timing skipped"
            );
            continue;
        }
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/planted-50pct-matched", n),
            &graph,
            |b, graph| {
                let initial_mate = mate.clone();
                b.iter(|| {
                    let mut solver = require_timed_result(MicaliVazirani::with_matching(
                        graph,
                        initial_mate.clone(),
                    ));
                    require_timed_result(solver.run_level_search_phase(graph))
                });
            },
        );
    }
    development.finish();
}

fn preflight_mv_level_search(
    graph: &Graph,
    initial_matching: Option<Vec<Option<usize>>>,
) -> Result<(), String> {
    let mut solver = match initial_matching {
        Some(mate) => MicaliVazirani::with_matching(graph, mate),
        None => Ok(MicaliVazirani::new(graph)),
    }
    .map_err(|error| format!("could not initialize phase benchmark: {error}"))?;
    solver
        .run_level_search_phase(graph)
        .map_err(|error| format!("level-search phase failed: {error}"))?;
    if solver.phase() != MicaliVaziraniPhase::BridgeProcessing {
        return Err(format!(
            "level-search ended in unexpected phase {:?}",
            solver.phase()
        ));
    }
    Ok(())
}

fn check_weighted_results(
    graph: &Graph,
    results: &[(&str, Result<MatchingResult, String>)],
) -> Result<(), String> {
    let mut baseline: Option<(&str, f64)> = None;
    for (name, result) in results {
        let result = result
            .as_ref()
            .map_err(|error| format!("{name} failed to solve: {error}"))?;
        result
            .validate(graph)
            .map_err(|error| format!("{name} returned an invalid matching: {error}"))?;
        if !result.is_perfect {
            return Err(format!("{name} did not return a perfect matching"));
        }
        let Some((baseline_name, baseline_weight)) = baseline else {
            baseline = Some((name, result.total_weight));
            continue;
        };
        let tolerance = 1e-9
            * baseline_weight
                .abs()
                .max(result.total_weight.abs())
                .max(1.0);
        if (result.total_weight - baseline_weight).abs() > tolerance {
            return Err(format!(
                "{name} returned weight {}, but {baseline_name} returned {baseline_weight}",
                result.total_weight
            ));
        }
    }
    if baseline.is_none() {
        return Err("weighted comparison must include at least one solver".to_string());
    }
    Ok(())
}

fn preflight_general_weighted_solvers(graph: &Graph) -> Result<(), String> {
    check_weighted_results(
        graph,
        &[
            (
                "primal-dual",
                blossom_study::algorithms::minimum_weight_perfect_matching(graph),
            ),
            (
                "blossom-engine",
                blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(graph),
            ),
            (
                "sparse-blossom",
                blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph),
            ),
        ],
    )
}

/// The DPS scaling reference is integer-only and substantially more expensive
/// than the other solvers, so it has its own small comparison range.
fn duan_weighted_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/duan-scaling");
    group.sample_size(10);
    for n in [8, 12, 20] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        let baseline = blossom_study::minimum_weight_perfect_matching(&graph);
        let candidate = blossom_study::duan_minimum_weight_perfect_matching(&graph);
        if let Err(error) = check_weighted_results(
            &graph,
            &[("primal-dual", baseline), ("duan-scaling", candidate)],
        ) {
            report_failed_preflight(
                "minimum-weight-perfect/duan-scaling",
                &format!("n={n}"),
                error,
            );
            continue;
        }
        group.bench_with_input(BenchmarkId::new("duan-scaling", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(blossom_study::duan_minimum_weight_perfect_matching(graph))
            })
        });
    }
    group.finish();
}

fn report_failed_preflight(family: &str, parameter: &str, error: String) {
    eprintln!("[benchmark correctness failure] {family}, {parameter}: {error}; timings skipped");
}

fn weighted_benchmarks_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/general-dense");
    for n in [20, 50, 100, 200] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        if let Err(error) = preflight_general_weighted_solvers(&graph) {
            report_failed_preflight(
                "minimum-weight-perfect/general-dense",
                &format!("n={n}"),
                error,
            );
            continue;
        }
        group.bench_with_input(BenchmarkId::new("primal-dual", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(blossom_study::algorithms::minimum_weight_perfect_matching(
                    graph,
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("blossom-engine", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(
                    blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(
                        graph,
                    ),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("sparse-blossom", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(
                    blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph),
                )
            });
        });
    }
    group.finish();
}

fn weighted_benchmarks_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/general-sparse");
    for n in [20, 50, 100, 200] {
        // Plant the edges (0,1), (2,3), ... so every benchmark graph is
        // matchable, then add a small deterministic set of chords.  This keeps
        // m=O(n), which is the regime the sparse representation targets.
        let mut edges = Vec::new();
        for u in (0..n).step_by(2) {
            edges.push(Edge::new(u, u + 1, Some(1.0)));
        }
        for u in 0..n {
            for offset in [2, 3, 5] {
                let v = (u + offset) % n;
                if u < v {
                    edges.push(Edge::new(u, v, Some(((u * 17 + v * 11) % 97) as f64)));
                }
            }
        }
        let graph = Graph::from_edges(n, edges);
        if let Err(error) = preflight_general_weighted_solvers(&graph) {
            report_failed_preflight(
                "minimum-weight-perfect/general-sparse",
                &format!("n={n}"),
                error,
            );
            continue;
        }
        group.bench_with_input(BenchmarkId::new("primal-dual", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(blossom_study::algorithms::minimum_weight_perfect_matching(
                    graph,
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("blossom-engine", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(
                    blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(
                        graph,
                    ),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("sparse-blossom", n), &graph, |b, graph| {
            b.iter(|| {
                require_timed_result(
                    blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph),
                )
            });
        });
    }
    group.finish();
}

/// Build a reproducible dense assignment instance.
///
/// General-graph solvers receive exactly the same `Graph` as Hungarian.  This
/// makes the group useful both as a speed comparison and as a continuous
/// cross-check that blossom machinery agrees with the blossom-free special
/// case.  The total graph size is `2 * partition_size`.
fn complete_bipartite_graph(partition_size: usize) -> Graph {
    let mut edges = Vec::with_capacity(partition_size * partition_size);
    for left in 0..partition_size {
        for right in 0..partition_size {
            let mixed = left
                .wrapping_mul(1_000_003)
                .wrapping_add(right.wrapping_mul(97_409))
                .wrapping_add((left ^ right).wrapping_mul(65_537));
            let weight = (mixed % 16_001) as f64 / 8.0 - 1_000.0;
            edges.push(Edge::new(left, partition_size + right, Some(weight)));
        }
    }
    Graph::from_edges(2 * partition_size, edges)
}

/// Keep the same bipartite topology as the weighted assignment benchmark,
/// but remove costs: Hopcroft–Karp and Fast Blossom solve a cardinality
/// problem, not the minimum-cost assignment problem solved by Hungarian.
fn unweighted_bipartite_graph(weighted: &Graph) -> Graph {
    Graph::from_edges(
        weighted.n,
        weighted
            .edges
            .iter()
            .map(|edge| Edge::new(edge.u, edge.v, None))
            .collect(),
    )
}

/// A sparse, planted-perfect bipartite instance with O(k) edges. The planted
/// diagonal guarantees a full matching while the chords create choices that
/// exercise alternating BFS/DFS beyond an isolated set of edges.
fn sparse_bipartite_graph(partition_size: usize) -> Graph {
    let mut edges = Vec::with_capacity(3 * partition_size);
    for left in 0..partition_size {
        let mut rights = [
            left,
            (left + 1) % partition_size,
            (left * 7 + 3) % partition_size,
        ];
        rights.sort_unstable();
        for (index, &right) in rights.iter().enumerate() {
            if index == 0 || right != rights[index - 1] {
                edges.push(Edge::new(left, partition_size + right, None));
            }
        }
    }
    Graph::from_edges(2 * partition_size, edges)
}

fn cardinality_benchmarks_bipartite(c: &mut Criterion) {
    for (shape, sizes) in [("dense", [10, 25, 50, 100]), ("sparse", [10, 25, 50, 100])] {
        let mut group = c.benchmark_group(format!("maximum-cardinality/bipartite-{shape}"));
        for partition_size in sizes {
            let graph = if shape == "dense" {
                unweighted_bipartite_graph(&complete_bipartite_graph(partition_size))
            } else {
                sparse_bipartite_graph(partition_size)
            };
            // Keep correctness checks outside the timed closures. Both
            // algorithms must solve the same cardinality instance before a
            // speed comparison is meaningful.
            let hopcroft_karp =
                match hopcroft_karp_maximum_cardinality_matching(&graph, partition_size) {
                    Ok(result) => result,
                    Err(error) => {
                        report_failed_preflight(
                            &format!("maximum-cardinality/bipartite-{shape}"),
                            &format!("partition_size={partition_size}"),
                            format!("Hopcroft–Karp failed to solve: {error}"),
                        );
                        continue;
                    }
                };
            let fast_blossom = FastBlossomMatching::max_cardinality_matching(&graph);
            let preflight = hopcroft_karp
                .validate(&graph)
                .map_err(|error| format!("Hopcroft–Karp returned an invalid matching: {error}"))
                .and_then(|()| {
                    fast_blossom.validate(&graph).map_err(|error| {
                        format!("Fast Blossom returned an invalid matching: {error}")
                    })
                })
                .and_then(|()| {
                    if hopcroft_karp.edges.len() == fast_blossom.edges.len()
                        && hopcroft_karp.edges.len() == partition_size
                    {
                        Ok(())
                    } else {
                        Err(format!(
                            "cardinalities disagree: Hopcroft–Karp={}, Fast Blossom={}, expected={partition_size}",
                            hopcroft_karp.edges.len(),
                            fast_blossom.edges.len()
                        ))
                    }
                });
            if let Err(error) = preflight {
                report_failed_preflight(
                    &format!("maximum-cardinality/bipartite-{shape}"),
                    &format!("partition_size={partition_size}"),
                    error,
                );
                continue;
            }
            let parameter = format!("{partition_size}+{partition_size}");
            group.bench_with_input(
                BenchmarkId::new("hopcroft-karp", &parameter),
                &graph,
                |b, graph| {
                    b.iter(|| {
                        require_timed_result(hopcroft_karp_maximum_cardinality_matching(
                            graph,
                            partition_size,
                        ))
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::new("fast-blossom", &parameter),
                &graph,
                |b, graph| {
                    b.iter(|| FastBlossomMatching::max_cardinality_matching(graph));
                },
            );
        }
        group.finish();
    }
}

fn weighted_benchmarks_bipartite(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/bipartite-dense");
    for partition_size in [10, 25, 50, 100] {
        let graph = complete_bipartite_graph(partition_size);
        let parameter = format!("{partition_size}+{partition_size}");
        if let Err(error) = check_weighted_results(
            &graph,
            &[
                (
                    "hungarian",
                    blossom_study::algorithms::hungarian_minimum_weight_perfect_matching(
                        &graph,
                        partition_size,
                    ),
                ),
                (
                    "primal-dual",
                    blossom_study::algorithms::minimum_weight_perfect_matching(&graph),
                ),
                (
                    "blossom-engine",
                    blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(
                        &graph,
                    ),
                ),
                (
                    "sparse-blossom",
                    blossom_study::algorithms::sparse_minimum_weight_perfect_matching(&graph),
                ),
            ],
        ) {
            report_failed_preflight(
                "minimum-weight-perfect/bipartite-dense",
                &format!("partition_size={partition_size}"),
                error,
            );
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("hungarian", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    require_timed_result(
                        blossom_study::algorithms::hungarian_minimum_weight_perfect_matching(
                            graph,
                            partition_size,
                        ),
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("primal-dual", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    require_timed_result(
                        blossom_study::algorithms::minimum_weight_perfect_matching(graph),
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("blossom-engine", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    require_timed_result(
                        blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(
                            graph,
                        ),
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sparse-blossom", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    require_timed_result(
                        blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    cardinality_benchmarks,
    cardinality_benchmarks_bipartite,
    weighted_benchmarks_dense,
    weighted_benchmarks_sparse,
    duan_weighted_benchmarks,
    weighted_benchmarks_bipartite
);
criterion_main!(benches);
