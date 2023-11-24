use crate::chess::{Color, Position, Square};
use crate::nnue::Evaluator;
use crate::search::{Depth, Engine, HashSize, Limits, Options, Score, ThreadCount};
use std::io::{self, stdin, stdout, Write};
use std::{num::NonZeroUsize, ops::Shr, time::Duration};
use vampirc_uci::{self as uci, *};

macro_rules! info {
    ($($arg:tt)*) => {{
        eprint!("INFO: ");
        eprintln!($($arg)*);
    }};
}

macro_rules! warn {
    ($($arg:tt)*) => {{
        eprint!("WARN: ");
        eprintln!($($arg)*);
    }};
}

macro_rules! error {
    ($($arg:tt)*) => {{
        eprint!("ERROR: ");
        eprintln!($($arg)*);
    }};
}

/// A basic *not fully compliant* UCI server.
pub struct Uci {
    engine: Engine,
    options: Options,
    position: Position,
    moves: Vec<UciMove>,
}

impl Default for Uci {
    fn default() -> Self {
        Self {
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
            Ok(h) => error!("hash size `{h}` is too large"),
            Err(e) => error!("invalid hash size `{value}`, {e}"),
        };
    }

    fn set_threads(&mut self, value: &str) {
        match value.parse::<NonZeroUsize>() {
            Ok(c) if ThreadCount::max() >= c => self.options.threads = ThreadCount::new(c),
            Ok(c) => error!("thread count `{c}` is too large"),
            Err(e) => error!("invalid thread count `{value}`, {e}"),
        }
    }

    fn set_position(&mut self, fen: UciFen) {
        match fen.as_str().parse() {
            Err(e) => error!("illegal FEN string `{fen}`, {e}"),
            Ok(pos) => {
                self.position = pos;
                self.moves.clear();
            }
        }
    }

    fn play(&mut self, uci: UciMove) {
        let whence: Square = uci.from.into();
        let moves = self.position.moves(whence.into());
        let Some(m) = moves.into_iter().find(|m| UciMove::from(*m) == uci) else {
            return error!("illegal move `{uci}` in position `{}`", self.position);
        };

        self.position.play(m);
        self.moves.push(uci);
    }

    fn go(&mut self, limits: Limits) -> Vec<UciMessage> {
        let pv = self.engine.search(&self.position, limits);
        let best = *pv.first().expect("the engine failed to find a move");

        let score = match pv.score().mate() {
            Some(p) if p > 0 => UciInfoAttribute::from_mate((p + 1).get() / 2),
            Some(p) => UciInfoAttribute::from_mate((p - 1).get() / 2),
            None => UciInfoAttribute::from_centipawns(pv.score().get().into()),
        };

        let pv = UciInfoAttribute::Pv(pv.into_iter().map(UciMove::from).collect());

        vec![
            UciMessage::Info(vec![score, pv]),
            UciMessage::best_move(best.into()),
        ]
    }

    fn eval(&mut self) -> Vec<UciMessage> {
        let pos = Evaluator::new(self.position.clone());
        let [material, positional, value]: [Score; 3] = match pos.turn() {
            Color::White => [
                pos.material().evaluate().cast(),
                pos.positional().evaluate().cast(),
                pos.evaluate().cast(),
            ],

            Color::Black => [
                -pos.material().evaluate().cast(),
                -pos.positional().evaluate().cast(),
                -pos.evaluate().cast(),
            ],
        };

        vec![UciMessage::Info(vec![
            UciInfoAttribute::Any("material".to_string(), material.to_string()),
            UciInfoAttribute::Any("positional".to_string(), positional.to_string()),
            UciInfoAttribute::Any("value".to_string(), value.to_string()),
        ])]
    }

