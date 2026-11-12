# Per-solver time and memory profiles

`profile_solvers.py` generates deterministic, planted-perfect graphs and runs
one solver in each child process. It measures the child's wall time and peak
resident set size (RSS) with POSIX `wait4`. The worker also reports the
solver's own elapsed time. General graphs cover `fast-blossom`,
`gabow-heuristic`, `gabow-weighted-approach`, `mehlhorn-nobahari`, experimental
`micali-vazirani`, `primal-dual`, `blossom-engine`, `sparse-blossom`, and
`duan-scaling`. Bipartite graphs compare `hopcroft-karp` with `fast-blossom`
and Hungarian with `primal-dual`.

From the repository root, reproduce the saved profile with:

```bash
cargo build --release --offline --bin profile_worker --bin minimize_profile_failure
python3 bench/profile_solvers.py \
  --config bench/profiles/phase3-profile-config.json \
  --output /tmp/blossom-phase3-profile --timeout 10
python3 bench/minimize_profile_failure.py \
  --profiles /tmp/blossom-phase3-profile/profiles.json
```

The output directory must start empty. The config varies total graph size,
requested edge density, and integer edge-weight range, with two graph seeds
per cell. Every general graph includes the edges `(0,1), (2,3), ...`; every
bipartite graph includes a diagonal perfect matching. These planted edges
make observed density slightly higher than requested density, especially for
sparse small graphs. Topology is identical across weight ranges for a given
family, size, density, and seed. Ranges are `[minimum, maximum)`.

| File | Contents |
|---|---|
| `profiles.json` | One row per solver and graph, including graph SHA-256, result or error, solver time, whole-process wall time, and peak RSS in bytes |
| `profiles.csv` | Same rows for filtering and plotting |
| `summary.csv` | Median time and memory grouped by family, size, density, weight range, and solver; `ok_samples` identifies missing/failed samples |
| `manifest.json` | Source commit and dirty flag, toolchain, platform, binary and config hashes, timeout, and status counts |
| `failures/*.json` | Reproducible false-infeasibility graphs reduced while preserving a separately verified perfect matching |

Peak RSS includes the executable, graph parsing, solver, validation, and JSON
output. It measures a process peak, **not** allocations made solely by the
algorithm. `solver_ms` excludes process launch and file I/O;
`wall_ms` includes them and is dominated by launch overhead on tiny graphs.
The driver normalizes macOS `ru_maxrss` bytes and Linux `ru_maxrss` KiB to
bytes. A timeout is recorded rather than silently omitted. A solver error or
cross-solver disagreement makes the driver exit nonzero after saving all rows;
the experimental MV track is recorded but does not determine that exit code.

## Saved 2026-11-12 baseline

The [baseline directory](2026-11-12-baseline/) contains 312 measurements:
20, 40, and 80 total vertices for both general and bipartite families; 20%
and 100% requested density; `[1,100)` and `[-1000,1000)` weights; two runs per
cell. The profiler recorded **310 successful invocations and two
`blossom-engine` errors** on planted-perfect 80-vertex general graphs. The
other weighted solvers returned matching weights that agreed on those two
graphs. The [reduced failure graphs](2026-11-12-baseline/failures/) retain 45
and 55 edges, respectively, and remain feasible according to Fast Blossom.
These are current solver defects, so the profiling command returns a nonzero
status. The raw rows remain available for investigation; `summary.csv` leaves
failed samples out of medians and reports their reduced `ok_samples` count.

Two runs per cell and graphs of at most 80 vertices are enough to locate
trends and failures, but they do not support performance rankings or
asymptotic claims. Repeat on larger inputs and multiple machines before
drawing those conclusions. The published time bounds of the experimental MV,
Gabow-inspired heuristic, and simplified Duan scaling code are not established
by these measurements.
