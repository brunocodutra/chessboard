use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::search::{Limits, Pv};

/// Trait for types that know how to analyze chess [`Position`]s.
#[cfg_attr(test, mockall::automock(type Error = String;))]
pub trait Player {
    /// The reason why the [`Player`] was unable to continue.
    type Error;

    /// Finds the best [`Move`].
    fn play<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxFuture<'c, Result<Move, Self::Error>>
    where
        'a: 'c,
        'b: 'c;

    /// Analyzes a [`Position`].
    fn analyze<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxStream<'c, Result<Pv, Self::Error>>
    where
        'a: 'c,
        'b: 'c;
}
