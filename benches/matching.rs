use blossom_study::{
    algorithms::FastBlossomMatching,
    graph::{Edge, Graph, generator},
};
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
        group.bench_with_input(BenchmarkId::new("blossom-tree", n), &graph, |b, graph| {
            b.iter(|| {
                blossom_study::algorithms::blossom_tree_minimum_weight_perfect_matching(graph)
            });
        });
        group.bench_with_input(
            BenchmarkId::new("cherry-tree-dense", n),
            &graph,
            |b, graph| {
                b.iter(|| {
                    blossom_study::algorithms::cherry_tree_minimum_weight_perfect_matching(graph)
                });
            },
        );
    }
    group.finish();
}

fn weighted_benchmarks_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimum-weight-perfect/sparse-event-driven");
    for n in [20, 50, 100, 200, 500] {
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
