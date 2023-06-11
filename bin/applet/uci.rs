use crate::io::{Io, Pipe};
use crate::{engine::Ai, play::Play};
use anyhow::{Context, Error as Anyhow};
use clap::Parser;
use lib::chess::{Color, Fen, Position};
use lib::search::{Depth, Limits, Options};
use rayon::max_num_threads;
use std::{num::NonZeroUsize, time::Duration};
use tokio::io::{stdin, stdout};
use tracing::{debug, error, instrument, warn};
use vampirc_uci::{self as uci, UciMessage, UciOptionConfig, UciSearchControl, UciTimeControl};

/// A basic *not fully compliant* UCI server.
#[derive(Debug, Default, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Uci {}

impl Uci {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let io = Pipe::new(stdout(), stdin());
        Server::new(io).run().await
    }
}

struct Server<T: Io> {
    ai: Ai,
    options: Options,
    position: Position,
    io: T,
}

impl<T: Io> Server<T> {
    fn new(io: T) -> Self {
        Server {
            ai: Ai::default(),
            options: Options::default(),
            position: Position::default(),
            io,
        }
    }

    fn new_game(&mut self) {
        self.ai = Ai::new(self.options)
    }

    fn set_hash(&mut self, value: &str) -> Result<(), Anyhow> {
        self.options = Options {
            hash: value
                .parse::<usize>()
                .context("invalid hash size")?
                .checked_shl(20)
                .unwrap_or(usize::MAX),
            ..self.options
        };

        Ok(())
    }

    fn set_threads(&mut self, value: &str) -> Result<(), Anyhow> {
        self.options = Options {
            threads: value
                .parse::<NonZeroUsize>()
                .context("invalid thread count")?,
            ..self.options
        };

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

                UciMessage::Position { fen, moves, .. } => {
                    match fen {
                        None => self.position = Position::default(),
                        Some(s) => match Ok::<_, Anyhow>(s).and_then(|s| {
                            let fen: Fen = s.as_str().parse().context("invalid fen")?;
                            fen.try_into().context("illegal fen")
                        }) {
                            Err(e) => warn!("ignored {:?}", e),
                            Ok(pos) => self.position = pos,
                        },
                    }

                    for m in moves {
                        if let Err(e) = self.position.make(m.into()) {
                            error!("{}", e);
                            break;
                        }
                    }
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
        let best = self.ai.play(&self.position, limits).await?;
        let msg = UciMessage::best_move(best.into());
        self.io.send(&msg.to_string()).await?;
        Ok(())
    }
}
