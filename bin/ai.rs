use chess::{Move, Position};
use futures_util::{future::BoxFuture, stream::BoxStream};
use search::{Limits, Pv};

/// Trait for types that know how to analyze chess [`Position`]s.
pub trait Ai {
    /// Finds the best [`Move`].
    fn play<'a, 'b, 'c>(&'a mut self, pos: &'b Position, limits: Limits) -> BoxFuture<'c, Move>
    where
        'a: 'c,
        'b: 'c;

    /// Analyzes a [`Position`].
    fn analyze<'a, 'b, 'c>(&'a mut self, pos: &'b Position, limits: Limits) -> BoxStream<'c, Pv>
    where
        'a: 'c,
        'b: 'c;
}
