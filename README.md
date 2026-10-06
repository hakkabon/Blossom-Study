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
│       ├── blossom_engine.rs # Independent weighted blossom solver
│       ├── blossom_tree.rs # Flat-containment adapter
│       ├── blossom_tree_flat.rs # MIT-attributed Blossom-Tree engine
│       ├── cherry_tree.rs # Complete-graph Cherry-Tree adapter
│       ├── cherry_tree_prototype.rs # Corrected dense-matrix front end
│       ├── fast_blossom.rs # Active conventional cardinality baseline
│       ├── micali_vazirani.rs # Experimental phase-based scaffold
│       ├── mwpm.rs         # Active weighted primal-dual solver adapter
│       ├── sparse_mwpm.rs  # Sparse event-driven experiment
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
- ✅ Independent Galil/Edmonds primal-dual MWPM solver (`blossom-engine`)
- ✅ Flat-containment Blossom-Tree solver (`blossom-tree`)
- 🧪 Sparse event-driven solver (`sparse-blossom`)
- ✅ Corrected Cherry-Tree dense-matrix solver (`cherry-tree-experimental`)
- ✅ Graph generation (complete, sparse, grid)
- ✅ JSON serialization for graphs
- ✅ Benchmark framework
- ✅ Criterion and CLI benchmark frameworks

### Known Limitations
- ⚠️ Micali–Vazirani level search, bridge processing, petals, and augmentation are not implemented yet
- ⚠️ Weighted solver metadata currently reports runtime but not its internal contractions or dual updates
- ⚠️ Cherry-Tree retains its historical `experimental` CLI name, but now uses the verified hierarchical primal-dual kernel; it is not an independent algorithmic baseline
- ⚠️ SparseBlossomSolver targets `O(VE)` behavior, but its contraction-map rebuilding has not received a formal amortized analysis

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

# Minimum weight perfect matching with the established baseline
cargo run -- min-weight-perfect -i graph.json

# Independent in-repository Galil/Edmonds implementation
cargo run -- min-weight-perfect --solver blossom-engine -i graph.json

# Sparse event-driven experiment
cargo run -- min-weight-perfect --solver sparse-blossom -i graph.json

# Flat-containment Blossom-Tree variant
cargo run -- min-weight-perfect --solver blossom-tree -i graph.json

# Corrected complete-distance-matrix variant (legacy CLI name retained)
cargo run -- min-weight-perfect --solver cherry-tree-experimental -i graph.json
```

### Benchmark
```bash
cargo run --bin bench

# Statistically robust Criterion suite with HTML reports
cargo bench --bench matching
```

## Algorithm Details

Fast Blossom is the active conventional unweighted baseline. Korte–Vygen is retained under `algorithms::reference` for study but is excluded from normal benchmarks. The former explicit-blossom implementation is retired from the public API.

Five weighted implementations are collected:

| CLI solver | Representation | Status |
| --- | --- | --- |
| `primal-dual` | edge-list Galil/Edmonds, exact `i128` duals | established baseline |
| `blossom-engine` | explicit alternating forest and nested blossoms, exact `i128` duals | independent verified implementation |
| `blossom-tree` | flat contraction history and hash-based containment, exact `i128` duals | verified MIT-attributed adaptation |
| `sparse-blossom` | adjacency lists and incident-edge maps, exact `i128` duals | experimental; intended sparse-graph solver |
| `cherry-tree-experimental` | complete distance matrix converted to the endpoint-aware hierarchical kernel | verified compatibility variant |

All five solvers are cross-checked against brute-force or established solver oracles. Cherry-Tree still accepts only complete graphs because its educational interface is a dense distance matrix. Its former DSU/bridge state machine was removed after same-tree collisions were shown to corrupt mate relations even on `K4`. The corrected front end now shares the exact hierarchical Galil/Edmonds kernel used by `blossom-engine`, and its Criterion row therefore measures dense-matrix construction and conversion overhead rather than an independent matching algorithm.

Key components:
- **Fast Blossom** - DSU contraction with timestamp-based LCA
- **Micali–Vazirani scaffold** - explicit phases, MIN/MAX levels, predecessors, bridges, and petal bases
- **Primal-dual weighted matching** - O(V³), with exact dual arithmetic and sparse-graph support
- **Blossom Engine** - independent, extensively commented Galil/Edmonds primal-dual implementation
- **Blossom Tree** - MIT-attributed flat-containment adaptation with immutable contraction history
- **Sparse Blossom** - experimental event-driven adjacency/map representation
- **Cherry Tree** - corrected dense-matrix compatibility front end with reciprocal-matching invariants

## Provenance and licensing

`Blossom-Tree` is adapted from `mwmatching` 0.1.1 and remains under its MIT license. The required notice is retained in [`LICENSE-MWMATCHING`](LICENSE-MWMATCHING). No `mwmatching` crate dependency is used. Cherry-Tree's matrix adapter is project source; its corrected combinatorial search reuses this repository's independently implemented `blossom-engine` kernel.

## Future Work

1. **Implement Micali–Vazirani level search** - BFS levels and tenacity
2. **Implement bridge processing and petals** - set-merging support
3. **Implement phase augmentation** - maximal set of vertex-disjoint shortest augmenting paths
4. **Add external Blossom V/VI benchmark adapters** - executables remain outside this repository
5. **Expose weighted-solver internals** - report contractions and dual updates

## References

- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Galil, Z. (1986). Efficient Algorithms for Finding Maximum Matching in Graphs. *ACM Computing Surveys*, 18(1), 23–38. https://doi.org/10.1145/6462.6502
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
- Arkhipov, P., & Kolmogorov, V. (2026). Blossom VI. *arXiv preprint*.
