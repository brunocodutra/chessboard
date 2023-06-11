use crate::{build::Build, play::Play};
use derive_more::{Constructor, Display, Error, From};
use lib::chess::{Color, Move, Outcome, Pgn, Position};
use lib::search::Limits;
use std::{fmt::Display, time::Instant};
use tokio::time::timeout;
use tracing::{field::display, instrument, warn, Span};

/// The reason why the [`Player`] was unable to make a move.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
enum PlayerError<E> {
    Error(E),

    #[from(ignore)]
    #[display(fmt = "lost on time")]
    LostOnTime,
}

/// A clock aware generic chess player.
#[derive(Debug, Constructor)]
struct Player<P: Play> {
    engine: P,
    limits: Limits,
}

impl<P: Play> Player<P> {
    #[instrument(level = "debug", skip(self, pos), fields(%pos, control = %self.limits))]
    async fn act(&mut self, pos: &Position) -> Result<Move, PlayerError<P::Error>> {
        use PlayerError::LostOnTime;

        if let Limits::Clock(t, i) = &mut self.limits {
            *t = t.saturating_add(*i);
        }

        let timer = Instant::now();
        let m = match timeout(self.limits.clock(), self.engine.play(pos, self.limits)).await {
            Err(_) => return Err(LostOnTime),
            Ok(r) => r?,
        };

        if let Limits::Clock(t, _) = &mut self.limits {
            let time = timer.elapsed();
            *t = t.checked_sub(time).ok_or(LostOnTime)?;
        }

        Ok(m)
    }
}

/// The reason why the [`Game`] was interrupted.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(fmt = "the {} player encountered an error")]
pub enum GameInterrupted<E> {
    #[display(fmt = "white")]
    White(E),

    #[display(fmt = "black")]
    Black(E),
}

/// Holds the state of a game of chess.
#[derive(Debug, Constructor)]
pub struct Game<C> {
    white: C,
    black: C,
}

