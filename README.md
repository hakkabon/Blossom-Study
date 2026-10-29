# Blossom Algorithm Study

A Rust project for studying maximum-cardinality and minimum-weight perfect matching algorithms in general and bipartite graphs.

## Project Structure

```
blossom-study/
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # CLI and REPL for running matching algorithms
│   ├── graph/
│   │   └── mod.rs          # Graph representations and generators
│   └── algorithms/
│       ├── mod.rs          # Common types and traits
│       ├── gabow_cardinality.rs # Gabow's phased cardinality solver (Mehlhorn et al. 2024)
│       ├── micali_vazirani.rs # Micali–Vazirani phased O(m√n) cardinality solver
│       ├── fast_blossom.rs # Conventional blossom cardinality baseline
│       ├── hopcroft_karp.rs # Bipartite maximum-cardinality solver
│       ├── hungarian.rs    # Bipartite assignment primal-dual solver
│       ├── blossom_engine.rs # Independent weighted blossom solver
│       ├── blossom_tree.rs # Flat-containment adapter
│       ├── blossom_tree_flat.rs # MIT-attributed Blossom-Tree engine
│       ├── cherry_tree_solver.rs # Self-contained repaired Cherry-Tree solver
│       ├── mwpm.rs         # Active weighted primal-dual solver adapter
│       ├── sparse_mwpm.rs  # Sparse event-driven experiment
│       └── korte_vygen.rs  # Reference-only implementation
├── tests/
│   ├── gabow_cardinality_test.rs # Gabow cardinality test suite & oracle verification
│   ├── micali_vazirani_test.rs # Micali–Vazirani tests, nested petals, and oracle checks
│   ├── hopcroft_karp_test.rs # Bipartite cardinality/oracle tests
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
- ✅ **Gabow's Cardinality Matching** (`gabow`): $O(m \sqrt{n})$ phased cardinality matching based on Gabow (2017) and Mehlhorn et al. (Sept 2024, arXiv:2409.14849). Features Phase 1 auxiliary graph $H$ construction with blossom contraction and Phase 2 maximal disjoint shortest augmenting path extraction.
- ✅ **Micali–Vazirani Phased Cardinality Matching** (`micali-vazirani`): $O(m \sqrt{n})$ general-graph cardinality solver featuring dynamic MIN/MAX parity levels, tenacity-ordered bridge queues, Double Depth-First Search (DDFS) petal detection, recursive blossom unrolling, and multi-phase augmentation.
- ✅ **Conventional Blossom Cardinality** (`fast-blossom`): Edmonds blossom contraction with timestamp-based LCA and DSU.
- ✅ **Hopcroft–Karp Bipartite Matching** (`hopcroft-karp`): $O(m \sqrt{n})$ multi-phase bipartite maximum cardinality matching.
- ✅ **O(V³) Minimum-Weight Perfect Matching**:
  - `primal-dual`: Standard dense-matrix primal-dual baseline with exact `i128` dual arithmetic.
  - `blossom-engine`: Independent Galil/Edmonds primal-dual solver with explicit alternating forests and nested blossoms.
  - `blossom-tree`: Flat-containment Blossom-Tree solver adapted from MIT-licensed `mwmatching`.
  - `cherry-tree`: Self-contained repaired Cherry-Tree primal-dual solver with recursive path lifting.
  - `sparse-blossom`: Sparse event-driven adjacency/map implementation.
- ✅ **Hungarian Bipartite Assignment**: $O(k^3)$ primal-dual shortest augmenting paths with exact cost scaling.
- ✅ **Graph Generators & Serialization**: Complete, sparse, grid, and planted matching generators with JSON serialization.
- ✅ **Comprehensive Test Oracles**: All solvers validated against exhaustive brute-force oracles and cross-solver baselines.

### Performance Summary (Debug Build)

#### Maximum Cardinality Matching Comparison

| Graph Family | Vertices | Edges | Algorithm | Match Size | Time (ms) | Phases |
|--------------|----------|-------|-----------|------------|-----------|--------|
| **Complete $K_{20}$** | 20 | 190 | `fast-blossom` | 10 | 0.05 | - |
| | 20 | 190 | `gabow` | 10 | 0.07 | 2 |
| | 20 | 190 | `mv` | 10 | 1.05 | 2 |
| **Complete $K_{50}$** | 50 | 1,225 | `gabow` | 25 | 0.39 | 2 |
| | 50 | 1,225 | `mv` | 25 | 15.63 | 2 |
| **Complete $K_{100}$** | 100 | 4,950 | `gabow` | 50 | 1.63 | 2 |
| | 100 | 4,950 | `mv` | 50 | 173.11 | 2 |
| **Grid $10 \times 10$** | 100 | 180 | `gabow` | 50 | 0.22 | 2 |
| | 100 | 180 | `mv` | 50 | 1.00 | 2 |
| **Grid $20 \times 20$** | 400 | 760 | `gabow` | 200 | 0.62 | 2 |
| | 400 | 760 | `mv` | 200 | 7.47 | 2 |
| **Grid $30 \times 30$** | 900 | 1,740 | `gabow` | 450 | 1.18 | 2 |
| | 900 | 1,740 | `mv` | 450 | 29.46 | 2 |
| **Planted $n=100$** | 100 | 315 | `gabow` | 50 | 0.19 | 3 |
| | 100 | 315 | `mv` | 50 | 1.79 | 2 |

> [!NOTE]
> **Asymptotic Bound vs. Practical Constant Factors**: While both Micali–Vazirani (1980) and Gabow (2017 / Mehlhorn et al. 2024) share the exact same theoretical worst-case time complexity of $O(m \sqrt{n})$, Gabow's algorithm runs **10x to 100x faster in practice**. Gabow's auxiliary graph $H$ and blossom contraction avoid the high constant-factor overhead of MV's dual MIN/MAX level bookkeeping, backtrack lists, and DDFS petal discovery.

---

## Usage

### CLI

```bash
# Generate a test graph
cargo run -- generate -n 20 -o graph.json

