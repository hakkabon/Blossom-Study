use blossom_study::{
    algorithms::{FastBlossomMatching, MicaliVazirani},
    graph::{Edge, Graph, generator},
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn cardinality_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("maximum-cardinality/general");
    for n in [20, 50, 100] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::new("fast-blossom", n), &graph, |b, graph| {
            b.iter(|| FastBlossomMatching::max_cardinality_matching(graph));
        });
    }
    group.finish();

    // MV is not placed beside completed solvers yet.  This group measures the
    // implemented alternating parity-level search only; its name prevents the
    // result from being mistaken for end-to-end maximum matching throughput.
    let mut development = c.benchmark_group("maximum-cardinality/mv-development-phase-1");
    for n in [20, 50, 100, 200] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/complete-dense", n),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let mut solver = MicaliVazirani::new(graph);
                    solver.run_level_search_phase(graph)
                });
            },
        );
    }

    for side in [5, 10, 15, 20] {
        let n = side * side;
        let graph = generator::grid_graph(side, side, (1.0, 100.0));
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/sparse-grid", n),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let mut solver = MicaliVazirani::new(graph);
                    solver.run_level_search_phase(graph)
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
        development.bench_with_input(
            BenchmarkId::new("mv-level-search/planted-50pct-matched", n),
            &graph,
            |b, graph| {
                let initial_mate = mate.clone();
                b.iter(|| {
                    let mut solver =
                        MicaliVazirani::with_matching(graph, initial_mate.clone()).unwrap();
                    solver.run_level_search_phase(graph)
                });
            },
        );
    }
    development.finish();
}

fn weighted_benchmarks_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/general-dense");
    for n in [20, 50, 100, 200] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::new("primal-dual", n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::minimum_weight_perfect_matching(graph));
        });
        group.bench_with_input(BenchmarkId::new("blossom-engine", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("blossom-tree", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::blossom_tree_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("cherry-tree", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::cherry_tree_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("sparse-blossom", n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph));
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
        group.bench_with_input(BenchmarkId::new("primal-dual", n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::minimum_weight_perfect_matching(graph));
        });
        group.bench_with_input(BenchmarkId::new("blossom-engine", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("blossom-tree", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::blossom_tree_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("cherry-tree", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::cherry_tree_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(BenchmarkId::new("sparse-blossom", n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph));
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

fn weighted_benchmarks_bipartite(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/bipartite-dense");
    for partition_size in [10, 25, 50, 100] {
        let graph = complete_bipartite_graph(partition_size);
        let parameter = format!("{partition_size}+{partition_size}");

        group.bench_with_input(
            BenchmarkId::new("hungarian", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    blossom_study::algorithms::hungarian_minimum_weight_perfect_matching(
                        graph,
                        partition_size,
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("primal-dual", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| blossom_study::algorithms::minimum_weight_perfect_matching(graph));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("blossom-engine", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    blossom_study::algorithms::blossom_engine_minimum_weight_perfect_matching(graph)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("blossom-tree", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    blossom_study::algorithms::blossom_tree_minimum_weight_perfect_matching(graph)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("cherry-tree", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| {
                    blossom_study::algorithms::cherry_tree_minimum_weight_perfect_matching(graph)
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sparse-blossom", &parameter),
            &graph,
            |b, graph| {
                b.iter(|| blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph));
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    cardinality_benchmarks,
    weighted_benchmarks_dense,
    weighted_benchmarks_sparse,
    weighted_benchmarks_bipartite
);
criterion_main!(benches);
