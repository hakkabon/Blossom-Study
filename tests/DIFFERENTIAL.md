# Larger deterministic differential tests

Run `cargo test --offline --test differential_large_test`. The two fixed seeds
are `0x5eed_2026_0000_0001` and `0x5eed_2026_0000_0002`; graph generation is
defined in `differential_large_test.rs` and uses no thread-local randomness.

| Category | Graphs | Checks |
|---|---:|---|
| General cardinality | 22: 12, 18, 20, 32, and 40 vertices at 15% and 65% edge probability, plus odd-order 21-vertex cases | All four registered solvers return valid matchings and equal cardinalities. The 12- and 18-vertex cases also use independent subset DP. |
| General weighted, feasible | 16: 12, 18, 24, and 32 vertices at 18% and 65%, with a planted perfect matching | Four weighted solvers return valid perfect matchings with equal cost. The 12- and 18-vertex cases also use independent subset DP. |
| General weighted, infeasible | 4: 18- and 30-vertex graphs split into two odd components, each containing a planted odd cycle | All four weighted solvers reject infeasibility; the 18-vertex cases also use subset DP. |
| Bipartite | 8: 12+12 and 20+20 vertices at 20% and 70%, with a planted perfect matching | The general solvers agree; Hopcroft–Karp agrees on cardinality and Hungarian agrees on minimum cost. |

The graph generator adds the planted edges before applying the edge
probability to other pairs, so the percentages are approximate for small
graphs. The larger cases use differential agreement and matching validation,
not an independent proof of optimality. They also do not establish any
solver's asymptotic running time.

## Failure records

On a mismatch, invalid result, unexpected rejection, or panic, the test
removes edges while preserving the same failure code and writes a JSON graph
under `target/differential-failures/<suite>-<case>.json` before failing. The
artifact includes the seed, case name, original and reduced edge counts,
failure details, and a graph loadable by `Graph::from_json_file`. Reduction
keeps the vertex count and edge weights fixed and stops when no single
remaining edge can be removed while preserving that code. This is
**one-edge-minimal**, not a claim of global minimality. Reducing a failure can
be expensive; it runs only after a failure.

To retain a discovery, copy the generated JSON into `tests/data/`, add a
focused regression test with the expected matching property, and include the
solver and original case seed in the test comment. `target/` is ignored by Git,
so an unpromoted failure record is local. The current deterministic run found
no unresolved failures; the recorder itself has a synthetic round-trip test.
