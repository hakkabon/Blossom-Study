# Historical cardinality defects and current status

This note records defects found during the Micali–Vazirani (MV) and
Gabow-inspired heuristic experiments. The earlier version of this file
described several tests as hanging or expecting errors. Those observations
predate the repairs and are **not** current test results. Run `cargo test
--offline` for the current suite.

| Earlier defect | Repair and permanent regression |
|---|---|
| The Gabow-inspired path trace read the parent of an outer vertex, although the alternating-tree parent was stored on its matched inner partner. Length-three candidate paths were lost. | `trace_to_root` now follows outer → matched inner → inner's parent. `gabow_phase2_augments_length3_path` covers this. |
| A blossom shortcut could produce a nonalternating path. | The trace follows the original tree through the blossom interior, and the completed path is checked before augmentation. `gabow_trace_to_root_lifts_blossom_interior` covers this case. |
| After one augmentation in a batch, a later candidate reused old BFS parents with the new matching and could cycle forever through a newly matched root. | The trace rejects vertices committed earlier in the batch and detects repeated vertices. `gabow_skips_paths_through_roots_matched_earlier_in_the_phase` preserves the 12-vertex case. |
| MV's recursive path and petal searches could take unbounded time on nested petals. | The recursive certificate search now shares a 100,000-state budget per solve and returns `SearchBudgetExceeded` on exhaustion. The length-nine nested-petal and varied-graph tests now complete. |
| MV could miss a matched bridge and report a false optimum. | Matched-bridge discovery and independent augmenting-path certification were added. The four-vertex bridge, two ten-vertex nested-petal graphs, and exhaustive small-graph oracle remain as regressions. |

## Remaining research limits

The MV implementation is an experimental, partially phased solver. Its finite
regressions and independent path certificates do not establish completeness or
the published running-time bound. Budget exhaustion is an explicit error, not
a maximum matching. It remains outside end-to-end benchmark comparisons.

The Gabow-inspired heuristic is distinct from the two imported Gabow-family
implementations. Its fallback can find paths missed by its phased search, but
the fallback's completeness has not been proved for this code. Its phase
structure also does not establish the paper's time bound. The imported
`gabow-weighted-approach` and `mehlhorn-nobahari` solvers are separate entries
in the general-cardinality comparison registry.

The tests above are in `tests/gabow_heuristic_phase_regressions_test.rs`,
`tests/gabow_heuristic_test.rs`, `tests/micali_vazirani_test.rs`, and
`tests/oracle_comparison_test.rs`.
