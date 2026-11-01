# Defect Analysis: Micali-Vazirani and Gabow Cardinality Matching

## Test Results Summary

### Gabow (gabow_cardinality_test.rs)
All 7 tests PASS. However, the phased approach (phase1+phase2) is largely
non-functional for paths longer than 1; correctness is salvaged by the
`augment_single_path` fallback.

### Micali-Vazirani (micali_vazirani_test.rs)
- 17 basic tests: PASS
- `matched_bridge_on_four_vertex_path_is_not_a_false_optimum`: PASS
- `nested_petals_are_unrolled_to_an_actual_augmenting_path`: PASS
- `mv_reports_uncertified_optimality_on_second_ten_vertex_regression`: PASS
  (but this test EXPECTS an error — the algorithm misses an augmenting path)
- `nested_petal_length_nine_augmenting_path_succeeds`: **HANGS** (infinite/exponential)
- `mv_agrees_with_fast_blossom_on_varied_graphs`: **HANGS** (same root cause)
- `mv_agrees_with_exhaustive_small_graph_oracle`: **HANGS** (same root cause)

---

## Defect 1 (Gabow): `trace_to_root` cannot trace non-root outer vertices

**File:** `src/algorithms/gabow_cardinality.rs`, function `trace_to_root`

**Root cause:** The function uses `self.p[curr]` to step backward from an outer
vertex. But `p` is only set for **inner** vertices during BFS (line: `self.p[v] = u`
where v is labeled inner). For outer vertices that are the *mate* of an inner
vertex, `p` is never set — it remains `usize::MAX` from initialization.

`trace_to_root` for a non-root outer vertex w (mate of inner v):
```
curr = w
mate[w] = v  (matched, so while-loop enters)
prev = p[w]  = MAX  → break
return [w]   // single vertex!
```

**Consequence:** In `phase2_augment`, `trace_to_root(u)` returns `[u]` for any
bridge endpoint u that is not an exposed root. The check
`self.mate[root_u].is_some()` then evaluates to true (u is matched), so the
bridge is skipped. Phase 2 can only augment length-1 paths (bridge between two
exposed roots). All longer paths fall through to the `augment_single_path`
fallback.

**Correct pattern** should be: outer → `mate[outer]` (matched edge to inner) →
`p[inner]` (unmatched edge to outer) → repeat. The function uses `p[curr]`
where it should use `mate[curr]` for the first step from an outer vertex.

## Defect 2 (Gabow): `trace_to_root` does not unroll blossoms

**File:** `src/algorithms/gabow_cardinality.rs`, function `trace_to_root`

When `mark_blossom` overwrites `p[v]` to the bridge endpoint, `trace_to_root`
follows that shortcut, skipping the blossom interior. The resulting path may
have two consecutive unmatched edges at the junction with the bridge, violating
the alternating property.

Example: Triangle 0-1-2-0, matching {0,1}, exposed {2,7}, bridge (7,0):
- `p[0]` is set to 2 (bridge endpoint) by `mark_blossom`
- `trace_to_root(0)` returns `[0, 2]` using direct unmatched edge (0,2)
- Full path `[2, 0, 7]` has edges (2,0) unmatched, (0,7) unmatched — NOT alternating
- `is_valid_augmenting_path` correctly rejects this
- Correct path should be `[0, 1, 2]` (matched 0=1, unmatched 1-2)

## Defect 3 (MV): Exponential backtracking causes hangs

**File:** `src/algorithms/micali_vazirani.rs`

Three functions use unbounded exponential backtracking:

1. **`find_path_to_root`**: Backtracks by unsetting `visited[p] = false` after
   failed recursion. Also scans ALL incident unmatched edges (not just
   predecessor props) as a "fallback", greatly increasing branching factor.

2. **`enumerate_half_paths`**: Backtracking DFS enumerating all simple alternating
   half-paths from a bridge endpoint. Called for each candidate bridge.

3. **`certified_petal_cycle`**: Tries EVERY vertex in the graph as a potential
   petal base (loop `for cand in 0..self.vertex_count`), each invoking
   `find_petal_cycle` — another exponential DFS.

The `nested_petal_length_nine` test triggers all three: a 10-vertex graph
requiring a length-9 augmenting path through nested blossoms. The certification
search never terminates in reasonable time.

## Defect 4 (MV): Matched bridges missed when odd_level not yet set

**File:** `src/algorithms/micali_vazirani.rs`, function `run_interleaved_search_phase`

In the odd-level scanning branch, when an odd vertex u follows its matched edge
to v, and `min_level[v] < next_lvl`:

```rust
if self.min_level[v].is_some_and(|level| level < next_lvl) {
    // Tries to queue as bridge, but bridge_tenacity returns None
    // because odd_level[v] is None
    if let Some(tenacity) = self.bridge_tenacity(graph, edge_idx) { ... }
    // Falls through — does NOT set odd_level[v]!
}
```

`bridge_tenacity` for a matched edge uses `odd_level` of both endpoints. If v
has `even_level` set (shorter path) but `odd_level` is None, `bridge_tenacity`
returns None. The code falls through without setting `odd_level[v] = next_lvl`.

The odd_level is never set, and the matched bridge is permanently missed in this
phase. This contributes to the `UncertifiedOptimality` error — the phased search
misses augmenting paths that the independent verifier then finds.

## Defect 5 (MV): Incomplete nested petal discovery (known/documented)

**File:** `src/algorithms/micali_vazirani.rs`

The test `mv_reports_uncertified_optimality_on_second_ten_vertex_regression`
EXPECTS `Err(UncertifiedOptimality { path_length: 9 })`. The phased search
declares completion (bottleneck = None) but an augmenting path of length 9
actually exists. The independent `shortest_augmenting_path` oracle catches this.

The code comments acknowledge this: "The phase's failure to find a bridge is not
yet a proof of optimality here: nested-petal discovery is incomplete."

---

## Verification

### Gabow defects confirmed by `tests/defect_demo_test.rs` (all 3 PASS):
- `gabow_phase2_fails_on_length3_path`: Phase 2 returns 0 for length-3 path
  (p[2]=MAX, trace_to_root(2)=[2], root 2 is matched → bridge skipped)
- `gabow_phase2_works_for_length1`: Phase 2 works for length-1 (bridge between roots)
- `gabow_trace_to_root_skips_blossom_interior`: Phase 2 returns 0 through blossom
  (traced path has two consecutive unmatched edges)

### MV defects confirmed by test execution:
- `nested_petal_length_nine_augmenting_path_succeeds`: **HANGS** (exponential backtracking)
- `mv_reports_uncertified_optimality_on_second_ten_vertex_regression`: PASSES but
  expects `Err(UncertifiedOptimality)` — algorithm misses a length-9 augmenting path
- All 17 basic MV tests: PASS
- `cargo check`: succeeds (code compiles correctly)

---

## Summary

| # | Algorithm | Severity | Type |
|---|-----------|----------|------|
| 1 | Gabow | High | trace_to_root can't trace non-root vertices |
| 2 | Gabow | High | trace_to_root doesn't unroll blossoms |
| 3 | MV | High | Exponential backtracking causes hangs |
| 4 | MV | Medium | Matched bridges missed (odd_level not set) |
| 5 | MV | Known | Incomplete nested petal discovery |
