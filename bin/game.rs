use crate::{build::Build, player::Player};
use derive_more::{Constructor, Display, Error};
use lib::chess::{Color, Pgn, Position};
use lib::search::Limits;
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
        fields(white = %self.white, black = %self.black, %pos, %limits, outcome))]
    pub async fn play(
        self,
        mut pos: Position,
        limits: Limits,
    ) -> Result<Pgn, GameInterrupted<W::Error, B::Error>> {
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
                Color::White => white.play(&pos, limits).await.map_err(White)?,
                Color::Black => black.play(&pos, limits).await.map_err(Black)?,
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
    use crate::{build::MockBuilder, player::MockPlayer};
    use futures_util::FutureExt;
    use lib::chess::MoveKind;
    use proptest::sample::Selector;
    use std::future::ready;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn game_ends_when_it_is_over(#[filter(#pos.outcome().is_some())] pos: Position, l: Limits) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockPlayer::new();
        let b = MockPlayer::new();

        let mut wb = MockBuilder::<MockPlayer>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlayer>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        let outcome = pos.outcome().unwrap();

        assert_eq!(
            rt.block_on(g.play(pos, l)),
            Ok(Pgn {
                white: wc,
                black: bc,
                outcome,
                moves: vec![]
            })
        );
    }

    #[proptest]
    fn game_returns_pgn(pos: Position, l: Limits, selector: Selector) {
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

        let act = move |_: &Position, _: Limits| ready(Ok(moves.pop().unwrap())).boxed();

        w.expect_play().returning(act.clone());
        b.expect_play().returning(act);

        let mut wb = MockBuilder::<MockPlayer>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlayer>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        assert_eq!(
            rt.block_on(g.play(pos, l)),
            Ok(Pgn {
                white: wc,
                black: bc,
                outcome: o,
                moves: sans
            })
        );
    }

    #[proptest]
    fn game_interrupts_if_player_fails_to_build(pos: Position, l: Limits, e: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut wb = MockBuilder::<MockPlayer>::new();
        let bb = MockBuilder::<MockPlayer>::new();

        wb.expect_build().once().return_once({
            let e = e.clone();
            || Err(e)
        });

        let g = Game::new(wb, bb);
        assert_eq!(rt.block_on(g.play(pos, l)), Err(GameInterrupted::White(e)));
    }

    #[proptest]
    fn game_interrupts_if_player_fails_to_act(
        #[filter(#pos.outcome().is_none())] pos: Position,
        l: Limits,
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
        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(err)));

        let mut wb = MockBuilder::<MockPlayer>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlayer>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let g = Game::new(wb, bb);

        assert_eq!(
            rt.block_on(g.play(pos, l)),
            match turn {
                Color::White => Err(GameInterrupted::White(e)),
                Color::Black => Err(GameInterrupted::Black(e)),
            }
        );
    }
}
