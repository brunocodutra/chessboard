#![feature(
    array_chunks,
    const_refs_to_cell,
    const_trait_impl,
    effects,
    stdarch_x86_mm_shuffle
)]

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
