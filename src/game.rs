use crate::{Action, Color, GameReport, InvalidAction, Outcome, Play, Position, San};
use anyhow::Context;
use derive_more::{Display, Error};
use tracing::{info, instrument, warn};

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
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Game {
    position: Position,
    resigned: Option<Color>,
}

impl Game {
    /// The current [`Position`].
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// The [`Outcome`] of the game if it has already ended.
    pub fn outcome(&self) -> Option<Outcome> {
        if let Some(p) = self.resigned {
            Outcome::Resignation(p).into()
        } else if self.position().is_checkmate() {
            Outcome::Checkmate(!self.position().turn()).into()
        } else if self.position().is_stalemate() {
            Outcome::Stalemate.into()
        } else if self.position().is_material_insufficient() {
            Outcome::DrawByInsufficientMaterial.into()
        } else {
            None
        }
    }

    /// Executes an [`Action`] if valid, otherwise returns the reason why not.
    ///
    /// If the action is valid, a [`San`] recording the move is returned.
    pub fn execute(&mut self, action: Action) -> Result<San, InvalidAction> {
        if let Some(result) = self.outcome() {
            return Err(InvalidAction::GameHasEnded(result));
        }

        match action {
            Action::Move(m) => Ok(self.position.play(m)?),

            Action::Resign => {
                self.resigned.replace(self.position.turn());
                Ok(San::null())
            }
        }
    }

