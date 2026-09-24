//! Blossom Algorithm Study Library
//!
//! This library provides implementations of Edmonds' Blossom Algorithm
//! for maximum cardinality matching and minimum weight perfect matching.

pub mod algorithms;
pub mod benchmark;
pub mod graph;

pub use algorithms::{
    EdmondsAlgorithm, FastBlossomMatching, KorteVygenEdmonds, MatchingAlgorithm, MatchingMetadata,
    MatchingResult, minimum_weight_perfect_matching,
};
pub use graph::{Edge, Graph};
