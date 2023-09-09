use futures_util::future::BoxFuture;
use lib::chess::{Move, Position};
use lib::search::Limits;

/// Trait for types that know how to analyze chess [`Position`]s.
pub trait Ai {
    /// Finds the best [`Move`].
    fn play<'a, 'b, 'c>(&'a mut self, pos: &'b Position, limits: Limits) -> BoxFuture<'c, Move>
    where
        'a: 'c,
        'b: 'c;
}
