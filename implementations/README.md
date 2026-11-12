# Imported matching implementations

Each solver is a small local Rust crate. Its algorithm modules, graph state,
and focused source tests stay together. The public Blossom-Study adapters live
in `src/algorithms/imported_cardinality.rs` and `src/algorithms/duan_mwpm.rs`.

| Folder | Problem | Main entry point |
|---|---|---|
| `dsu/` | Shared disjoint-set union | `UnionFind` |
| `gabow_matching/` | General-graph maximum cardinality | `maximum_matching` |
| `mn_matching/` | General-graph maximum cardinality | `maximum_matching` |
| `duan_mwpm/` | Integer-weight perfect matching | `maximum_weight_perfect_matching` |

The cardinality solvers use the same DSU crate for blossom contraction.
`union_root` reports the representative chosen by rank, while `union_into`
preserves a specified blossom base. These operations cannot be interchanged.

The Duan crate follows the scaling framework but uses straightforward search
and event scans. Its own README describes the gap from the paper's published
running-time bound and retains the source attribution and license notice.
