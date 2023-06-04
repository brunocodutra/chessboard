use futures_util::stream::BoxStream;
use lib::chess::Position;
use lib::search::{Limits, Pv};

/// Trait for types that know how to analyze chess [`Position`]s.
#[cfg_attr(test, mockall::automock(type Error = String;))]
pub trait Analyze {
    /// The reason why the analysis failed.
    type Error;

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
