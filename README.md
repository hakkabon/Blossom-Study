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
│       ├── general_cardinality.rs # Named general-graph cardinality solver registry
│       ├── gabow_heuristic.rs # Gabow-inspired heuristic cardinality solver
│       ├── micali_vazirani.rs # Experimental phased Micali–Vazirani track
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
│   ├── gabow_cardinality_test.rs # Gabow-inspired heuristic tests
│   ├── micali_vazirani_test.rs # Micali–Vazirani tests, nested petals, and oracle checks
│   ├── hopcroft_karp_test.rs # Bipartite cardinality/oracle tests
│   ├── hungarian_test.rs   # Bipartite assignment/oracle tests
│   ├── oracle_comparison_test.rs # Independent exact oracles for cardinality and weighted solvers
│   ├── matching.rs         # Integration and cross-implementation tests
│   └── data/               # Test graph fixtures
├── bench/
│   └── main.rs             # Benchmark runner
├── Cargo.toml
└── README.md
```

## Current Status

### Working Features
- 🧪 **Gabow-inspired cardinality** (`gabow`): phased alternating search with blossom contraction and an Edmonds-style single-path search when a phase stalls. It does **not** construct the paper's tight-edge auxiliary graph $H$ or establish its $O(m\sqrt n)$ bound. An eight-vertex false-optimum regression is retained.
- ⚠️ **Experimental Micali–Vazirani research track** (`micali-vazirani`): parity levels, bridge tenacity, DDFS proposals, and petal unrolling. Nested-petal discovery and the complete algorithm have not been independently established as complete. Recursive path and petal certificates share a fixed 100,000-state safety budget; exceeding it returns an error rather than risking an unbounded search or claiming a maximum. This is not a dependable general-purpose solver and the published MV time bound does **not** apply.
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
- ✅ **Independent exact test oracles**: registered general-cardinality solvers are checked on every simple graph through six vertices. All five weighted general-graph solvers are checked against an independent dynamic-programming oracle on reproducible sparse instances up to ten vertices, with odd, infeasible, and empty graph cases. These finite tests provide regression coverage, not a proof of correctness on arbitrary graphs.

### Performance interpretation

Use the benchmark commands below for measurements on your machine. The generic runner compares completed solvers only; it excludes both experimental phased cardinality tracks. Criterion includes a separately named MV level-search-only development group, which is not an end-to-end solver benchmark. No MV timing constitutes a comparison with the published algorithm or evidence for its asymptotic bound. Check matching correctness before comparing times.

Run the independent exact-oracle comparisons with:

```bash
cargo test --test oracle_comparison_test
```

---

## Usage

### CLI

```bash
# Generate a test graph
cargo run -- generate -n 20 -o graph.json

# Maximum cardinality matching using Fast Blossom (default)
cargo run -- max-cardinality -i graph.json

# Experimental Gabow-inspired cardinality track
cargo run -- --algorithm gabow max-cardinality -i graph.json

# Experimental Micali–Vazirani research track (may return an error)
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

### Input validation

JSON graph deserialization and the fallible graph constructors reject edge endpoints outside `0..n`. DIMACS input supports comments, one optional `p edge <vertices> <edges>` record, and `e <u> <v>` or `e <u> <v> <weight>` records. When a problem record is present, its vertex range and declared edge count are checked. Malformed records, zero/out-of-range DIMACS vertices, and non-finite weights return line-numbered errors. Headerless edge lists remain supported and infer the vertex count.

### Benchmarks

```bash
# Run the quick CLI comparisons (general cardinality and weighted matching)
cargo run --bin bench

# Run statistically sampled Criterion comparisons
cargo bench --bench matching
```

The general-graph cardinality comparisons run `fast-blossom` and the
`gabow-heuristic` on identical graphs, validate each returned matching, and
require their cardinalities to agree before displaying timings. This agreement
is a comparison check, not an optimality proof; the dedicated small-graph
oracle tests provide independent correctness evidence. The Gabow-inspired
implementation is labeled as a heuristic and makes no $O(m\sqrt n)$ claim.
Experimental Micali–Vazirani remains outside end-to-end comparisons; only its
isolated level-search phase is benchmarked. Bipartite cardinality and weighted
perfect-matching comparisons remain separate because they measure different
problems. Criterion performs matching validity checks and requires weighted
solvers on each generated instance to agree on the reported optimum before it
starts timing the comparison. Correctness preflight failures are written to
stderr with the affected graph/solver and the corresponding timings are
skipped; a failure during a timed invocation aborts that sample rather than
being accepted as a timing result.

The configurable CLI runner uses every `weight_ranges` entry and executes
`runs_per_config` fresh graph instances for each size, density, and weight
range. All selected solvers share each instance. Configure `seed` to make the
generated sequence repeatable; each result record includes the seed, run,
requested and observed weight bounds, and a graph signature. Density is a
fraction in `[0, 1]`, and ranges use inclusive lower and exclusive upper
bounds.

### Interpreting solver counters

`MatchingMetadata` uses one shared output shape, not shared counter semantics.
Do not compare these counters across algorithms.
In particular, zero means either “not applicable” or “not instrumented,” not
necessarily that no such work occurred. Several adapters derive an
augmentation count from the returned perfect matching rather than measuring
internal augmentation operations. `runtime_ms` is the solver's own elapsed-time
measurement; use Criterion's reported measurements for benchmark comparisons.

