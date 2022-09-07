use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::search::Pv;

/// Trait for types that know how to analyze chess [`Position`]s.
pub trait Player {
    /// The reason why the [`Player`] was unable to continue.
    type Error;

    /// Finds the best [`Move`].
    fn play<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
    ) -> BoxFuture<'c, Result<Move, Self::Error>>
    where
        'a: 'c,
        'b: 'c;

    /// Analyzes a [`Position`].
    fn analyze<'a, 'b, 'c, const N: usize>(
        &'a mut self,
        pos: &'b Position,
    ) -> BoxStream<'c, Result<Pv<N>, Self::Error>>
    where
        'a: 'c,
        'b: 'c;
}