    /// Challenge two players for a game of chess.
    #[instrument(level = "trace", err, ret, skip(white, black))]
    pub async fn run<W: Play, B: Play>(
        &mut self,
        mut white: W,
        mut black: B,
    ) -> Result<GameReport, GameInterrupted<W::Error, B::Error>> {
        let mut moves = Vec::new();

        loop {
            match self.outcome() {
                Some(outcome) => break Ok(GameReport { outcome, moves }),

                None => {
                    let position = self.position();

                    info!(%position);

                    let turn = position.turn();

                    use GameInterrupted::*;
                    let action = match turn {
                        Color::White => white.play(position).await.map_err(White)?,
                        Color::Black => black.play(position).await.map_err(Black)?,
                    };

                    info!(player = %turn, %action);

                    match self.execute(action).context("invalid player action") {
                        Err(e) => warn!("{:?}", e),
                        Ok(san) => moves.push(san),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockPlay, Move, PositionKind};
    use proptest::{prop_assume, sample::select};
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn position_borrows_the_current_game_state(game: Game) {
        assert_eq!(game.position(), &game.position);
    }

    #[proptest]
    fn outcome_returns_some_result_if_a_player_has_resigned(pos: Position, p: Color) {
        let game = Game {
            position: pos,
            resigned: Some(p),
        };

        assert_eq!(game.outcome(), Some(Outcome::Resignation(p)));
    }

    #[proptest]
    fn outcome_returns_some_result_on_a_checkmate_position(
        #[any(PositionKind::Checkmate)] pos: Position,
    ) {
        let game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(
            game.outcome(),
            Some(Outcome::Checkmate(!game.position().turn()))
        );
    }

    #[proptest]
    fn outcome_returns_some_result_on_a_stalemate_position(
        #[any(PositionKind::Stalemate)] pos: Position,
    ) {
        let game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(game.outcome(), Some(Outcome::Stalemate));
    }

    #[proptest]
    fn outcome_returns_some_result_on_position_with_insufficient_material(
        #[any(PositionKind::InsufficientMaterial)] pos: Position,
    ) {
        let game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(game.outcome(), Some(Outcome::DrawByInsufficientMaterial));
    }

    #[proptest]
    fn any_player_action_after_one_side_has_resigned_is_invalid(
        pos: Position,
        p: Color,
        a: Action,
    ) {
        let mut game = Game {
            position: pos,
            resigned: Some(p),
        };

        assert_eq!(game.execute(a).err(), game.outcome().map(Into::into));
    }

    #[proptest]
    fn any_player_action_on_a_checkmate_position_is_invalid(
        #[any(PositionKind::Checkmate)] pos: Position,
        a: Action,
    ) {
        let mut game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(game.execute(a).err(), game.outcome().map(Into::into));
    }

    #[proptest]
    fn any_player_action_on_a_stalemate_position_is_invalid(
        #[any(PositionKind::Stalemate)] pos: Position,
        a: Action,
    ) {
        let mut game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(game.execute(a).err(), game.outcome().map(Into::into));
    }

    #[proptest]
    fn any_player_action_on_position_with_insufficient_material_is_invalid(
        #[any(PositionKind::InsufficientMaterial)] pos: Position,
        a: Action,
    ) {
        let mut game = Game {
            position: pos,
            resigned: None,
        };

        assert_eq!(game.execute(a).err(), game.outcome().map(Into::into));
    }

    #[proptest]
    fn game_state_does_not_change_after_an_invalid_action(mut game: Game, a: Action) {
        let before = game.clone();
        prop_assume!(game.execute(a).is_err());
        assert_eq!(game, before);
    }

    #[proptest]
    fn players_can_resign(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
    ) {
        assert_eq!(game.execute(Action::Resign), Ok(San::null()));
        assert_eq!(
            game,
            Game {
                resigned: Some(game.position.turn()),
                ..game.clone()
            }
        );
    }

    #[proptest]
    fn players_can_make_a_move(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
        m: Move,
    ) {
        let mut pos = game.position.clone();

        assert_eq!(
            game.execute(Action::Move(m)),
            pos.play(m).map_err(Into::into)
        );

        assert_eq!(
            game,
            Game {
                position: pos,
                resigned: None
            }
        );
    }

    #[proptest]
    fn game_ends_once_an_outcome_is_reached(
        #[by_ref]
        #[filter(#game.outcome().is_some())]
        mut game: Game,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockPlay::new();
        let b = MockPlay::new();

        assert_eq!(
            rt.block_on(game.run(w, b)).ok().map(|r| r.outcome),
            game.outcome()
        );
    }

    #[proptest]
    fn game_returns_sequence_of_moves_in_standard_notation(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
        #[strategy(select(#game.position().moves().collect::<Vec<_>>()))] m: Move,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = game.position().turn();
        let san = game.position().clone().play(m)?;

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let (p, q) = match turn {
            Color::White => (&mut w, &mut b),
            Color::Black => (&mut b, &mut w),
        };

        p.expect_play().return_const(Ok(Action::Move(m)));
        q.expect_play().return_const(Ok(Action::Resign));

        let report = rt.block_on(game.run(w, b));

        prop_assume!(game.outcome() == Some(Outcome::Resignation(!turn)));
        assert_eq!(report.map(|r| r.moves), Ok(vec![san, San::null()]));
    }

    #[proptest]
    fn game_executes_player_actions_in_their_turn(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = game.position().turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_play().return_const(Ok(Action::Resign));

        assert_eq!(
            rt.block_on(game.run(w, b)).map(|r| r.outcome),
            Ok(Outcome::Resignation(turn))
        );
    }

    #[proptest]
    fn game_ignores_invalid_player_actions(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
        #[filter(#game.clone().execute(#action).is_err())] action: Action,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = game.position().turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_play()
            .return_const(Ok(action))
            .return_const(Ok(Action::Resign));

        assert_eq!(
            rt.block_on(game.run(w, b)).map(|r| r.outcome),
            Ok(Outcome::Resignation(turn))
        );
    }

    #[proptest]
    fn game_interrupts_if_player_encounters_an_error(
        #[by_ref]
        #[filter(#game.outcome().is_none())]
        mut game: Game,
        e: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = game.position().turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_play().return_const(Err(e.clone()));

        assert_eq!(
            rt.block_on(game.run(w, b)),
            match turn {
                Color::White => Err(GameInterrupted::White(e)),
                Color::Black => Err(GameInterrupted::Black(e)),
            }
        );
    }
}