| Solver | `iterations` | `augmentations` | Other counters |
|---|---|---|---|
| `fast-blossom` | Alternating-forest queue vertices processed | Successful augmenting paths | `blossoms_created` counts contraction events; expansion and dual-update counters are uninstrumented |
| `gabow-heuristic` | Outer heuristic search phases | Paths augmented, including fallback single-path augmentations | Blossom contractions counted; expansion and dual-update counters are uninstrumented |
| `micali-vazirani` | Solver phases | Paths augmented | Petals/blossoms created counted; expansion and dual-update counters are uninstrumented. The Criterion development group measures only level search and has no end-to-end solver counter comparison |
| `hopcroft-karp` | Adjacency entries examined by BFS and DFS | Successful shortest augmenting paths | Blossom and dual-update counters are not applicable |
| Hungarian | Candidate row/column entries examined by the assignment search | Rows inserted into the assignment, one augmentation attempt per left vertex | `dual_updates` counts potential-adjustment steps; blossom counters are not applicable |
| `primal-dual` | Not instrumented (`0`) | Reported perfect-matching cardinality, `n / 2`, not an internal counter | Blossom and dual-update counters are not exposed by the wrapped dependency |
| `blossom-tree` | Not instrumented (`0`) | Reported perfect-matching cardinality, `n / 2`, not an internal counter | Blossom and dual-update counters are not instrumented |
| `blossom-engine`, `cherry-tree` | Edge scans | Successful augmentations | Blossom creations/expansions and dual adjustments are counted by the implementation |
| `sparse-blossom` | Search-queue entries processed | Reported perfect-matching cardinality, `n / 2`, not an internal counter | Blossom creations/expansions and dual adjustments are counted; its augmentation counter is inferred |

---

## Algorithm Details

### General-graph maximum-cardinality solvers

- **`fast-blossom` — conventional Edmonds baseline.** Searches from exposed vertices, contracts odd cycles around a lowest common ancestor, and augments one path at a time. It is the simple general-graph comparator, not an MV or Gabow implementation. Literature: [Edmonds (1965)](https://doi.org/10.4153/CJM-1965-045-4); [Galil's survey (1986)](https://doi.org/10.1145/6462.6502).
- **`micali-vazirani` — experimental MV research track, not a production solver.** Alternating parity levels and tenacity-ordered bridges feed a DDFS/petal search; certified paths are unrolled and augmented in phases. Nested-petal behavior is covered by targeted regressions, but complete correctness has not been established. Exact recursive certificate searches are capped at 100,000 explored states per solve and report `SearchBudgetExceeded` if they reach the cap. This prevents unbounded fallback work but can reject otherwise solvable inputs; it does not establish completeness or the published $O(m\sqrt n)$ bound. The generic benchmark runner excludes this solver; Criterion measures only its explicitly labeled level-search phase. Literature: [Micali–Vazirani (1980)](https://doi.org/10.1109/SFCS.1980.12); [Vazirani's proof](https://arxiv.org/abs/2012.03582).
- **`gabow` — experimental Gabow-inspired track.** Uses a multi-root alternating search and DSU blossom contraction, then tries candidate augmenting bridges. When the phased search stalls, an Edmonds-style single-path search checks for a missed augmentation. This is **not** the algorithm of [Ansaripour, Danaei, and Mehlhorn (2024)](https://arxiv.org/abs/2409.14849): their phase one maintains dual/tight-edge information and builds an auxiliary graph $H$; phase two finds paths in $H$. No $O(m\sqrt n)$ bound is claimed for this Rust version.

### Bipartite solvers

- **`hopcroft-karp` — unweighted cardinality.** BFS layers free left-side vertices and DFS extracts a maximal set of shortest augmenting paths, giving the classical $O(m\sqrt n)$ bound. Literature: [Hopcroft–Karp (1973)](https://doi.org/10.1137/0202019).
- **Hungarian — weighted assignment.** Maintains row/column dual labels and minimum reduced-cost slack while augmenting a square bipartite assignment. This is a bipartite comparator, not a general-graph MWPM solver. Literature: [Kuhn (1955)](https://doi.org/10.1002/nav.3800020109).

### General-graph minimum-weight perfect matching solvers

All five are in-repository primal-dual blossom studies, with distinct state representations. Their measured behavior should be compared on the same graph families; a common framework does not imply identical complexity or robustness.

- **`primal-dual` — weighted baseline.** Uses edge-list Edmonds/Galil alternating forests, integer-scaled costs and exact `i128` dual updates. Background: [Galil (1986)](https://doi.org/10.1145/6462.6502).
- **`blossom-engine` — independent weighted state machine.** Makes blossom creation, dual adjustments and path lifting explicit for study; it solves the same weighted matching problem through a separate implementation. Background: [Edmonds (1965)](https://doi.org/10.1145/321250.321261); [Galil (1986)](https://doi.org/10.1145/6462.6502).
- **`blossom-tree` — flat-containment variant.** Tracks nested contractions with a flat history and maps the final mate pairs back to input edges. Its code lineage is the MIT-licensed `mwmatching` adaptation, not Blossom V. Background: [Galil (1986)](https://doi.org/10.1145/6462.6502).
- **`cherry-tree` — ordered blossom variant.** Represents nested odd cycles as cherries and recursively lifts paths through them. This is an educational representation of weighted primal-dual matching, not a claim to a distinct published complexity bound. Background: [Edmonds (1965)](https://doi.org/10.1145/321250.321261).
- **`sparse-blossom` — event-driven sparse experiment.** Uses incident-edge maps rather than a dense weight matrix, while retaining weighted blossom/dual operations. Its amortized bound is not proved here. Background: [Kolmogorov's Blossom V paper (2009)](https://pub.ista.ac.at/~vnk/papers/blossom5.pdf) for a contrasting mature weighted implementation.

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
