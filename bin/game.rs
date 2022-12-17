use crate::{build::Build, player::Player};
use derive_more::{Constructor, Display, Error};
use lib::chess::{Color, Pgn, Position};
use std::fmt::Display;
use tracing::{field::display, instrument, warn, Span};

/// The reason why the [`Game`] was interrupted.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "the {} player encountered an error")]
pub enum GameInterrupted<W, B> {
    #[display(fmt = "white")]
    White(W),

    #[display(fmt = "black")]
    Black(B),
}

/// Holds the state of a game of chess.
#[derive(Debug, Constructor)]
pub struct Game<W, B> {
    white: W,
    black: B,
}

impl<W, B> Game<W, B>
where
    W: Build + Display,
    B: Build + Display,
    W::Output: Player<Error = W::Error>,
    B::Output: Player<Error = B::Error>,
{
    /// Play a game of chess from the given starting [`Position`].
    #[instrument(level = "debug", skip(self), err,
        fields(white = %self.white, black = %self.black, %pos, outcome))]
    pub async fn play(self, mut pos: Position) -> Result<Pgn, GameInterrupted<W::Error, B::Error>> {
        use GameInterrupted::*;

        let white_config = self.white.to_string();
        let black_config = self.black.to_string();

        let mut white = self.white.build().map_err(White)?;
        let mut black = self.black.build().map_err(Black)?;

        let mut moves = Vec::new();

        let outcome = loop {
            if let Some(o) = pos.outcome() {
                Span::current().record("outcome", display(o));
                break o;
            }

            let m = match pos.turn() {
                Color::White => white.play(&pos).await.map_err(White)?,
                Color::Black => black.play(&pos).await.map_err(Black)?,
            };

            match pos.make(m) {
                Err(e) => warn!("{:?}", e),
                Ok(san) => moves.push(san),
            }
        };

        Ok(Pgn {
            white: white_config,
            black: black_config,
            outcome,
            moves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryStreamExt};
    use lib::chess::{Move, MoveKind};
    use lib::search::Pv;
    use mockall::mock;
    use proptest::sample::Selector;
    use std::fmt;
    use std::future::ready;
    use test_strategy::proptest;
    use tokio::runtime;

    mock! {
        #[derive(Debug)]
        Player {
            fn play<'a, 'b, 'c>(
                &'a mut self,
                pos: &'b Position,
            ) -> BoxFuture<'c, Result<Move, String>>
            where
                'a: 'c,
                'b: 'c;

            fn analyze<'a, 'b, 'c>(
                &'a mut self,
                pos: &'b Position,
            ) -> BoxStream<'c, Result<Pv<4>, String>>
            where
                'a: 'c,
                'b: 'c;
        }
    }

    impl Player for MockPlayer {
        type Error = String;

        fn play<'a, 'b, 'c>(
            &'a mut self,
            pos: &'b Position,
        ) -> BoxFuture<'c, Result<Move, Self::Error>>
        where
            'a: 'c,
            'b: 'c,
        {
            MockPlayer::play(self, pos)
        }

        fn analyze<'a, 'b, 'c, const N: usize>(
            &'a mut self,
            pos: &'b Position,
        ) -> BoxStream<'c, Result<Pv<N>, Self::Error>>
        where
            'a: 'c,
            'b: 'c,
        {
            MockPlayer::analyze(self, pos)
                .map_ok(|pv| pv.truncate())
                .boxed()
        }
    }

    mock! {
        #[derive(Debug)]
        PlayerBuilder {}
        impl Build for PlayerBuilder {
            type Output = MockPlayer;
            type Error = String;
            fn build(self) -> Result<MockPlayer, String>;
        }
    }

    impl fmt::Display for MockPlayerBuilder {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Debug::fmt(&self, f)
        }
    }

    #[proptest]
    fn game_ends_when_it_is_over(
        #[by_ref]
        #[filter(#pos.outcome().is_some())]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockPlayer::new();
        let b = MockPlayer::new();

        let mut wb = MockPlayerBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockPlayerBuilder::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        let outcome = pos.outcome().unwrap();

        assert_eq!(
            rt.block_on(g.play(pos)),
            Ok(Pgn {
                white: wc,
                black: bc,
                outcome,
                moves: vec![]
            })
        );
    }

    #[proptest]
    fn game_returns_pgn(pos: Position, selector: Selector) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut next = pos.clone();
        let mut sans = Vec::new();
        let mut moves = Vec::new();

        let o = loop {
            match next.outcome() {
                Some(o) => break o,
                _ => {
                    let (m, _) = selector.select(next.moves(MoveKind::ANY));
                    moves.push(m);
                    sans.push(next.make(m)?);
                }
            }
        };

        moves.reverse();

        let mut w = MockPlayer::new();
        let mut b = MockPlayer::new();

        let act = move |_: &Position| ready(Ok(moves.pop().unwrap())).boxed();

        w.expect_play().returning(act.clone());
        b.expect_play().returning(act);

        let mut wb = MockPlayerBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockPlayerBuilder::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        assert_eq!(
            rt.block_on(g.play(pos)),
            Ok(Pgn {
                white: wc,
                black: bc,
                outcome: o,
                moves: sans
            })
        );
    }

    #[proptest]
    fn game_interrupts_if_player_fails_to_build(pos: Position, e: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut wb = MockPlayerBuilder::new();
        let bb = MockPlayerBuilder::new();

        wb.expect_build().once().return_once({
            let e = e.clone();
            || Err(e)
        });

        let g = Game::new(wb, bb);
        assert_eq!(rt.block_on(g.play(pos)), Err(GameInterrupted::White(e)));
    }

    #[proptest]
    fn game_interrupts_if_player_fails_to_act(
        #[by_ref]
        #[filter(#pos.outcome().is_none())]
        pos: Position,
        e: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = pos.turn();

        let mut w = MockPlayer::new();
        let mut b = MockPlayer::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        let err = Err(e.clone());
        p.expect_play().return_once(move |_| Box::pin(ready(err)));

        let mut wb = MockPlayerBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockPlayerBuilder::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let g = Game::new(wb, bb);

        assert_eq!(
            rt.block_on(g.play(pos)),
            match turn {
                Color::White => Err(GameInterrupted::White(e)),
                Color::Black => Err(GameInterrupted::Black(e)),
            }
        );
    }
}
