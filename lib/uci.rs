use crate::chess::{Color, Position};
use crate::nnue::Evaluator;
use crate::search::{Depth, Engine, HashSize, Limits, Options, ThreadCount};
use crate::util::{Assume, Io};
use std::io::{self, stdin, stdout, Stdin, Stdout};
use std::{num::NonZeroUsize, ops::Shr, time::Duration};
use vampirc_uci::{self as uci, *};

/// A basic *not fully compliant* UCI server.
pub struct Uci {
    io: Io<Stdout, Stdin>,
    engine: Engine,
    options: Options,
    position: Position,
    moves: Vec<UciMove>,
}

impl Default for Uci {
    fn default() -> Self {
        Uci {
            io: Io::new(stdout(), stdin()),
            engine: Engine::default(),
            options: Options::default(),
            position: Position::default(),
            moves: Vec::with_capacity(128),
        }
    }
}

impl Uci {
    fn new_game(&mut self) {
        self.engine = Engine::with_options(self.options);
        self.position = Position::default();
        self.moves.clear();
    }

    fn set_hash(&mut self, value: &str) {
        match value.parse::<usize>() {
            Ok(h) if HashSize::max().shr(20) >= h => self.options.hash = HashSize::new(h << 20),
            Ok(h) => eprintln!("hash size `{h}` is too large"),
            Err(e) => eprintln!("invalid hash size `{value}`, {e}"),
        };
    }

    fn set_threads(&mut self, value: &str) {
        match value.parse::<NonZeroUsize>() {
            Ok(c) if ThreadCount::max() >= c => self.options.threads = ThreadCount::new(c),
            Ok(c) => eprintln!("thread count `{c}` is too large"),
            Err(e) => eprintln!("invalid thread count `{value}`, {e}"),
        }
    }

    fn set_position(&mut self, fen: UciFen) {
        match fen.as_str().parse() {
            Err(e) => eprintln!("illegal FEN string `{fen}`, {e}"),
            Ok(pos) => {
                self.position = pos;
                self.moves.clear();
            }
        }
    }

    fn play(&mut self, uci: UciMove) {
        let Some(m) = self.position.moves().find(|m| UciMove::from(*m) == uci) else {
            return eprintln!("illegal move `{uci}` in position `{}`", self.position);
        };

        self.position.play(m).assume();
        self.moves.push(uci);
    }

    fn go(&mut self, limits: Limits) -> io::Result<()> {
        let pv = self.engine.search(&self.position, limits);
        let best = *pv.first().expect("the engine failed to find a move");

        let score = match pv.score().mate() {
            Some(p) if p > 0 => UciInfoAttribute::from_mate((p + 1).get() / 2),
            Some(p) => UciInfoAttribute::from_mate((p - 1).get() / 2),
            None => UciInfoAttribute::from_centipawns(pv.score().get().into()),
        };

        let pv = UciInfoAttribute::Pv(pv.into_iter().map(UciMove::from).collect());

        self.io.send(UciMessage::Info(vec![score, pv]))?;
        self.io.send(UciMessage::best_move(best.into()))?;

        Ok(())
    }

    fn eval(&mut self) -> io::Result<()> {
        let pos = Evaluator::new(self.position.clone());
        let (material, positional, value) = match pos.turn() {
            Color::White => (
                pos.material().evaluate(),
                pos.positional().evaluate(),
                pos.evaluate(),
            ),

            Color::Black => (
                -pos.material().evaluate(),
                -pos.positional().evaluate(),
                -pos.evaluate(),
            ),
        };

        self.io.send(UciMessage::Info(vec![
            UciInfoAttribute::Any("material".to_string(), material.to_string()),
            UciInfoAttribute::Any("positional".to_string(), positional.to_string()),
            UciInfoAttribute::Any("value".to_string(), value.to_string()),
        ]))?;

        Ok(())
    }

