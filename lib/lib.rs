#![feature(const_maybe_uninit_write, const_mut_refs, const_transmute_copy)]

/// Chess domain types.
pub mod chess;
/// Neural network for position evaluation.
pub mod nnue;
/// Minimax searching algorithm.
pub mod search;
/// Assorted utilities.
pub mod util;
