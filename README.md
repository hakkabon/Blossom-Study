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
│       ├── mwpm.rs         # Dense-matrix weighted primal-dual baseline adapter
│       ├── duan_mwpm.rs    # Adapter for the imported scaling solver
│       ├── sparse_mwpm.rs  # Sparse event-driven experiment
│       └── korte_vygen.rs  # Reference-only implementation
├── tests/
│   ├── gabow_heuristic_test.rs # Gabow-inspired heuristic tests
│   ├── micali_vazirani_test.rs # Micali–Vazirani tests, nested petals, and oracle checks
│   ├── hopcroft_karp_test.rs # Bipartite cardinality/oracle tests
│   ├── hungarian_test.rs   # Bipartite assignment/oracle tests
│   ├── oracle_comparison_test.rs # Independent exact oracles for cardinality and weighted solvers
│   ├── weighted_regressions_test.rs # Retained weighted defect graphs
│   ├── matching.rs         # Integration and cross-implementation tests
│   └── data/               # Test graph fixtures
├── bench/
│   ├── main.rs             # Quick comparison runner
│   └── records/            # Saved reproducible smoke-run inputs and outputs
├── benches/
│   └── matching.rs         # Criterion benchmark groups
├── implementations/         # Focused source crates for imported algorithms
│   ├── dsu/                # Shared union-find implementation
│   ├── duan_mwpm/          # Scaling-based weighted solver
│   ├── gabow_matching/     # Gabow's 2017 phased cardinality solver
│   └── mn_matching/        # Mehlhorn–Nobahari variant
├── Cargo.toml
└── README.md
```

## Current Status

### Working Features
- 🧪 **Gabow-inspired cardinality** (`gabow`): phased alternating search with blossom contraction and an Edmonds-style single-path search when a phase stalls. It does **not** construct the paper's tight-edge auxiliary graph $H$ or establish its $O(m\sqrt n)$ bound. Regressions cover the eight-vertex false optimum and a path-tracing cycle after a batch augmentation.
- ⚠️ **Experimental Micali–Vazirani research track** (`micali-vazirani`): parity levels, bridge tenacity, DDFS proposals, and petal unrolling. Nested-petal discovery and the complete algorithm have not been independently established as complete. Recursive path and petal certificates share a fixed 100,000-state safety budget; exceeding it returns an error rather than risking an unbounded search or claiming a maximum. This is not a dependable general-purpose solver and the published MV time bound does **not** apply.
- ✅ **Conventional Blossom Cardinality** (`fast-blossom`): Edmonds blossom contraction with timestamp-based LCA and DSU.
- ✅ **Shared DSU**: fast blossom, the Gabow tracks, the MV petal search, and the retained Korte–Vygen reference use `implementations/dsu/`. Directed contraction keeps the requested blossom base; ordinary union reports its chosen root.
- ✅ **Gabow-family cardinality implementations**: `gabow-weighted-approach` builds a tight-edge auxiliary graph from an Edmonds search; `mehlhorn-nobahari` uses the revised shortest-path search. Their source is kept in separate, small modules under `implementations/`.
- ✅ **Hopcroft–Karp Bipartite Matching** (`hopcroft-karp`): $O(m \sqrt{n})$ multi-phase bipartite maximum cardinality matching.
- ✅ **General-graph Minimum-Weight Perfect Matching**:
  - `primal-dual`: Dense `n × n` weight-matrix adapter around `integer-blossom`, with exact `i128` cost scaling and transformed weights. Matrix construction and solver execution are both inside its reported runtime.
  - `blossom-engine`: Independent Galil/Edmonds primal-dual solver with explicit alternating forests and nested blossoms.
  - `sparse-blossom`: Sparse event-driven adjacency/map implementation.
  - `duan-scaling`: integer-weight scaling implementation of Duan–Pettie–Su's Liquidationist framework. It uses simpler search/event routines than the paper, so its published runtime bound does not apply.
- ✅ **Hungarian Bipartite Assignment**: $O(k^3)$ primal-dual shortest augmenting paths with exact cost scaling.
- ✅ **Graph Generators & Serialization**: Complete, sparse, grid, and planted matching generators with JSON serialization.
- ✅ **Independent exact test oracles**: registered general-cardinality solvers are checked on every simple graph through six vertices. The three general weighted solvers in the shared oracle suite are checked against dynamic programming on reproducible sparse instances up to ten vertices; Duan scaling has a separate exhaustive small-graph test. Retired solvers' defect graphs remain in `weighted_regressions_test.rs`. These finite tests provide regression coverage, not a proof of correctness on arbitrary graphs.
- ✅ **Larger deterministic differential suite**: seeded general, weighted, infeasible, and bipartite cases run through the comparable solvers up to 40 vertices. Smaller cases in this suite have independent subset-DP checks; larger cases check validity and cross-solver agreement. Failures produce edge-reduced, replayable JSON artifacts. See [test coverage and failure records](tests/DIFFERENTIAL.md).

### Performance interpretation

Use the benchmark commands below for measurements on your machine. The generic runner excludes experimental Micali–Vazirani. Criterion includes a separately named MV level-search-only development group, which is not an end-to-end solver benchmark. No MV timing constitutes a comparison with the published algorithm or evidence for its asymptotic bound. See [benchmark records](bench/records/README.md) for a saved, seeded smoke run and its measurement limits.

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

# Phased Gabow-family cardinality solvers
cargo run -- --algorithm gabow-weighted-approach max-cardinality -i graph.json
cargo run -- --algorithm mehlhorn-nobahari max-cardinality -i graph.json

# Experimental Micali–Vazirani research track (may return an error)
cargo run -- --algorithm micali-vazirani max-cardinality -i graph.json

# Minimum weight perfect matching with the established baseline
cargo run -- min-weight-perfect -i graph.json

# Independent in-repository Galil/Edmonds implementation
cargo run -- min-weight-perfect --solver blossom-engine -i graph.json

# Sparse event-driven experiment
cargo run -- min-weight-perfect --solver sparse-blossom -i graph.json

# Duan–Pettie–Su scaling study (exact integer conversion must fit i64)
cargo run -- min-weight-perfect --solver duan-scaling -i graph.json
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

The general-graph cardinality comparisons run `fast-blossom`,
`gabow-heuristic`, `gabow-weighted-approach`, and `mehlhorn-nobahari` on identical graphs, validate each returned matching, and
require their cardinalities to agree before displaying timings. This agreement
is a comparison check, not an optimality proof; the dedicated small-graph
oracle tests provide independent correctness evidence. The Gabow-inspired
implementation is labeled as a heuristic and makes no $O(m\sqrt n)$ claim.
The separate `duan-scaling` Criterion group uses small weighted graphs and
checks its answer against the weighted baseline before timing. The imported
scaling code intentionally omits the paper's fast search data structures;
benchmark its actual runtime rather than assuming the paper's bound.
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

The quick `--bin bench` runner creates random weighted complete and sparse
graphs without a recorded seed; its printed timings are exploratory. For a
repeatable input record, use the configurable CLI runner with a saved config.
Its JSON captures the seed, graph signature and each solver's internal runtime.
Those are single-call timings, not Criterion estimates. The committed
[smoke record](bench/records/2026-11-12-smoke/) documents the exact command,
toolchain and graph identities; do not use its small timings to rank solvers.

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
| `gabow-weighted-approach`, `mehlhorn-nobahari` | Number of phases returned by the imported crate | Reported matching cardinality, not an internal augmentation counter | Blossom and dual-update counters are not exposed by the adapters |
| `micali-vazirani` | Solver phases | Paths augmented | Petals/blossoms created counted; expansion and dual-update counters are uninstrumented. The Criterion development group measures only level search and has no end-to-end solver counter comparison |
| `hopcroft-karp` | Adjacency entries examined by BFS and DFS | Successful shortest augmenting paths | Blossom and dual-update counters are not applicable |
| Hungarian | Candidate row/column entries examined by the assignment search | Rows inserted into the assignment, one augmentation attempt per left vertex | `dual_updates` counts potential-adjustment steps; blossom counters are not applicable |
| `primal-dual` | Not instrumented (`0`) | Reported perfect-matching cardinality, `n / 2`, not an internal counter | Blossom and dual-update counters are not exposed by the wrapped dependency |
| `blossom-engine` | Edge scans | Successful augmentations | Blossom creations/expansions and dual adjustments are counted by the implementation |
| `sparse-blossom` | Search-queue entries processed | Reported perfect-matching cardinality, `n / 2`, not an internal counter | Blossom creations/expansions and dual adjustments are counted; its augmentation counter is inferred |
| `duan-scaling` | Number of scaling stages | Internal augmentation count reported by the imported crate | Contractions, dissolutions, and dual events are reported by the imported crate; they are not directly comparable with the other weighted counters |

---

## Algorithm Details

### General-graph maximum-cardinality solvers

- **`fast-blossom` — conventional Edmonds baseline.** Searches from exposed vertices, contracts odd cycles around a lowest common ancestor, and augments one path at a time. It is the simple general-graph comparator, not an MV or Gabow implementation. Literature: [Edmonds (1965)](https://doi.org/10.4153/CJM-1965-045-4); [Galil's survey (1986)](https://doi.org/10.1145/6462.6502).
- **`micali-vazirani` — experimental MV research track, not a production solver.** Alternating parity levels and tenacity-ordered bridges feed a DDFS/petal search; certified paths are unrolled and augmented in phases. Nested-petal behavior is covered by targeted regressions, but complete correctness has not been established. Exact recursive certificate searches are capped at 100,000 explored states per solve and report `SearchBudgetExceeded` if they reach the cap. This prevents unbounded fallback work but can reject otherwise solvable inputs; it does not establish completeness or the published $O(m\sqrt n)$ bound. The generic benchmark runner excludes this solver; Criterion measures only its explicitly labeled level-search phase. Literature: [Micali–Vazirani (1980)](https://doi.org/10.1109/SFCS.1980.12); [Vazirani's proof](https://arxiv.org/abs/2012.03582).
- **`gabow` — experimental Gabow-inspired track.** Uses a multi-root alternating search and DSU blossom contraction, then tries candidate augmenting bridges. When the phased search stalls, an Edmonds-style single-path search checks for a missed augmentation. This is **not** the algorithm of [Ansaripour, Danaei, and Mehlhorn (2024)](https://arxiv.org/abs/2409.14849): their phase one maintains dual/tight-edge information and builds an auxiliary graph $H$; phase two finds paths in $H$. No $O(m\sqrt n)$ bound is claimed for this Rust version.
- **`gabow-weighted-approach` — phased cardinality.** The first phase runs an Edmonds primal-dual search with cardinality-encoding weights and contracts positive blossoms into a tight-edge graph $H$. The second phase finds disjoint augmenting paths in $H$, lifts them, and augments. Source: `implementations/gabow_matching/`. Literature: [Gabow (2017)](https://arxiv.org/abs/1703.03998).
- **`mehlhorn-nobahari` — revised phased cardinality.** Replaces the first phase with the paper's direct shortest-augmenting-path search, then constructs $H$ and lifts a maximal path set. Source: `implementations/mn_matching/`. Literature: [Mehlhorn and Nobahari (2026)](https://arxiv.org/abs/2603.22909).

### Bipartite solvers

- **`hopcroft-karp` — unweighted cardinality.** BFS layers free left-side vertices and DFS extracts a maximal set of shortest augmenting paths, giving the classical $O(m\sqrt n)$ bound. Literature: [Hopcroft–Karp (1973)](https://doi.org/10.1137/0202019).
- **Hungarian — weighted assignment.** Maintains row/column dual labels and minimum reduced-cost slack while augmenting a square bipartite assignment. This is a bipartite comparator, not a general-graph MWPM solver. Literature: [Kuhn (1955)](https://doi.org/10.1002/nav.3800020109).

### General-graph minimum-weight perfect matching solvers

These are in-repository weighted matching studies with distinct state representations. Their measured behavior should be compared on the same graph families; a common framework does not imply identical complexity or robustness.

- **`primal-dual` — weighted baseline.** Filters the graph's edges, converts finite `f64` costs to exact scaled `i128` values when representable, then builds a dense symmetric matrix with a cardinality-dominating bonus for `integer-blossom::max_weight_matching`. It returns an error if scaling or the bonus overflows `i128`. The repository adapter does not expose the dependency's blossom or dual counters. Background: [Galil (1986)](https://doi.org/10.1145/6462.6502).
- **`blossom-engine` — independent weighted state machine.** Makes blossom creation, dual adjustments and path lifting explicit for study; it solves the same weighted matching problem through a separate implementation. Background: [Edmonds (1965)](https://doi.org/10.1145/321250.321261); [Galil (1986)](https://doi.org/10.1145/6462.6502).
- **`sparse-blossom` — event-driven sparse experiment.** Uses incident-edge maps rather than a dense weight matrix, while retaining weighted blossom/dual operations. Its amortized bound is not proved here. Background: [Kolmogorov's Blossom V paper (2009)](https://pub.ista.ac.at/~vnk/papers/blossom5.pdf) for a contrasting mature weighted implementation.
- **`duan-scaling` — Liquidationist integer scaling study.** Carries duals and inherited blossoms across bits, liquidates blossoms, and uses SearchOne before final perfection. The source preserves its BSD notice and implementation notes in `implementations/duan_mwpm/`. It omits the specialized data structures needed for the paper's $O(m\sqrt n\log(nN))$ bound. The adapter preserves exact costs only if a common binary scaling fits signed 64-bit integers; otherwise it reports an error. Literature: [Duan, Pettie, and Su (2017)](https://arxiv.org/abs/1411.1919).

The `duan-scaling` benchmark adapter disables the solver's expensive internal
invariant scans, then validates the final matching and checks its weight
against the baseline before timing. The imported crate's own tests exercise
the internal checks.

---

## References

- Ansaripour, M., Danaei, A., & Mehlhorn, K. (2024). *Gabow's Cardinality Matching Algorithm in General Graphs: Implementation and Experiments*. arXiv:2409.14849 [cs.DS].
- Gabow, H. N. (2017). *The Weighted Matching Approach to Maximum Cardinality Matching*. arXiv:1703.03998.
- Mehlhorn, K., & Nobahari, R. (2026). *Gabow's O(√nm) Maximum Cardinality Matching Algorithm, Revisited*. arXiv:2603.22909v2.
- Duan, R., Pettie, S., & Su, H.-H. (2017). *Scaling Algorithms for Weighted Matching in General Graphs*. arXiv:1411.1919v6.
- Gabow, H.N. (2017). *Data Structures for Weighted Matching and Extensions to b-matching*. arXiv:1611.07541.
- Micali, S., & Vazirani, V.V. (1980). An $O(\sqrt{|V|} \cdot |E|)$ algorithm for finding maximum matching in general graphs. *21st Annual Symposium on Foundations of Computer Science (FOCS 1980)*, 17–27.
- Hopcroft, J.E., & Karp, R.M. (1973). An $n^{5/2}$ algorithm for maximum matchings in bipartite graphs. *SIAM Journal on Computing*, 2(4), 225–231.
- Galil, Z. (1986). Efficient Algorithms for Finding Maximum Matching in Graphs. *ACM Computing Surveys*, 18(1), 23–38.
- Kuhn, H.W. (1955). The Hungarian method for the assignment problem. *Naval Research Logistics Quarterly*, 2, 83–97.
- Gerards, A.M.H. (1995). Matching. In *Network Models*, Handbooks in Operations Research and Management Science, Vol. 7, pp. 135-224. Elsevier.
- Kolmogorov, V. (2009). Blossom V: A new implementation of a minimum cost perfect matching algorithm. *Mathematical Programming Computation*, 1(1), 43-67.