    /// Processes one [`UciMessage`].
    pub fn process(&mut self, msg: UciMessage) -> Option<Vec<UciMessage>> {
        match msg {
            UciMessage::Uci => {
                let name = UciMessage::id_name(env!("CARGO_PKG_NAME"));
                let authors = UciMessage::id_author(env!("CARGO_PKG_AUTHORS"));

                let hash = UciMessage::Option(UciOptionConfig::Spin {
                    name: "Hash".to_string(),
                    default: Some(HashSize::default().shr(20) as _),
                    min: Some(0),
                    max: Some(HashSize::max().shr(20) as _),
                });

                let thread = UciMessage::Option(UciOptionConfig::Spin {
                    name: "Threads".to_string(),
                    default: Some(ThreadCount::default().get() as _),
                    min: Some(1),
                    max: Some(ThreadCount::max().get() as _),
                });

                Some(vec![name, authors, hash, thread, UciMessage::UciOk])
            }

            UciMessage::SetOption {
                name,
                value: Some(value),
            } if name.to_lowercase() == "hash" => {
                self.set_hash(&value);
                Some(vec![])
            }

            UciMessage::SetOption {
                name,
                value: Some(value),
            } if name.to_lowercase() == "threads" => {
                self.set_threads(&value);
                Some(vec![])
            }

            UciMessage::UciNewGame => {
                self.new_game();
                Some(vec![])
            }

            UciMessage::IsReady => Some(vec![UciMessage::ReadyOk]),
            UciMessage::Quit => None,

            UciMessage::Position {
                startpos: true,
                fen: None,
                moves,
            } => match moves.as_slice() {
                [history @ .., m, n] if history == self.moves => {
                    self.play(*m);
                    self.play(*n);
                    Some(vec![])
                }

                ms => {
                    self.moves.clear();
                    self.position = Position::default();
                    ms.iter().for_each(|&m| self.play(m));
                    Some(vec![])
                }
            },

            UciMessage::Position {
                startpos: false,
                fen: Some(fen),
                moves,
            } if moves.is_empty() => {
                self.set_position(fen);
                Some(vec![])
            }

            UciMessage::Go {
                time_control: None,
                search_control: None,
            }
            | UciMessage::Go {
                time_control: Some(UciTimeControl::Infinite),
                search_control: None,
            } => Some(self.go(Limits::None)),

            UciMessage::Go {
                time_control: Some(UciTimeControl::MoveTime(time)),
                search_control: None,
            } => {
                let time = time.to_std().unwrap_or(Duration::MAX);
                Some(self.go(Limits::Time(time)))
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

                Some(self.go(limits))
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

                Some(self.go(limits))
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
            } if search_moves.is_empty() => Some(self.go(Depth::saturate(depth).into())),

            UciMessage::Go {
                time_control: None,
                search_control:
                    Some(UciSearchControl {
                        depth: None,
                        search_moves,
                        mate: None,
                        nodes: Some(nodes),
                    }),
            } if search_moves.is_empty() => Some(self.go(nodes.into())),

            UciMessage::Unknown(msg, _) if msg.to_lowercase() == "eval" => Some(self.eval()),
            UciMessage::Unknown(msg, _) if msg.is_empty() => Some(vec![]),

            UciMessage::Unknown(_, Some(e)) => {
                error!("failed to parse UCI message\n{e}");
                Some(vec![])
            }

            UciMessage::Unknown(msg, None) => {
                error!("failed to parse UCI message `{msg}`");
                Some(vec![])
            }

            msg => match msg.direction() {
                uci::CommunicationDirection::GuiToEngine => {
                    warn!("ignored engine bound message '{msg}'");
                    Some(vec![])
                }

                uci::CommunicationDirection::EngineToGui => {
                    info!("ignored unexpected gui bound message '{msg}'");
                    Some(vec![])
                }
            },
        }
    }

