use chessboard::chess::{Color, Outcome, Pgn, Position};
use chessboard::{play::Play, util::Build};
use derive_more::{Constructor, Display, Error};
use std::fmt::Display;
use test_strategy::Arbitrary;
use tracing::{field::display, instrument, warn, Span};

/// The reason why the [`Game`] was interrupted.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "the {} player encountered an error")]
pub enum GameInterrupted<W, B> {
    #[display(fmt = "white")]
    White(W),

    #[display(fmt = "black")]
    Black(B),
}

/// Holds the state of a game of chess.
#[derive(Debug, Arbitrary, Constructor)]
pub struct Game<W, B> {
    white: W,
    black: B,
}

impl<W, B> Game<W, B>
where
    W: Build + Display,
    B: Build + Display,
    W::Output: Play<Error = W::Error>,
    B::Output: Play<Error = B::Error>,
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
    use chessboard::chess::MoveKind;
    use chessboard::play::{MockBuilder as MockPlayerBuilder, MockPlay};
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

        let w = MockPlay::new();
        let b = MockPlay::new();

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
                    let (m, _, _) = selector.select(next.moves(MoveKind::ANY));
                    moves.push(m);
                    sans.push(next.make(m)?);
                }
            }
        };

        moves.reverse();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let act = move |_: &Position| Ok(moves.pop().unwrap());

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
        #[filter(is_game_over(#pos).is_none())]
        pos: Position,
        e: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = pos.turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_play().return_const(Err(e.clone()));

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
