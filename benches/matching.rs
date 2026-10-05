use blossom_study::{algorithms::FastBlossomMatching, graph::generator};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn cardinality_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("maximum-cardinality");
    for n in [20, 50, 100] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::new("fast-blossom", n), &graph, |b, graph| {
            b.iter(|| FastBlossomMatching::max_cardinality_matching(graph));
        });
    }
    group.finish();
}

fn weighted_benchmarks_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/dense-O(V3)");
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
    }
    group.finish();
}

fn weighted_benchmarks_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/sparse-subcubic");
    for n in [20, 50, 100, 200, 500] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::from_parameter(n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::sparse_minimum_weight_perfect_matching(graph));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    cardinality_benchmarks,
    weighted_benchmarks_dense,
    weighted_benchmarks_sparse
);
criterion_main!(benches);
