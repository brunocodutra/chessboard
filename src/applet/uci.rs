use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::{Build, Fen, Io, Pipe, Position, Search, SearchLimits, Strategy, StrategyBuilder};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use tokio::io::{stdin, stdout};
use tokio::task::block_in_place;
use tracing::{debug, error, instrument, warn};
use vampirc_uci::{self as uci, UciMessage, UciSearchControl, UciTimeControl};

/// A basic *not fully compliant* UCI server.
#[derive(Debug, Default, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Uci {
    /// The search strategy.
    #[clap(short, long, default_value_t)]
    strategy: StrategyBuilder,
}

#[async_trait]
impl Execute for Uci {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let strategy = self.strategy.build()?;
        let io = Pipe::new(stdout(), stdin());
        Server::new(strategy, io).run().await
    }
}

struct Server<T: Io> {
    position: Position,
    strategy: Strategy,
    io: T,
}

impl<T: Io> Server<T> {
    fn new(strategy: Strategy, io: T) -> Self {
        Server {
            position: Position::default(),
            strategy,
            io,
        }
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
                }

                UciMessage::Quit => break Ok(()),
                UciMessage::UciNewGame => self.strategy.clear(),
                UciMessage::IsReady => self.io.send(&UciMessage::ReadyOk.to_string()).await?,
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
                    self.go(SearchLimits::None).await?;
                }

                UciMessage::Go {
                    time_control: Some(UciTimeControl::MoveTime(time)),
                    search_control: None,
                } => {
                    let limits = match time.to_std() {
                        Ok(time) => SearchLimits::Time(time),
                        Err(_) => SearchLimits::None,
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

                    self.go(SearchLimits::Depth(depth)).await?;
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

    async fn go(&mut self, limits: SearchLimits) -> Result<(), Anyhow> {
        let pv = block_in_place(|| self.strategy.search::<1>(&self.position, limits));
        let best = *pv.first().context("no legal move found")?;
        let msg = UciMessage::best_move(best.into());
        self.io.send(&msg.to_string()).await?;
        Ok(())
    }
}