    /// Runs the UCI server.
    pub fn run(&mut self) -> io::Result<()> {
        for line in stdin().lines() {
            let request = uci::parse_one(line?.trim());
            let Some(reply) = self.process(request) else {
                break;
            };

            let mut stdout = stdout().lock();
            for msg in reply {
                writeln!(&mut stdout, "{msg}")?;
            }

            stdout.flush()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{Bitboard, Move};
    use proptest::prelude::*;
    use proptest::sample::{size_range, Selector};
    use std::ops::BitAnd;
    use test_strategy::proptest;
    use vampirc_uci::Duration as UciDuration;

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
        #[map(|s: Selector| s.select(#pos.moves(Bitboard::full())))] m: Move,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            ..Uci::default()
        };

        uci.play(m.into());
        pos.play(m);
        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn play_updates_move_history(
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[any(size_range(..=10).lift())] ms: Vec<Move>,
        #[map(|s: Selector| s.select(#pos.moves(Bitboard::full())))] m: Move,
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
        #[by_ref]
        #[filter(#pos.outcome().is_none())]
        pos: Position,
        #[any(size_range(..=10).lift())] ms: Vec<Move>,
        #[filter(!#pos.moves(Bitboard::full()).any(|m| (m.whence(), m.whither()) == (#m.whence(), #m.whither())))]
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

    #[proptest]
    fn process_handles_uci() {
        let mut uci = Uci::default();

        assert!(uci
            .process(UciMessage::Uci)
            .is_some_and(|v| v.contains(&UciMessage::UciOk)));
    }

    #[proptest]
    fn process_handles_new_game(
        o: Options,
        pos: Position,
        #[any(size_range(..=10).lift())] ms: Vec<Move>,
    ) {
        let mut uci = Uci {
            options: o,
            position: pos,
            moves: ms.into_iter().map(UciMove::from).collect(),
            ..Uci::default()
        };

        uci.process(UciMessage::UciNewGame);
        assert_eq!(uci.options, o);
        assert_eq!(uci.position, Position::default());
        assert_eq!(uci.moves, Vec::default());
    }

    #[proptest]
    fn process_handles_option_hash(s: HashSize) {
        let mut uci = Uci::default();

        uci.process(UciMessage::SetOption {
            name: "hash".to_string(),
            value: Some(s.shr(20u32).to_string()),
        });

        assert_eq!(uci.options.hash, s.bitand(!0 << 20));
    }

    #[proptest]
    fn process_handles_option_threads(c: ThreadCount) {
        let mut uci = Uci::default();

        uci.process(UciMessage::SetOption {
            name: "threads".to_string(),
            value: Some(c.to_string()),
        });

        assert_eq!(uci.options.threads, c);
    }

    #[proptest]
    fn process_handles_ready() {
        let mut uci = Uci::default();

        assert_eq!(
            uci.process(UciMessage::IsReady),
            Some(vec![UciMessage::ReadyOk])
        );
    }

    #[proptest]
    fn process_handles_quit() {
        let mut uci = Uci::default();
        assert_eq!(uci.process(UciMessage::Quit), None);
    }

    #[proptest]
    fn process_handles_position(pos: Position) {
        let mut uci = Uci::default();

        uci.process(UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(pos.to_string())),
            moves: vec![],
        });

        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn process_handles_go_depth(d: Depth) {
        let mut uci = Uci::default();

        let reply = uci.process(UciMessage::Go {
            time_control: None,
            search_control: Some(UciSearchControl::depth(d.get())),
        });

        assert!(reply.is_some_and(|v| v.iter().any(|m| matches!(m, UciMessage::BestMove { .. }))));
    }

    #[proptest]
    fn process_handles_go_nodes(#[strategy(..1000u64)] n: u64) {
        let mut uci = Uci::default();

        let reply = uci.process(UciMessage::Go {
            time_control: None,
            search_control: Some(UciSearchControl::nodes(n)),
        });

        assert!(reply.is_some_and(|v| v.iter().any(|m| matches!(m, UciMessage::BestMove { .. }))));
    }

    #[proptest]
    fn process_handles_go_time(#[strategy(..10u8)] ms: u8) {
        let mut uci = Uci::default();

        let time = Duration::from_millis(ms as _);
        let reply = uci.process(UciMessage::Go {
            time_control: Some(UciTimeControl::MoveTime(UciDuration::from_std(time)?)),
            search_control: None,
        });

        assert!(reply.is_some_and(|v| v.iter().any(|m| matches!(m, UciMessage::BestMove { .. }))));
    }

    #[proptest]
    fn process_handles_go_time_left(#[strategy(..10u8)] t: u8, #[strategy(..10u8)] i: u8) {
        let mut uci = Uci::default();

        let clock = Duration::from_millis(t as _);
        let inc = Duration::from_millis(i as _);
        let reply = uci.process(UciMessage::Go {
            time_control: Some(UciTimeControl::TimeLeft {
                white_time: Some(UciDuration::from_std(clock)?),
                black_time: None,
                white_increment: Some(UciDuration::from_std(inc)?),
                black_increment: None,
                moves_to_go: None,
            }),
            search_control: None,
        });

        assert!(reply.is_some_and(|v| v.iter().any(|m| matches!(m, UciMessage::BestMove { .. }))));
    }

    #[proptest]
    fn process_handles_eval(pos: Position) {
        let mut uci = Uci {
            position: pos,
            ..Uci::default()
        };

        let reply = uci.process(UciMessage::Unknown("eval".to_string(), None));
        assert_eq!(reply, Some(uci.eval()));
    }
}