# Maximum cardinality matching using Fast Blossom (default)
cargo run -- max-cardinality -i graph.json

# Maximum cardinality matching using Gabow's algorithm (Mehlhorn et al. 2024)
cargo run -- --algorithm gabow max-cardinality -i graph.json

# Maximum cardinality matching using Micali-Vazirani
cargo run -- --algorithm micali-vazirani max-cardinality -i graph.json

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

### Benchmarks

```bash
# Run the complete comparison runner (Cardinality + MWPM + Bipartite)
cargo run --bin bench

# Statistically robust Criterion benchmark suite
cargo bench --bench matching
```

---

## Algorithm Details

### 1. Gabow's Cardinality Matching Algorithm (Mehlhorn et al., Sept 2024)

Implemented in [`src/algorithms/gabow_cardinality.rs`](src/algorithms/gabow_cardinality.rs). Based on:
> *Gabow's Cardinality Matching Algorithm in General Graphs: Implementation and Experiments*  
> Matin Ansaripour, Alireza Danaei, and Kurt Mehlhorn (arXiv:2409.14849, Sept 24, 2024).

The algorithm runs in $O(\sqrt{n})$ phases, each taking $O(m)$ time, achieving an overall runtime of $O(m \sqrt{n})$:

- **Phase 1 (Search & Auxiliary Graph $H$)**:
  - Performs a multi-source breadth-first search from all currently exposed (unmatched) vertices.
  - Maintains outer (even distance) and inner (odd distance) labels.
  - When an edge connects two outer vertices in the same alternating tree, an odd cycle (blossom) is detected. The blossom base (LCA) is identified, the blossom is contracted in the union-find structure, and parent links are updated around the cycle so path lifting remains direct.
  - When an edge connects two outer vertices in different trees, an augmenting path is found. The shortest augmenting path length $L$ is established, and all augmenting bridges of length $L$ are collected.
