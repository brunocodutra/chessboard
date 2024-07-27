#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_mm_shuffle))]
#![feature(array_chunks, coverage_attribute, new_uninit, optimize_attribute)]

/// Chess domain types.
pub mod chess;
/// Neural network for position evaluation.
pub mod nnue;
/// Minimax searching algorithm.
pub mod search;
/// UCI protocol.
pub mod uci;
/// Assorted utilities.
pub mod util;