impl<C> Game<C>
where
    C: Build + Display,
    C::Output: Play<Error = C::Error>,
{
    /// Play a game of chess from the given starting [`Position`].
    #[instrument(level = "debug", skip(self), err,
        fields(white = %self.white, black = %self.black, %pos, %limits, outcome))]
    pub async fn play(
        self,
        mut pos: Position,
        limits: Limits,
    ) -> Result<Pgn, GameInterrupted<C::Error>> {
        use GameInterrupted::*;

        let (white, black) = (self.white.to_string(), self.black.to_string());

        let mut players = [
            Player::new(self.white.build().map_err(White)?, limits),
            Player::new(self.black.build().map_err(Black)?, limits),
        ];

        let mut moves = Vec::new();

        let outcome = loop {
            match pos.outcome() {
                Some(o) => break o,
                _ => match players[pos.turn() as usize].act(&pos).await {
                    Err(PlayerError::LostOnTime) => break Outcome::LossOnTime(pos.turn()),
                    Err(PlayerError::Error(e)) => match pos.turn() {
                        Color::White => return Err(White(e)),
                        Color::Black => return Err(Black(e)),
                    },
                    Ok(m) => match pos.make(m) {
                        Ok(san) => moves.push(san),
                        Err(e) => {
                            warn!("{:?}", e);
                            break Outcome::Resignation(pos.turn());
                        }
                    },
                },
            }
        };

        Span::current().record("outcome", display(outcome));

        Ok(Pgn {
            white,
            black,
            outcome,
            moves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build::MockBuilder, play::MockPlay};
    use futures_util::FutureExt;
    use lib::chess::MoveKind;
    use proptest::sample::Selector;
    use std::sync::{Arc, Mutex};
    use std::{future::ready, time::Duration};
    use test_strategy::proptest;
    use tokio::{runtime, time::sleep};

    #[proptest]
    fn player_keeps_track_of_clock(
        #[filter(#pos.outcome().is_none())] pos: Position,
        selector: Selector,
        #[strategy(0u32..)] clock: u32,
        #[strategy(0u32..)] inc: u32,
    ) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut p = MockPlay::new();

        let (m, _) = selector.select(pos.moves(MoveKind::ANY));
        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(Ok(m))));

        let l = Limits::Clock(
            Duration::from_secs(clock.into()),
            Duration::from_secs(inc.into()),
        );

        let mut p = Player::new(p, l);
        assert_eq!(rt.block_on(p.act(&pos)), Ok(m));
        assert!(l.clock() < p.limits.clock());
        assert!(p.limits.clock() < l.clock() + l.increment());
    }

    #[proptest]
    fn player_loses_on_time_if_does_not_act_in_time(
        #[filter(#pos.outcome().is_none())] pos: Position,
        selector: Selector,
        clock: u8,
        inc: u8,
    ) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut p = MockPlay::new();

        let moves = pos.moves(MoveKind::ANY);
        p.expect_play().return_once(move |_, _| {
            Box::pin(async move {
                sleep(Duration::from_millis(1)).await;
                let (m, _) = selector.select(moves);
                Ok(m)
            })
        });

        let l = Limits::Clock(
            Duration::from_micros(clock.into()),
            Duration::from_micros(inc.into()),
        );

        assert_eq!(
            rt.block_on(Player::new(p, l).act(&pos)),
            Err(PlayerError::LostOnTime)
        );
    }

    #[proptest]
    fn player_loses_on_time_if_clock_reaches_zero(
        #[filter(#pos.outcome().is_none())] pos: Position,
        selector: Selector,
    ) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut p = MockPlay::new();

        let (m, _) = selector.select(pos.moves(MoveKind::ANY));
        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(Ok(m))));

        let l = Limits::Clock(Duration::ZERO, Duration::ZERO);

        assert_eq!(
            rt.block_on(Player::new(p, l).act(&pos)),
            Err(PlayerError::LostOnTime)
        );
    }

    #[proptest]
    fn game_ends_when_it_is_over(#[filter(#pos.outcome().is_some())] pos: Position, l: Limits) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockPlay::new();
        let b = MockPlay::new();

        let mut wb = MockBuilder::<MockPlay>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlay>::new();
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
    fn game_ends_when_player_loses_on_time(
        #[filter(#pos.outcome().is_none())] pos: Position,
        selector: Selector,
    ) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let turn = pos.turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        let (m, _) = selector.select(pos.moves(MoveKind::ANY));

        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(Ok(m))));

        let mut wb = MockBuilder::<MockPlay>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlay>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        let l = Limits::Clock(Duration::ZERO, Duration::ZERO);

        let outcome = Outcome::LossOnTime(turn);

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
    fn game_ends_in_resignation_when_player_attempts_illegal_move(
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter(#pos.moves(MoveKind::ANY).find(|&(m, _)| m == #m).is_none())] m: Move,
    ) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let turn = pos.turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(Ok(m))));

        let mut wb = MockBuilder::<MockPlay>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlay>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);
        let outcome = Outcome::Resignation(turn);

        assert_eq!(
            rt.block_on(g.play(pos, Limits::None)),
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
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

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

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let moves = Arc::new(Mutex::new(moves));

        let act =
            move |_: &Position, _: Limits| ready(Ok(moves.lock().unwrap().pop().unwrap())).boxed();

        w.expect_play().returning(act.clone());
        b.expect_play().returning(act);

        let mut wb = MockBuilder::<MockPlay>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlay>::new();
        bb.expect_build().once().return_once(move || Ok(b));

        let wc = wb.to_string();
        let bc = bb.to_string();

        let g = Game::new(wb, bb);

        assert_eq!(
            rt.block_on(g.play(pos, Limits::None)),
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

        let mut wb = MockBuilder::<MockPlay>::new();
        let bb = MockBuilder::<MockPlay>::new();

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
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let turn = pos.turn();

        let mut w = MockPlay::new();
        let mut b = MockPlay::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        let err = Err(e.clone());
        p.expect_play()
            .return_once(move |_, _| Box::pin(ready(err)));

        let mut wb = MockBuilder::<MockPlay>::new();
        wb.expect_build().once().return_once(move || Ok(w));

        let mut bb = MockBuilder::<MockPlay>::new();
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