- **Phase 2 (Maximal Disjoint Path Extraction & Augmentation)**:
  - Traverses the auxiliary graph $H$ to extract a maximal set of vertex-disjoint shortest augmenting paths of length $L$.
  - Internal blossom cycles are unrolled via parent pointers (`p`, `orig_p`).
  - Matched and unmatched edges along each path are flipped, strictly increasing the cardinality of the matching.

### 2. Micali–Vazirani (MV) Cardinality Algorithm (1980)

Implemented in [`src/algorithms/micali_vazirani.rs`](src/algorithms/micali_vazirani.rs). Based on:
> *An $O(\sqrt{|V|} \cdot |E|)$ Algorithm for Finding Maximum Matching in General Graphs*  
> Silvio Micali and Vijay V. Vazirani (FOCS 1980).

Key components:
- **Dynamic Level Search**: Alternating BFS that assigns dual levels: $\min(v)$ (the length of the shortest alternating walk from an exposed vertex to $v$) and $\max(v)$ (the length of the shortest walk after blossom contraction).
- **Tenacity Priority Queues**: Candidate edges (bridges) are bucketed by tenacity $2k + 1$ into an $O(1)$-access queue.
- **Double Depth-First Search (DDFS)**: Searches backwards from bridge endpoints simultaneously at equal pace. If searches collide at a common ancestor, a petal/blossom is discovered without requiring physical contraction; if they reach distinct roots, an augmenting path is formed.
- **Petal Unrolling**: Petals are recursively unpacked along precursor pointers to yield valid alternating paths.
- **Maximal Disjoint Augmentation Engine**: Dequeues candidate paths at length $L$, lifts them through nested petals, and commits non-overlapping augmentations.

### 3. Hopcroft–Karp (1973)

Bipartite $O(m \sqrt{n})$ cardinality baseline. A multi-source BFS layers exposed left vertices, and a DFS extracts a maximal collection of vertex-disjoint shortest augmenting paths in each phase.

### 4. Weighted Perfect Matching (MWPM) Solvers ($O(V^3)$)

Five general-graph weighted implementations:
| CLI solver | Representation | Status |
| --- | --- | --- |
| `primal-dual` | Edge-list Galil/Edmonds, exact `i128` duals | Established baseline |
| `blossom-engine` | Explicit alternating forest and nested blossoms, exact `i128` duals | Independent verified implementation |
| `blossom-tree` | Flat contraction history and hash-based containment | Adapted from MIT-licensed `mwmatching` |
| `sparse-blossom` | Adjacency lists and incident-edge maps, exact `i128` duals | Experimental sparse-graph solver |
| `cherry-tree` | Ordered cherry blossoms with recursive path lifting | Self-contained repaired implementation |

---

## References

- Ansaripour, M., Danaei, A., & Mehlhorn, K. (2024). *Gabow's Cardinality Matching Algorithm in General Graphs: Implementation and Experiments*. arXiv:2409.14849 [cs.DS].
- Gabow, H.N. (2017). *Data Structures for Weighted Matching and Extensions to b-matching*. arXiv:1611.07541.
- Micali, S., & Vazirani, V.V. (1980). An $O(\sqrt{|V|} \cdot |E|)$ algorithm for finding maximum matching in general graphs. *21st Annual Symposium on Foundations of Computer Science (FOCS 1980)*, 17–27.
- Hopcroft, J.E., & Karp, R.M. (1973). An $n^{5/2}$ algorithm for maximum matchings in bipartite graphs. *SIAM Journal on Computing*, 2(4), 225–231.
- Galil, Z. (1986). Efficient Algorithms for Finding Maximum Matching in Graphs. *ACM Computing Surveys*, 18(1), 23–38.
- Kuhn, H.W. (1955). The Hungarian method for the assignment problem. *Naval Research Logistics Quarterly*, 2, 83–97.
- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
