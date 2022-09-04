use super::Execute;
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use chessboard::chess::{Color, Outcome, Pgn, Position};
use chessboard::engine::{Builder as EngineBuilder, Engine};
use chessboard::util::Build;
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use derive_more::{Constructor, Display, Error};
use libm::erf;
use std::{fmt::Display, num::NonZeroUsize};
use test_strategy::Arbitrary;
use tracing::{field::display, info, instrument, warn, Span};

/// A match of chess between two players.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Play {
    /// How many games to play.
    #[clap(short = 'n', long, default_value = "1")]
    games: NonZeroUsize,

    /// The challenging player starts with the white pieces.
    challenger: EngineBuilder,

    /// The defending player starts with the black pieces.
    defender: EngineBuilder,
}

#[async_trait]
impl Execute for Play {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let players = [self.challenger, self.defender];
        let (mut wins, mut losses, mut draws) = (0f64, 0f64, 0f64);
        let mut pgns = Vec::with_capacity(self.games.into());

        for n in 0..self.games.into() {
            let game = Game::new(players[n % 2].clone(), players[(n + 1) % 2].clone());
            let pgn = game.play(Position::default()).await?;

            let wl = [&mut wins, &mut losses];
            match pgn.outcome.winner() {
                Some(c) => *wl[(n + c as usize) % 2] += 1.,
                _ => draws += 1.,
            }

            info!(
                games = n + 1,
                challenger = wins + draws / 2.,
                defender = losses + draws / 2.,
                ΔELO = -400. * ((wins + losses + draws) / (wins + draws / 2.) - 1.).log10(),
                LOS = (1. + erf((wins - losses) / (2. * (wins + losses)).sqrt())) / 2.
            );

            pgns.push(pgn);
        }

        for pgn in pgns {
            println!("{}\n", pgn);
        }

        Ok(())
    }
}

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
    W::Output: Engine<Error = W::Error>,
    B::Output: Engine<Error = B::Error>,
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
                Color::White => white.best(&pos).await.map_err(White)?,
                Color::Black => black.best(&pos).await.map_err(Black)?,
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
    use chessboard::engine::{MockBuilder as MockEngineBuilder, MockEngine};
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

        let w = MockEngine::new();
        let b = MockEngine::new();

        let mut wb = MockEngineBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockEngineBuilder::new();
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

        let mut w = MockEngine::new();
        let mut b = MockEngine::new();

        let act = move |_: &Position| Ok(moves.pop().unwrap());

        w.expect_best().returning(act.clone());
        b.expect_best().returning(act);

        let mut wb = MockEngineBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockEngineBuilder::new();
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

        let mut wb = MockEngineBuilder::new();
        let bb = MockEngineBuilder::new();

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

        let mut w = MockEngine::new();
        let mut b = MockEngine::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_best().return_const(Err(e.clone()));

        let mut wb = MockEngineBuilder::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockEngineBuilder::new();
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
