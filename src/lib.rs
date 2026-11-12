//! Blossom Algorithm Study Library
//!
//! This library provides implementations of Edmonds' Blossom Algorithm
//! for maximum cardinality matching and minimum weight perfect matching.

pub mod algorithms;
pub mod benchmark;
pub mod graph;

pub use algorithms::{
    BlossomEngine, BlossomTreeSolver, CherryTreeSolver, DdfsResult, FastBlossomMatching,
    GabowCardinality, GabowHeuristic, HopcroftKarpSolver, HungarianSolver, MatchingMetadata,
    MatchingResult, MicaliVazirani, MicaliVaziraniError, MicaliVaziraniPhase, TenacityQueue,
    blossom_engine_minimum_weight_perfect_matching, blossom_tree_minimum_weight_perfect_matching,
    cherry_tree_minimum_weight_perfect_matching, duan_minimum_weight_perfect_matching,
    gabow_heuristic_matching, gabow_maximum_cardinality_matching,
    hopcroft_karp_maximum_cardinality_matching, hungarian_minimum_weight_perfect_matching,
    minimum_weight_perfect_matching, mn_maximum_cardinality_matching,
    sparse_minimum_weight_perfect_matching,
};
pub use graph::{Edge, Graph};
