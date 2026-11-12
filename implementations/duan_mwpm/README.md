# wmatch: Duan–Pettie–Su scaling reference implementation in Rust

This is the Blossom-Study copy of the source. The production modules and their
small exhaustive unit tests are present; the original repository's standalone
file, optional NetworkX harness, and vendored `mwmatching` differential test
dependency are intentionally not copied. The study's
`tests/duan_integration_test.rs` provides an independent small-graph oracle.
Some validation counts below describe the original source repository, not a
new validation run in Blossom-Study.

This crate implements the **Liquidationist scaling framework** from §3,
Figure 5 of:

> Ran Duan, Seth Pettie, and Hsin-Hao Su. *Scaling Algorithms for Weighted
> Matching in General Graphs*. ACM Transactions on Algorithms 14(1),
> Article 8, January 2018. DOI: https://doi.org/10.1145/3155301
> Full algorithm reference: https://arxiv.org/pdf/1411.1919v6
> Journal manuscript: https://par.nsf.gov/servlets/purl/10059651

**Scope:** a correctness-oriented reference implementation, NOT an optimized
implementation of the paper's O(m sqrt(n) log(nN)) running time. It is not a
conventional exact Edmonds solver wrapped in bit-scaling calls: inherited
duals and blossoms actually pass through liquidation, reweighting, height
reduction, SearchOne, and pendant perfection at every scale. The Hybrid
algorithm in §4 is not implemented.

This copy has no runtime or test dependencies and uses Rust edition 2024.
See NOTICE and LICENSE.

## Build and run

```sh
cargo build --manifest-path implementations/duan_mwpm/Cargo.toml --release --offline
cargo test --manifest-path implementations/duan_mwpm/Cargo.toml --release --offline
```

Run these commands from the Blossom-Study repository root.

## Library usage

```rust
use duan_mwpm::{maximum_weight_matching, maximum_weight_perfect_matching, Edge};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let edges = vec![
        Edge::new(0, 1, 8), Edge::new(1, 2, 10), Edge::new(2, 0, 9),
        Edge::new(2, 3, 7), Edge::new(3, 4, 6), Edge::new(4, 5, 11),
    ];
    let matching = maximum_weight_matching(6, &edges)?;
    assert_eq!(matching.total_weight, 26);
    for (u, v) in matching.pairs() {
        println!("{u} -- {v}");
    }
    let perfect = maximum_weight_perfect_matching(6, &edges)?;
    assert_eq!(perfect.cardinality(), 3);
    Ok(())
}
```

When used from another local project, add:

```toml
[dependencies]
duan_mwpm = { path = "../Blossom-Study/implementations/duan_mwpm" }
```

### API semantics

- Vertex IDs are zero-based, in `0..n`.
- `Edge::new(u, v, weight)` accepts signed `i64` weights.
- `Matching.total_weight` and all internal dual / weight arithmetic use `i128`.
- `maximum_weight_matching` maximizes weight **without** a cardinality
  constraint. An empty matching is allowed; negative edges need not be used.
- `maximum_weight_perfect_matching` requires all vertices to be matched and
  returns `Error::NoPerfectMatching` when infeasible. It is NOT an API for
  maximum-weight maximum-cardinality matching in an infeasible graph.
- Empty graphs, isolated vertices, disconnected graphs, zero weights, signed
  weights, ties, and odd-order graphs are handled according to these semantics.
- Parallel edges are collapsed to the heaviest edge. Self-loops and out-of-range
  endpoints are rejected. Ties do not imply any particular mate ordering.
- Invariant checking is enabled by default, including in release mode. Use
  `*_with_options` with `Options { tau: None, check_invariants: false }` to
  disable expensive checks. Basic internal consistency assertions remain.
- The default `tau` is ceil(sqrt(number of vertices in the initial MWPM
  graph)), i.e. ceil(sqrt(2n)) for the unconstrained matching API. A custom
  positive threshold can be set in `Options.tau`.
