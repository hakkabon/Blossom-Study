use blossom_study::{
    algorithms::{EdmondsAlgorithm, FastBlossomMatching, KorteVygenEdmonds},
    graph::generator,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn cardinality_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("maximum-cardinality");
    for n in [20, 50, 100] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::new("edmonds", n), &graph, |b, graph| {
            b.iter(|| EdmondsAlgorithm::new(graph).max_cardinality(graph));
        });
        group.bench_with_input(BenchmarkId::new("korte-vygen", n), &graph, |b, graph| {
            b.iter(|| KorteVygenEdmonds::max_cardinality_matching(graph));
        });
        group.bench_with_input(BenchmarkId::new("fast-blossom", n), &graph, |b, graph| {
            b.iter(|| FastBlossomMatching::max_cardinality_matching(graph));
        });
    }
    group.finish();
}

fn weighted_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect");
    for n in [20, 50, 100, 200] {
        let graph = generator::complete_graph(n, (1.0, 100.0));
        group.bench_with_input(BenchmarkId::from_parameter(n), &graph, |b, graph| {
            b.iter(|| blossom_study::algorithms::minimum_weight_perfect_matching(graph));
        });
    }
    group.finish();
}

criterion_group!(benches, cardinality_benchmarks, weighted_benchmarks);
criterion_main!(benches);
