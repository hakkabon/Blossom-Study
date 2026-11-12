# Benchmark records

Each dated directory contains the input `config.json`, raw
`bench_results.json`, and a `manifest.json` recording the source revision,
toolchain, command, and result coverage. The raw JSON is the configurable CLI
runner's output; each row contains a problem category, solver name, graph size
and edge count, requested density and weight range, observed weight bounds,
seed, run number, graph signature, matching, and solver metadata.

Reproduce the saved smoke run from the repository root with:

```bash
cargo run --release --offline --bin blossom_study -- bench \
  --config bench/records/2026-11-12-smoke/config.json \
  --output /tmp/blossom-study-smoke
```

Compare graph signatures and matching cardinalities or optimal weights, not
the raw elapsed times. The same seed and pinned `Cargo.lock` reproduce the
input sequence for this version of the generator. If its implementation or
dependency versions change, compare signatures before treating two rows as
the same graph. The runner validates each returned matching and compares
selected solvers within each problem category, but agreement is not an
independent optimality proof. A solver that returns an error is skipped; check
the row count in the manifest before interpreting a run.

`result.metadata.runtime_ms` is one solver-internal wall-clock measurement. It
includes work inside the solver adapter, but excludes graph construction,
output serialization, and the runner's result validation. The first invocation
may include cold-cache effects. These small smoke records are for checking
coverage and result consistency, **not** ranking speed or inferring complexity.

For statistical timing, use `cargo bench --bench matching` and archive the
Criterion output with its command, toolchain, source revision, graph generator
parameters, preflight status, and hardware details. Some Criterion groups use
random complete-graph weights, so their generated inputs are shared within a
single group execution but are not identical across separate invocations.
The MV group times only its level-search phase. Keep general-cardinality,
general-weighted, and bipartite comparisons separate.

For isolated per-solver time and peak-process-memory measurements across size,
density, and weight range, see the [profiling workflow](../profiles/README.md).
