use crate::io::Io;
use anyhow::{Context, Error as Anyhow};
use lib::chess::{Color, Move, Position};
use lib::nnue::Evaluator;
use lib::search::{Depth, Engine, Limits, Options, Pv};
use rayon::max_num_threads;
use std::io::{stdin, stdout, Stdin, Stdout};
use std::{num::NonZeroUsize, time::Duration};
use vampirc_uci::{self as uci, *};

/// A basic *not fully compliant* UCI server.
pub struct Uci {
    io: Io<Stdout, Stdin>,
    engine: Engine,
    options: Options,
    position: Position,
    moves: Vec<Move>,
}

impl Default for Uci {
    fn default() -> Self {
        Uci {
            io: Io::new(stdout(), stdin()),
            engine: Engine::default(),
            options: Options::default(),
            position: Position::default(),
            moves: Vec::default(),
        }
    }
}

impl Uci {
    fn new_game(&mut self) {
        self.engine = Engine::with_options(self.options);
        self.position = Position::default();
        self.moves.clear();
    }

    fn set_hash(&mut self, value: &str) -> Result<(), Anyhow> {
        self.options.hash = value
            .parse::<usize>()
            .context("invalid hash size")?
            .checked_shl(20)
            .unwrap_or(usize::MAX);

        Ok(())
    }

    fn set_threads(&mut self, value: &str) -> Result<(), Anyhow> {
        self.options.threads = value
            .parse::<NonZeroUsize>()
            .context("invalid thread count")?;

        Ok(())
    }

    fn go(&mut self, limits: Limits) -> Result<(), Anyhow> {
        let pv: Pv<1> = self.engine.search(&self.position, limits);
        let best = *pv.first().expect("expected some legal move");

        let score = match pv.score().mate() {
            Some(p) if p > 0 => UciInfoAttribute::from_mate((p + 1).get() / 2),
            Some(p) => UciInfoAttribute::from_mate((p - 1).get() / 2),
            None => UciInfoAttribute::from_centipawns(pv.score().get().into()),
        };

        self.io.send(UciMessage::Info(vec![score]))?;
        self.io.send(UciMessage::best_move(best.into()))?;

        Ok(())
    }

    fn eval(&mut self) -> Result<(), Anyhow> {
        let pos = Evaluator::borrow(&self.position);
        let (material, positional, value) = match pos.turn() {
            Color::White => (pos.material(), pos.positional(), pos.value()),
            Color::Black => (-pos.material(), -pos.positional(), -pos.value()),
        };

        self.io.send(UciMessage::Info(vec![
            UciInfoAttribute::Any("material".to_string(), material.to_string()),
            UciInfoAttribute::Any("positional".to_string(), positional.to_string()),
            UciInfoAttribute::Any("value".to_string(), value.to_string()),
        ]))?;

        Ok(())
    }

    /// Runs the UCI server.
    pub fn run(&mut self) -> Result<(), Anyhow> {
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
                        default: Some((Options::default().hash >> 20) as _),
                        min: Some(1),
                        max: Some(u16::MAX.into()),
                    });

                    self.io.send(hash)?;

                    let thread = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Threads".to_string(),
                        default: Some(Options::default().threads.get() as _),
                        min: Some(1),
                        max: Some(max_num_threads().try_into().unwrap()),
                    });

                    self.io.send(thread)?;
                    self.io.send(UciMessage::UciOk)?;
                }

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "hash" => match self.set_hash(&value) {
                    Ok(_) => self.new_game(),
                    Err(e) => eprintln!("{:?}", e),
                },

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "threads" => match self.set_threads(&value) {
                    Ok(_) => self.new_game(),
                    Err(e) => eprintln!("{:?}", e),
                },

                UciMessage::UciNewGame => self.new_game(),
                UciMessage::IsReady => self.io.send(UciMessage::ReadyOk)?,
                UciMessage::Quit => break Ok(()),

                UciMessage::Position {
                    startpos: true,
                    fen: None,
                    moves,
                } => match Vec::from_iter(moves.into_iter().map(Move::from)).as_slice() {
                    [history @ .., m, n] if history == self.moves => {
                        self.position.play(*m)?;
                        self.moves.push(*m);

                        self.position.play(*n)?;
                        self.moves.push(*n);
                    }

                    ms => {
                        self.position = Position::default();
                        self.moves.clear();
                        for &m in ms {
                            self.position.play(m)?;
                            self.moves.push(m);
                        }
                    }
                },

                UciMessage::Position {
                    startpos: false,
                    fen: Some(fen),
                    moves,
                } if moves.is_empty() => {
                    self.position = fen.as_str().parse()?;
                    self.moves.clear();
                }

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

                UciMessage::Unknown(m, _) if m.to_lowercase() == "eval" => self.eval()?,

                UciMessage::Unknown(m, cause) => {
                    let error = cause.map(Anyhow::new).unwrap_or_else(|| Anyhow::msg(m));
                    eprintln!("{:?}", error.context("failed to parse UCI message"));
                }

                msg => match msg.direction() {
                    uci::CommunicationDirection::GuiToEngine => {
                        eprintln!("ignored engine bound message '{}'", msg);
                    }

                    uci::CommunicationDirection::EngineToGui => {
                        eprintln!("ignored unexpected gui bound message '{}'", msg);
                    }
                },
            }
        }
    }
}