- As with most reference implementations, allocation failure and an internal
  invariant failure are not converted to recoverable graph-input errors.

## Implementation map to the paper

| Module | Paper operation |
|---|---|
| `src/graph.rs` | Working graph and even integer weights, §2 |
| `src/blossom.rs` | Laminar odd-cycle forest, base rotation, path lifting, dissolution, §2 |
| `src/dual.rs` | Properties 1/2, dual adjustments from Figure 2, event computation |
| `src/search.rs` | EdmondsSearch forest, contraction, augmentation; Criteria 1/2/3, §2.2 |
| `src/pq_search.rs` | Event-jumping replacement for the optimized PQSearch of §5 |
| `src/liquidation.rs` | Liquidate and Figure 5 steps 2–6, §3 |
| `src/search_one.rs` | SearchOne, Figure 3, and its Criterion 2 usage, Lemmas 2.3/2.5 |
| `src/scaling.rs` | Complete Figure 5 scale loop, perfection, finalization |
| `src/lib.rs` | Validated API, MWM-to-MWPM reduction, signed-weight normalization entry |

### One scale

1. Reset the matching while snapshotting the old blossom family.
2. Expose the next most-significant bit: `w(e) = 2*(old_w(e) + bit(e))`;
   initialize `y(v) = 2*old_y(v) + 3` and double inherited blossom duals.
3. Liquidate large inherited blossoms, adding `z(B)/2` to **every leaf**.
4. Reweight edges by `w(u,v) -= y(u)+y(v)`, then clear vertex duals.
5. Liquidate small inherited blossoms. For each maximal old small blossom,
   lower free-vertex height levels using exact-tight searches (Criterion 1).
6. Perform `tau` SearchOne calls under Criterion 2, each with one unit dual
   adjustment. Greedy exhaustive eligible searches realize the maximal
   disjoint augmentation step; unlike the paper's DFS/union-find routine,
   the forest is rebuilt for each augmentation.
7. Delete free old pendants; match each remaining free original vertex to a
   fresh zero-weight pendant with dual `tau`.

After all bits, remove **all** pendants and finish under Criterion 3.
Free roots are selected from one dual-parity class at a time to satisfy the
EdmondsSearch precondition (§2.2); a path can end at any other free vertex.
Retained pendants can otherwise expose mixed parities when deleted.

### Eligibility and invariants

Define `yz(u,v) = y(u)+y(v)+sum(z(B): u,v in B)`.

- Criterion 1: `yz(e) == w(e)`.
- Criterion 2: stored blossom-cycle edges remain usable; outside contracted
  blossoms, matched edges need `yz(e) == w(e)`, while unmatched edges need
  `yz(e) == w(e)-2`. In particular, a matched edge need NOT be eligible. The
  search supports an inner vertex whose mate cannot yet be traversed.
- Criterion 3: `yz(e) == w(e)` OR `yz(e) == w(e)-2`.

Properties checked at stable states are integral `y`, even nonnegative `z`,
positive-dual root blossoms, odd/full blossoms, `yz(e) >= w(e)-2` for all
edges, and `yz(e) <= w(e)` on matched and stored blossom-cycle edges. During
Criterion 1 searches, the stronger lower bound `yz(e) >= w(e)` is checked.
Matching symmetry and accumulated reweighting identities are also checked.
Temporary zero-dual root blossoms are dissolved before a search restarts.
A nested zero-dual blossom can remain until it becomes a root, as permitted
by Property 1.

### Why additive slack still gives an exact answer

This is the argument behind §2.1, Lemma 2.2, not a formal verification of the
Rust implementation:

1. Normalize signed weights only on the perfect-matching working graph.
   Subtracting the minimum (if negative) preserves ordering because every
   perfect matching has exactly n/2 edges.
2. Multiply normalized original weights by `(n/2 + 1)`. After all bits,
   unreweighted weights are `2*(n/2 + 1)*normalized_weight`.
