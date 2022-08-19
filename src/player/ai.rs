use crate::{Move, Play, Position, Search};
use async_trait::async_trait;
use derive_more::{Constructor, From};
use std::convert::Infallible;
use tokio::task::block_in_place;
use tracing::instrument;

/// A computed controlled player.
#[derive(Debug, From, Constructor)]
pub struct Ai<S: Search> {
    strategy: S,
}

#[async_trait]
impl<S: Search + Send> Play for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos))]
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        let best = block_in_place(|| Some(self.strategy.search(pos).next()?.best()));
        Ok(best.expect("expected non-terminal position"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use proptest::prop_assume;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn searches_for_best_move(mut s: Strategy, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let t = s.search(&pos).next();
        prop_assume!(t.is_some());

        let best = t.unwrap().best();
        let mut ai = Ai::new(s);
        assert_eq!(rt.block_on(ai.play(&pos))?, best);
    }

    #[proptest]
    #[should_panic]
    fn panics_if_there_are_no_moves(
        s: Strategy,
        #[by_ref]
        #[filter(#pos.moves().len() == 0)]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        rt.block_on(Ai::new(s).play(&pos))?;
    }
}