    /// Runs the UCI server.
    pub fn run(&mut self) -> io::Result<()> {
        loop {
            self.io.flush()?;
            match uci::parse_one(self.io.recv()?.trim()) {
                UciMessage::Uci => {
                    let name = UciMessage::id_name(env!("CARGO_PKG_NAME"));
                    let authors = UciMessage::id_author(env!("CARGO_PKG_AUTHORS"));

                    self.io.send(name)?;
                    self.io.send(authors)?;

                    let hash = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Hash".to_string(),
                        default: Some(HashSize::default().shr(20) as _),
                        min: Some(0),
                        max: Some(HashSize::max().shr(20) as _),
                    });

                    self.io.send(hash)?;

                    let thread = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Threads".to_string(),
                        default: Some(ThreadCount::default().get() as _),
                        min: Some(1),
                        max: Some(ThreadCount::max().get() as _),
                    });

                    self.io.send(thread)?;
                    self.io.send(UciMessage::UciOk)?;
                }

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "hash" => self.set_hash(&value),

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "threads" => self.set_threads(&value),

                UciMessage::UciNewGame => self.new_game(),
                UciMessage::IsReady => self.io.send(UciMessage::ReadyOk)?,
                UciMessage::Quit => break Ok(()),

                UciMessage::Position {
                    startpos: true,
                    fen: None,
                    moves,
                } => match moves.as_slice() {
                    [history @ .., m, n] if history == self.moves => {
                        self.play(*m);
                        self.play(*n);
                    }

                    ms => {
                        self.position = Position::default();
                        self.moves.clear();
                        for m in ms {
                            self.play(*m);
                        }
                    }
                },

                UciMessage::Position {
                    startpos: false,
                    fen: Some(fen),
                    moves,
                } if moves.is_empty() => self.set_position(fen),

                UciMessage::Go {
                    time_control: None,
                    search_control: None,
                }
                | UciMessage::Go {
                    time_control: Some(UciTimeControl::Infinite),
                    search_control: None,
                } => {
                    self.go(Limits::None)?;
                }

                UciMessage::Go {
                    time_control: Some(UciTimeControl::MoveTime(time)),
                    search_control: None,
                } => {
                    let time = time.to_std().unwrap_or(Duration::MAX);
                    self.go(Limits::Time(time))?;
                }

                UciMessage::Go {
                    search_control: None,
                    time_control:
                        Some(UciTimeControl::TimeLeft {
                            white_time: Some(time),
                            white_increment: Some(increment),
                            moves_to_go: None,
                            ..
                        }),
                } if self.position.turn() == Color::White => {
                    let limits = Limits::Clock(
                        time.to_std().unwrap_or(Duration::MAX),
                        increment.to_std().unwrap_or(Duration::MAX),
                    );

                    self.go(limits)?;
                }

                UciMessage::Go {
                    search_control: None,
                    time_control:
                        Some(UciTimeControl::TimeLeft {
                            black_time: Some(time),
                            black_increment: Some(increment),
                            moves_to_go: None,
                            ..
                        }),
                } if self.position.turn() == Color::Black => {
                    let limits = Limits::Clock(
                        time.to_std().unwrap_or(Duration::MAX),
                        increment.to_std().unwrap_or(Duration::MAX),
                    );

                    self.go(limits)?;
                }

                UciMessage::Go {
                    time_control: None,
                    search_control:
                        Some(UciSearchControl {
                            depth: Some(depth),
                            search_moves,
                            mate: None,
                            nodes: None,
                        }),
                } if search_moves.is_empty() => self.go(Depth::saturate(depth).into())?,

                UciMessage::Go {
                    time_control: None,
                    search_control:
                        Some(UciSearchControl {
                            depth: None,
                            search_moves,
                            mate: None,
                            nodes: Some(nodes),
                        }),
                } if search_moves.is_empty() => self.go(nodes.into())?,

                UciMessage::Unknown(m, _) if m.to_lowercase() == "eval" => self.eval()?,

                UciMessage::Unknown(m, cause) => {
                    eprintln!("failed to parse UCI message `{m}`");

                    if let Some(e) = cause {
                        eprintln!("{e}");
                    }
                }

                msg => match msg.direction() {
                    uci::CommunicationDirection::GuiToEngine => {
                        eprintln!("ignored engine bound message '{msg}'");
                    }

                    uci::CommunicationDirection::EngineToGui => {
                        eprintln!("ignored unexpected gui bound message '{msg}'");
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Move;
    use proptest::prelude::*;
    use proptest::sample::{size_range, Selector};
    use std::ops::BitAnd;
    use test_strategy::proptest;

    #[proptest]
    fn new_game_preserves_options(o: Options) {
        let mut uci = Uci {
            options: o,
            ..Uci::default()
        };

        uci.new_game();
        assert_eq!(uci.options, o);
    }

    #[proptest]
    fn new_game_resets_position(pos: Position) {
        let mut uci = Uci {
            position: pos,
            ..Uci::default()
        };

        uci.new_game();
        assert_eq!(uci.position, Position::default());
    }

    #[proptest]
    fn new_game_resets_move_history(#[any(size_range(..=10).lift())] ms: Vec<Move>) {
        let mut uci = Uci {
            moves: ms.into_iter().map(UciMove::from).collect(),
            ..Uci::default()
        };

        uci.new_game();
        assert_eq!(uci.moves, Vec::default());
    }

    #[proptest]
    fn set_hash_updates_options(s: HashSize) {
        let mut uci = Uci::default();
        uci.set_hash(&s.shr(20u32).to_string());
        assert_eq!(uci.options.hash, s.bitand(!0 << 20));
    }

    #[proptest]
    fn set_hash_ignores_value_too_large(#[strategy(HashSize::max().shr(20) as u64 + 1..)] v: u64) {
        let mut uci = Uci::default();
        uci.set_hash(&v.to_string());
        assert_eq!(uci.options.hash, Options::default().hash);
    }

    #[proptest]
    fn set_hash_ignores_not_a_number(#[filter(#v.parse::<usize>().is_err())] v: String) {
        let mut uci = Uci::default();
        uci.set_hash(&v);
        assert_eq!(uci.options.hash, Options::default().hash);
    }

    #[proptest]
    fn set_threads_updates_options(c: ThreadCount) {
        let mut uci = Uci::default();
        uci.set_threads(&c.to_string());
        assert_eq!(uci.options.threads, c);
    }

    #[proptest]
    fn set_threads_ignores_number_too_large(
        #[strategy((ThreadCount::max().get() + 1..).prop_map(|t| NonZeroUsize::new(t).unwrap()))]
        v: NonZeroUsize,
    ) {
        let mut uci = Uci::default();
        uci.set_threads(&v.to_string());
        assert_eq!(uci.options.threads, Options::default().threads);
    }

    #[proptest]
    fn set_threads_ignores_not_a_number(#[filter(#v.parse::<NonZeroUsize>().is_err())] v: String) {
        let mut uci = Uci::default();
        uci.set_threads(&v);
        assert_eq!(uci.options.threads, Options::default().threads);
    }

    #[proptest]
    fn set_position_updates_position(pos: Position) {
        let mut uci = Uci::default();
        uci.set_position(UciFen(pos.to_string()));
        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn set_position_ignores_illegal_fen(#[filter(#fen.parse::<Position>().is_err())] fen: String) {
        let mut uci = Uci::default();
        uci.set_position(UciFen(fen));
        assert_eq!(uci.position, Position::default());
    }

    #[proptest]
    fn play_updates_position(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            ..Uci::default()
        };

        uci.play(m.into());
        assert_eq!(pos.play(m), Ok(m));
        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn play_updates_move_history(
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[any(size_range(..=10).lift())] ms: Vec<Move>,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            moves: ms.iter().copied().map(UciMove::from).collect(),
            ..Uci::default()
        };

        uci.play(m.into());

        assert_eq!(
            uci.moves,
            Vec::from_iter(ms.into_iter().chain([m]).map(UciMove::from))
        );
    }

    #[proptest]
    fn play_ignores_illegal_move(
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[any(size_range(..=10).lift())] ms: Vec<Move>,
        #[filter(!#pos.moves().any(|m| (m.whence(), m.whither()) == (#m.whence(), #m.whither())))]
        m: Move,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            moves: ms.iter().copied().map(UciMove::from).collect(),
            ..Uci::default()
        };

        uci.play(m.into());
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, Vec::from_iter(ms.into_iter().map(UciMove::from)));
    }
}