3. For a perfect matching M satisfying Property 2, the dual objective D is
   at most w(M), by near-tightness and blossom fullness. Adding 1 to every
   vertex dual gives a feasible dual with objective D+n. Thus
   `w(OPT) <= D+n <= w(M)+n`.
4. Reweighting subtracts the same sum of vertex potentials from EVERY
   perfect matching, so it cannot change their relative weights.
5. A one-unit difference in original integer weight would change scaled
   weight by `n+2`, strictly more than the possible error n. Therefore the
   final answer is exact in the original integer objective.

The implementation retains an explicit accumulated vertex potential and
checks the final reweighting identity on every original edge.

### MWM-to-MWPM reduction is NOT pendant perfection

For unconstrained maximum weight matching, construct two copies of the graph
and a zero-weight edge `v--v'` for every vertex. This adds n vertices and n
cross edges: the reduced graph has 2n vertices and 2m+n edges before any
per-scale pendants. Every matching can be lifted twice to a perfect matching,
using cross edges for its unmatched vertices. Conversely, every reduced
perfect matching projects to two matchings, each of weight at most OPT.
The reduced optimum is exactly 2*OPT and either projection is optimal.
The implementation returns the first projection.

The per-scale degree-one pendant vertices in Figure 5 are an entirely
separate temporary mechanism. They are all removed before the reduced
perfect-matching solver returns.

## Important complexity deviations

The published O(m sqrt(n) log(nN)) bound **does not apply** to this code.

- `SearchOne` uses repeated full searches rather than the linear-time
  DFS/specialized union-find maximal-disjoint-path routine.
- `PQSearch` scans the whole edge set to find the next event, jumps over
  unchanged unit adjustments, and rebuilds the complete alternating forest.
  It does not implement §5's priority queue, split-findmin, or advanced
  least-slack-edge maintenance.
- Blossom leaves and some ancestry information are materialized explicitly;
  invariant verification repeatedly traverses those structures.
- Deleted pendants are marked inactive rather than compacted from the graph.
- Blossom path lifting is recursive, so extremely deep nesting can exhaust
  the native stack. This is not a hardened large-instance production solver.

These choices make the data flow inspectable and the invariants executable,
but can substantially increase runtime and memory. No asymptotic runtime
bound for this simplified implementation is claimed. Disabling invariant
checks does NOT restore the paper's bound.

## Validation reported by the original source repository

With release-mode Rust and all invariant checks enabled:

- 6,000 deterministic randomized graphs (0–10 vertices) compared against a
  separate exhaustive matching enumerator, for BOTH APIs. Includes multiple
  blossom-size thresholds, sparse/dense inputs, signed weights, and ties.
- 3,000 independent randomized graphs (0–48 vertices) compared against the
  vendored plain Edmonds/Galil implementation, for BOTH APIs.
- 5 classic nested/relabel/expansion regression graphs, each with 5 thresholds.
  These tests actually exercised 32 nested contractions in the scaling code.
- Independent Edmonds randomized-suite coverage counters: 11,439 nested
  contractions, 1,556 large liquidations, 24,984 small liquidations, 9,427
  dissolutions (counters from the unconstrained API runs).
- A third, optional NetworkX 3.7 oracle compared BOTH APIs on 402 graphs,
  including graphs up to 80 vertices, 50-bit weights, and i64 extrema.
- Explicit tests for parallel edges, invalid endpoints, self-loops, zero tau,
  empty/infeasible graphs, and signed 64-bit extrema.

These differential tests are evidence, not a proof that every possible
implementation bug is absent. The original repository retained raw logs in
`validation/`; that directory is not part of this copy.
In the original source repository, the vendored plain solver was a test
oracle only and was not invoked by the production scaling implementation.

## Extending toward the published time bound

Replace `SearchOne` with the paper's linear-time maximal-disjoint-path
routine, and replace the event scans / forest rebuilding in `PQSearch` with
the §5 data structures and their stated guarantees. Merely adding a heap to
the current scans is not sufficient. Keep the differential tests and dual
invariant checks when replacing these subroutines.

