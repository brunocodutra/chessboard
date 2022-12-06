use crate::io::{Io, Pipe};
use anyhow::{Context, Error as Anyhow};
use clap::Parser;
use lib::chess::{Fen, Position};
use lib::eval::Evaluator;
use lib::search::{Limits, Options, Searcher};
use std::num::NonZeroUsize;
use tokio::io::{stdin, stdout};
use tokio::task::block_in_place;
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
    options: Options,
    strategy: Searcher,
    position: Position,
    io: T,
}

impl<T: Io> Server<T> {
    fn new(io: T) -> Self {
        Server {
            options: Options::default(),
            strategy: Searcher::default(),
            position: Position::default(),
            io,
        }
    }

    fn new_game(&mut self) {
        self.strategy = Searcher::with_options(Evaluator::default(), self.options)
    }

    fn set_hash(&mut self, value: &str) -> Result<(), Anyhow> {
        self.options = Options {
            hash: value.parse::<usize>().context("invalid hash size")? * (1 << 20),
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
                    self.io.send(&UciMessage::UciOk.to_string()).await?;

                    let hash = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Hash".to_string(),
                        default: Some(Options::default().hash as _),
                        min: Some(0),
                        max: None,
                    });

                    self.io.send(&hash.to_string()).await?;

                    let thread = UciMessage::Option(UciOptionConfig::Spin {
                        name: "Threads".to_string(),
                        default: Some(Options::default().threads.get() as _),
                        min: Some(1),
                        max: None,
                    });

                    self.io.send(&thread.to_string()).await?;
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
                    let limits = match time.to_std() {
                        Ok(time) => Limits::Time(time),
                        Err(_) => Limits::None,
                    };

                    self.go(limits).await?;
                }

                UciMessage::Go {
                    time_control,
                    search_control:
                        Some(UciSearchControl {
                            depth: Some(depth),
                            search_moves,
                            mate,
                            nodes,
                        }),
                } => {
                    if let Some(ctrl) = time_control {
                        warn!("ignored time control {:#?}", ctrl);
                    }

                    if !search_moves.is_empty() {
                        let moves: Vec<_> = search_moves.iter().map(ToString::to_string).collect();
                        warn!("ignored request to limit search to [{}]", moves.join(","));
                    }

                    if let Some(n) = mate {
                        warn!("ignored request to search for mate in {} moves", n);
                    }

                    if let Some(n) = nodes {
                        warn!("ignored request to terminate the search after {} nodes", n);
                    }

                    self.go(Limits::Depth(depth)).await?;
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
        let pv = block_in_place(|| self.strategy.search::<1>(&self.position, limits));
        let best = *pv.first().context("no legal move found")?;
        let msg = UciMessage::best_move(best.into());
        self.io.send(&msg.to_string()).await?;
        Ok(())
    }
}
