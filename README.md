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
│       ├── cherry_tree_solver.rs # Self-contained repaired Cherry-Tree solver
│       ├── fast_blossom.rs # Active conventional cardinality baseline
│       ├── hungarian.rs    # Bipartite assignment primal-dual solver
│       ├── micali_vazirani.rs # Phased cardinality implementation
│       ├── mwpm.rs         # Active weighted primal-dual solver adapter
│       ├── sparse_mwpm.rs  # Sparse event-driven experiment
│       └── korte_vygen.rs  # Reference-only implementation
├── tests/
│   ├── hungarian_test.rs   # Bipartite assignment/oracle tests
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
- 🚧 Micali–Vazirani alternating level-search milestone
- ✅ O(V³) minimum-weight perfect matching
- ✅ Independent Galil/Edmonds primal-dual MWPM solver (`blossom-engine`)
- ✅ Flat-containment Blossom-Tree solver (`blossom-tree`)
- 🧪 Sparse event-driven solver (`sparse-blossom`)
- ✅ Self-contained repaired Cherry-Tree primal-dual solver (`cherry-tree`)
- ✅ Hungarian-style bipartite MWPM solver with exact `i128` reduced costs
- ✅ Graph generation (complete, sparse, grid)
- ✅ JSON serialization for graphs
- ✅ Benchmark framework
- ✅ Criterion and CLI benchmark frameworks

### Known Limitations
- ⚠️ Micali–Vazirani now discovers alternating parity levels and prop/bridge candidates, but DDFS, tenacity-ordered bridge processing, petals, set merging, and augmentation remain incomplete
- ⚠️ Weighted solver metadata currently reports runtime but not its internal contractions or dual updates
- ⚠️ Cherry-Tree and Blossom Engine implement the same Edmonds/Galil primal-dual framework with separate solver state; benchmark differences therefore reflect implementation structure rather than different asymptotic complexity
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

# Repaired self-contained Cherry-Tree implementation
cargo run -- min-weight-perfect --solver cherry-tree -i graph.json
```

### Benchmark
```bash
cargo run --bin bench

# Statistically robust Criterion suite with HTML reports
cargo bench --bench matching
```

The Criterion suite is divided into deliberately non-interchangeable groups:

- `maximum-cardinality/general` — completed general-graph cardinality solvers.
- `maximum-cardinality/mv-development-phase-1` — MV level search only; not an end-to-end matching result.
- `minimum-weight-perfect/general-dense` — all general MWPM solvers on complete graphs.
- `minimum-weight-perfect/general-sparse` — the same solvers on planted sparse instances.
- `minimum-weight-perfect/bipartite-dense` — Hungarian plus every general MWPM solver on identical assignment graphs.

Hungarian requires an explicit balanced bipartition. The public convention is
left vertices `0..k` and right vertices `k..2k`:

```rust
use blossom_study::hungarian_minimum_weight_perfect_matching;

let result = hungarian_minimum_weight_perfect_matching(&graph, k)?;
```

## Algorithm Details

Fast Blossom is the active conventional unweighted baseline. Korte–Vygen is retained under `algorithms::reference` for study but is excluded from normal benchmarks. The former explicit-blossom implementation is retired from the public API.

Five general-graph weighted implementations are collected:

| CLI solver | Representation | Status |
| --- | --- | --- |
| `primal-dual` | edge-list Galil/Edmonds, exact `i128` duals | established baseline |
| `blossom-engine` | explicit alternating forest and nested blossoms, exact `i128` duals | independent verified implementation |
| `blossom-tree` | flat contraction history and hash-based containment, exact `i128` duals | verified MIT-attributed adaptation |
| `sparse-blossom` | adjacency lists and incident-edge maps, exact `i128` duals | experimental; intended sparse-graph solver |
| `cherry-tree` | ordered cherry blossoms with recursive path lifting and exact `i128` duals | self-contained repaired implementation |

All five solvers are cross-checked against brute-force or established solver oracles. The retired Cherry prototypes stopped when a greedy matching was merely perfect, could report augmentation without changing the matching, and represented a contracted blossom as an unordered vertex set. The repaired solver starts from a dual-feasible state, augments only on tight edges, records ordered nested blossom children and original boundary endpoints, and recursively lifts augmenting paths during contraction and expansion. It accepts sparse as well as complete graphs.

The Hungarian solver is a sixth, domain-specific baseline. It accepts balanced
bipartite graphs, including sparse graphs with missing assignments. Its
shortest-augmenting-path implementation maintains row/column potentials and a
minimum slack for each unreached right vertex. No odd-set duals or blossom
contractions are needed. Randomized tests compare it against the general MWPM
baseline on the same graphs.

Key components:
- **Fast Blossom** - DSU contraction with timestamp-based LCA
- **Micali–Vazirani track** - alternating even/odd levels and preliminary prop/bridge classification; later phases remain explicit rather than using a fallback
- **Hungarian assignment** - O(k³) bipartite primal-dual shortest augmenting paths with exact cost scaling
- **Primal-dual weighted matching** - O(V³), with exact dual arithmetic and sparse-graph support
- **Blossom Engine** - independent, extensively commented Galil/Edmonds primal-dual implementation
- **Blossom Tree** - MIT-attributed flat-containment adaptation with immutable contraction history
- **Sparse Blossom** - experimental event-driven adjacency/map representation
- **Cherry Tree** - self-contained ordered-blossom reconstruction with explicit alternating-tree, contraction, expansion, and dual-update state

## Provenance and licensing

`Blossom-Tree` is adapted from `mwmatching` 0.1.1 and remains under its MIT license. The required notice is retained in [`LICENSE-MWMATCHING`](LICENSE-MWMATCHING). No `mwmatching` crate dependency is used. The repaired Cherry-Tree solver is project source and does not call another matching implementation in the crate.

## Future Work

1. **Complete MV bridge tenacity** - order candidate bridges for the current phase
2. **Implement MV DDFS and petals** - include efficient set-merging support
3. **Implement MV phase augmentation** - find a maximal set of vertex-disjoint shortest augmenting paths
4. **Add external Blossom V/VI benchmark adapters** - executables remain outside this repository
5. **Expose weighted-solver internals** - report contractions and dual updates

## References

- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Galil, Z. (1986). Efficient Algorithms for Finding Maximum Matching in Graphs. *ACM Computing Surveys*, 18(1), 23–38. https://doi.org/10.1145/6462.6502
- Kuhn, H.W. (1955). The Hungarian method for the assignment problem. *Naval Research Logistics Quarterly*, 2, 83–97.
- Micali, S., & Vazirani, V.V. (1980). An O(sqrt(V)E) algorithm for finding maximum matching in general graphs. *FOCS 1980*, 17–27.
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
- Arkhipov, P., & Kolmogorov, V. (2026). Blossom VI. *arXiv preprint*.
