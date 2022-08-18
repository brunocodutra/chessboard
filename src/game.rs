use crate::{Act, Action, Build, Color, Outcome, Pgn, Position, San};
use anyhow::Context;
use derive_more::{Constructor, Display, Error};
use std::fmt::Display;
use tracing::{field::display, instrument, warn, Span};

/// The reason why the [`Game`] was interrupted.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "the {} player encountered an error")]
pub enum GameInterrupted<W, B> {
    #[display(fmt = "white")]
    White(W),

    #[display(fmt = "black")]
    Black(B),
}

/// Holds the state of a game of chess.
#[derive(Debug, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Game<W, B> {
    white: W,
    black: B,
}

impl<W, B> Game<W, B>
where
    W: Build + Display,
    B: Build + Display,
    W::Output: Act<Error = W::Error>,
    B::Output: Act<Error = B::Error>,
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
            if let Some(o) = is_game_over(&pos) {
                Span::current().record("outcome", display(o));
                break o;
            }

            let action = match pos.turn() {
                Color::White => white.act(&pos).await.map_err(White)?,
                Color::Black => black.act(&pos).await.map_err(Black)?,
            };

            match action {
                Action::Move(m) => match pos.make(m).context("illegal player action") {
                    Err(e) => warn!("{:?}", e),
                    Ok(san) => {
                        moves.push(san);
                    }
                },

                Action::Resign => {
                    moves.push(San::null());
                    break Outcome::Resignation(pos.turn());
                }
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

fn is_game_over(pos: &Position) -> Option<Outcome> {
    if pos.is_checkmate() {
        Some(Outcome::Checkmate(!pos.turn()))
    } else if pos.is_stalemate() {
        Some(Outcome::Stalemate)
    } else if pos.is_material_insufficient() {
        Some(Outcome::DrawByInsufficientMaterial)
    } else if pos.halfmoves() >= 150 {
        Some(Outcome::DrawBy75MoveRule)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockAct, MockPlayerBuilder};
    use proptest::sample::Selector;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn game_ends_when_it_is_over(
        #[by_ref]
        #[filter(is_game_over(#pos).is_some())]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockAct::new();
        let b = MockAct::new();

        let mut wb = MockPlayerBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockPlayerBuilder::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        let outcome = is_game_over(&pos).unwrap();

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
            match is_game_over(&next) {
                Some(o) => break o,
                _ => {
                    let (m, _) = selector.select(next.moves());
                    moves.push(m);
                    sans.push(next.make(m)?);
                }
            }
        };

        moves.reverse();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let act = move |_: &Position| Ok(Action::Move(moves.pop().unwrap()));

        w.expect_act().returning(act.clone());
        b.expect_act().returning(act);

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
    fn players_can_resign_the_game(
        #[by_ref]
        #[filter(is_game_over(#pos).is_none())]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = pos.turn();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_act().return_const(Ok(Action::Resign));

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
                outcome: Outcome::Resignation(turn),
                moves: vec![San::null()]
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
        #[filter(is_game_over(#pos).is_none())]
        pos: Position,
        e: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = pos.turn();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_act().return_const(Err(e.clone()));

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
