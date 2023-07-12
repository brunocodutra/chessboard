use crate::{ai::Ai, engine::Engine, io::Io};
use anyhow::{Context, Error as Anyhow};
use clap::Parser;
use lib::chess::{Color, Fen, Move, Position};
use lib::search::{Depth, Limits, Options};
use rayon::max_num_threads;
use std::{num::NonZeroUsize, time::Duration};
use tokio::io::{stdin, stdout, Stdin, Stdout};
use tracing::{debug, error, instrument, warn};
use vampirc_uci::{self as uci, UciMessage, UciOptionConfig, UciSearchControl, UciTimeControl};

/// A basic *not fully compliant* UCI server.
#[derive(Debug, Default, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Uci {}

impl Uci {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        Server::new().run().await
    }
}

struct Server {
    io: Io<Stdout, Stdin>,
    engine: Engine,
    options: Options,
    position: Position,
    moves: Vec<Move>,
}

impl Server {
    fn new() -> Self {
        Server {
            io: Io::new(stdout(), stdin()),
            engine: Engine::default(),
            options: Options::default(),
            position: Position::default(),
            moves: Vec::default(),
        }
    }

    fn new_game(&mut self) {
        self.engine = Engine::new(self.options);
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

    async fn run(&mut self) -> Result<(), Anyhow> {
        loop {
            match uci::parse_one(self.io.recv().await?.trim()) {
                UciMessage::Uci => {
                    let name = UciMessage::id_name(env!("CARGO_PKG_NAME"));
                    let authors = UciMessage::id_author(env!("CARGO_PKG_AUTHORS"));

                    self.io.send(&name.to_string()).await?;
                    self.io.send(&authors.to_string()).await?;

                    let hash = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Hash".to_string(),
                        default: Some((Options::default().hash >> 20) as _),
                        min: Some(1),
                        max: Some(u16::MAX.into()),
                    });

                    self.io.send(&hash.to_string()).await?;

                    let thread = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Threads".to_string(),
                        default: Some(Options::default().threads.get() as _),
                        min: Some(1),
                        max: Some(max_num_threads().try_into().unwrap()),
                    });

                    self.io.send(&thread.to_string()).await?;
                    self.io.send(&UciMessage::UciOk.to_string()).await?;
                }

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "hash" => match self.set_hash(&value) {
                    Ok(_) => self.new_game(),
                    Err(e) => warn!("{:?}", e),
                },

                UciMessage::SetOption {
                    name,
                    value: Some(value),
                } if name.to_lowercase() == "threads" => match self.set_threads(&value) {
                    Ok(_) => self.new_game(),
                    Err(e) => warn!("{:?}", e),
                },

                UciMessage::UciNewGame => self.new_game(),
                UciMessage::IsReady => self.io.send(&UciMessage::ReadyOk.to_string()).await?,
                UciMessage::Quit => break Ok(()),

                UciMessage::Position {
                    startpos: true,
                    fen: None,
                    moves,
                } => {
                    let moves = Vec::from_iter(moves.into_iter().map(Move::from));
                    match moves.as_slice() {
                        [history @ .., m] if history == self.moves => {
                            self.position.play(*m)?;
                            self.moves.push(*m);
                        }

                        _ => {
                            self.position = Position::default();
                            self.moves.clear();
                            for m in moves {
                                self.position.play(m)?;
                                self.moves.push(m);
                            }
                        }
                    }
                }

                UciMessage::Position {
                    startpos: false,
                    fen: Some(fen),
                    moves,
                } if moves.is_empty() => {
                    let fen: Fen = fen.as_str().parse().context("invalid fen")?;
                    self.position = fen.try_into().context("illegal fen")?;
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
                    self.go(Limits::None).await?;
                }

                UciMessage::Go {
                    time_control: Some(UciTimeControl::MoveTime(time)),
                    search_control: None,
                } => {
                    let time = time.to_std().unwrap_or(Duration::MAX);
                    self.go(Limits::Time(time)).await?;
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

                    self.go(limits).await?;
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

                    self.go(limits).await?;
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
                } if search_moves.is_empty() => {
                    self.go(Depth::saturate(depth).into()).await?;
                }

                UciMessage::Unknown(m, cause) => {
                    let error = cause.map(Anyhow::new).unwrap_or_else(|| Anyhow::msg(m));
                    error!("{:?}", error.context("failed to parse UCI message"));
                }

                msg => match msg.direction() {
                    uci::CommunicationDirection::GuiToEngine => {
                        warn!("ignored engine bound message '{}'", msg)
                    }

                    uci::CommunicationDirection::EngineToGui => {
                        debug!("ignored unexpected gui bound message '{}'", msg)
                    }
                },
            }

            self.io.flush().await?;
        }
    }

    async fn go(&mut self, limits: Limits) -> Result<(), Anyhow> {
        let best = self.engine.play(&self.position, limits).await;
        self.position.play(best)?;
        self.moves.push(best);

        let msg = UciMessage::best_move(best.into());
        self.io.send(&msg.to_string()).await?;

        Ok(())
    }
}
