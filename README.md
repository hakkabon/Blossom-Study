# Blossom Algorithm Study

A Rust project for studying maximum-cardinality and minimum-weight perfect matching algorithms.

## Project Structure

```
blossom-study/
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # CLI for running matching algorithms
│   ├── graph/
│   │   └── mod.rs          # Graph representations and generators
│   └── algorithms/
│       ├── mod.rs          # Common types and traits
│       ├── fast_blossom.rs # Active conventional cardinality baseline
│       ├── micali_vazirani.rs # Experimental phase-based scaffold
│       ├── mwpm.rs         # Active weighted primal-dual solver adapter
│       └── korte_vygen.rs  # Reference-only implementation
├── tests/
│   ├── matching.rs         # Integration and cross-implementation tests
│   └── data/               # Test graph fixtures
├── bench/
│   └── main.rs             # Benchmark runner
├── Cargo.toml
└── README.md
```

## Current Status

### Working Features
- ✅ Maximum cardinality matching (`fast-blossom`)
- 🚧 Micali–Vazirani state and phase scaffold
- ✅ O(V³) minimum-weight perfect matching
- ✅ Graph generation (complete, sparse, grid)
- ✅ JSON serialization for graphs
- ✅ Benchmark framework
- ✅ Criterion and CLI benchmark frameworks

### Known Limitations
- ⚠️ Micali–Vazirani level search, bridge processing, petals, and augmentation are not implemented yet
- ⚠️ Weighted solver metadata currently reports runtime but not its internal contractions or dual updates

### Performance (Debug Build)
| Graph | Vertices | Edges | Match Size | Time (ms) | Blossoms |
|-------|----------|-------|------------|-----------|----------|
| K6    | 6        | 15    | 3          | 0.03      | 2        |
| K10   | 10       | 45    | 5          | 0.04      | 4        |
| K20   | 20       | 190   | 10         | 0.08      | 9        |
| Sparse (20, 0.3) | 20 | 60 | 10 | 0.06 | 3 |

## Usage

### CLI
```bash
# Generate a test graph
cargo run -- generate -n 20 -o graph.json

# Maximum cardinality matching
cargo run -- max-cardinality -i graph.json

# Minimum weight perfect matching (weighted graphs only)
cargo run -- min-weight-perfect -i graph.json
```

### Benchmark
```bash
cargo run --bin bench

# Statistically robust Criterion suite with HTML reports
cargo bench --bench matching
```

## Algorithm Details

Fast Blossom is the active conventional unweighted baseline. Korte–Vygen is retained under `algorithms::reference` for study but is excluded from normal benchmarks. The former explicit-blossom implementation is retired from the public API. Weighted matching uses the `integer-blossom` O(V³) primal-dual implementation. Finite `f64` costs are converted exactly to a common `i128` fixed-point representation; inputs with an exponent range too wide for `i128` are rejected explicitly.

Key components:
- **Fast Blossom** - DSU contraction with timestamp-based LCA
- **Micali–Vazirani scaffold** - explicit phases, MIN/MAX levels, predecessors, bridges, and petal bases
- **Primal-dual weighted matching** - O(V³), with exact dual arithmetic and sparse-graph support

## Future Work

1. **Implement Micali–Vazirani level search** - BFS levels and tenacity
2. **Implement bridge processing and petals** - set-merging support
3. **Implement phase augmentation** - maximal set of vertex-disjoint shortest augmenting paths
4. **Add external Blossom V/VI benchmark adapters** - executables remain outside this repository
5. **Expose weighted-solver internals** - report contractions and dual updates

## References

- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
- Arkhipov, P., & Kolmogorov, V. (2026). Blossom VI. *arXiv preprint*.
