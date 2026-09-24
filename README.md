# Blossom Algorithm Study

A Rust project for studying and benchmarking Edmonds' Blossom Algorithm implementations for maximum cardinality matching and minimum weight perfect matching.

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
│       ├── edmonds.rs      # Explicit-blossom implementation
│       ├── korte_vygen.rs  # Union-Find implementation
│       └── fast_blossom.rs # DSU/timestamp implementation
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
- ✅ Maximum cardinality matching (Edmonds' Blossom Algorithm)
- ✅ Graph generation (complete, sparse, grid)
- ✅ JSON serialization for graphs
- ✅ Benchmark framework
- ✅ Basic blossom contraction/expansion

### Known Limitations
- ⚠️ Nested blossom handling creates excessive blossoms in some graph types (e.g., grids)
- ⚠️ No proper handling of blocked blossoms in unweighted case
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

# Minimum weight perfect matching (weighted graphs only), selecting any adapter
cargo run -- --algorithm fast-blossom min-weight-perfect -i graph.json
```

### Benchmark
```bash
cargo run --bin bench
```

## Algorithm Details

The unweighted implementations follow Gerards (1995) "Matching" and the Korte–Vygen presentation. Weighted matching uses the `integer-blossom` O(V³) primal-dual implementation. Finite `f64` costs are converted exactly to a common `i128` fixed-point representation; inputs with an exponent range too wide for `i128` are rejected explicitly.

Key components:
- **Alternating tree growth** (`grow()`) - BFS for augmenting paths
- **Blossom contraction** (`blossom()`) - Odd cycle detection and contraction
- **Blossom expansion** (`expand()`) - Recursive blossom dissolution
- **Augmentation** (`augment()`) - Path flipping along augmenting paths
- **Primal-dual weighted matching** - O(V³), with exact dual arithmetic and sparse-graph support

## Future Work

1. **Fix nested blossom handling** - Proper blossom contraction/expansion cycle
2. **Expose weighted-solver internals in benchmark metadata** - report contractions and dual updates
3. **Implement priority queues for weighted dual updates** - improve practical performance
4. **Add Blossom V/VI integration** - FFI bindings to state-of-the-art implementations
5. **Benchmark against Blossom V/VI** - Performance comparison framework

## References

- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
- Arkhipov, P., & Kolmogorov, V. (2026). Blossom VI. *arXiv preprint*.
