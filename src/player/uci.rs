use crate::{Act, Action, Game, Io};
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use std::{fmt::Debug, future::Future, io, pin::Pin};
use tokio::{runtime, task::block_in_place};
use tracing::{debug, instrument, warn};
use vampirc_uci::{parse_one, Duration, UciFen, UciMessage, UciSearchControl, UciTimeControl};

#[derive(DebugCustom)]
#[debug(fmt = "Lazy({})")]
enum Lazy<T: Debug, E> {
    #[debug(fmt = "{:?}", _0)]
    Initialized(T),
    #[debug(fmt = "?")]
    Uninitialized(Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>),
}

impl<T: Debug, E> Lazy<T, E> {
    async fn get_or_init(&mut self) -> Result<&mut T, E> {
        match self {
            Lazy::Initialized(v) => Ok(v),
            Lazy::Uninitialized(f) => {
                *self = Lazy::Initialized(f.await?);
                match self {
                    Lazy::Initialized(v) => Ok(v),
                    Lazy::Uninitialized(_) => unreachable!(),
                }
            }
        }
    }
}

/// The reason why an [`Action`] could not be received from the UCI server.
#[derive(Debug, Display, Error, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "the UCI server encountered an error")]
pub struct UciError(#[from(forward)] io::Error);

/// A Universal Chess Interface client for a computer controlled player.
#[derive(Debug)]
pub struct Uci<T: Io + Debug> {
    io: Lazy<T, UciError>,
}

impl<T: Io + Debug + Send + 'static> Uci<T> {
    pub fn new(mut io: T) -> Self {
        Uci {
            io: Lazy::Uninitialized(Box::pin(async move {
                io.send(&UciMessage::Uci.to_string()).await?;
                io.flush().await?;

                while !matches!(recv_uci_message(&mut io).await?, UciMessage::UciOk) {}

                io.send(&UciMessage::UciNewGame.to_string()).await?;
                io.send(&UciMessage::IsReady.to_string()).await?;
                io.flush().await?;

                while !matches!(recv_uci_message(&mut io).await?, UciMessage::ReadyOk) {}

                Ok(io)
            })),
        }
    }
}

impl<T: Io + Debug> Drop for Uci<T> {
    #[instrument(level = "trace")]
    fn drop(&mut self) {
        let result: Result<(), Anyhow> = block_in_place(|| {
            runtime::Handle::try_current()?.block_on(async {
                let io = self.io.get_or_init().await?;
                io.send(&UciMessage::Stop.to_string()).await?;
                io.send(&UciMessage::Quit.to_string()).await?;
                io.flush().await?;
                Ok(())
            })
        });

        if let Err(e) = result.context("failed to gracefully shutdown the uci engine") {
            warn!("{:?}", e);
        }
    }
}

#[async_trait]
impl<T: Io + Debug + Send> Act for Uci<T> {
    type Error = UciError;

    /// Request an action from the CLI server.
    #[instrument(level = "trace", err, ret)]
    async fn act(&mut self, game: &Game) -> Result<Action, Self::Error> {
        let position = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(game.position().to_string())),
            moves: Vec::new(),
        };

        let go = UciMessage::Go {
            time_control: Some(UciTimeControl::MoveTime(Duration::milliseconds(100))),
            search_control: Some(UciSearchControl::depth(13)),
        };

        let io = self.io.get_or_init().await?;
        io.send(&position.to_string()).await?;
        io.send(&go.to_string()).await?;
        io.flush().await?;

        let m = loop {
            match recv_uci_message(io).await? {
                UciMessage::BestMove { best_move: m, .. } => break m.into(),
                _ => continue,
            }
        };

        Ok(Action::Move(m))
    }
}

#[instrument(level = "trace", err, ret)]
async fn recv_uci_message<T: Io + Debug>(io: &mut T) -> Result<UciMessage, UciError> {
    loop {
        match parse_one(io.recv().await?.trim()) {
            UciMessage::Unknown(m, cause) => {
                let error = cause.map(Anyhow::new).unwrap_or_else(|| Anyhow::msg(m));
                debug!("{:?}", error.context("failed to parse UCI message"));
            }

            msg => {
                debug!(received = %msg);
                return Ok(msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockIo, Move};
    use mockall::Sequence;
    use proptest::prelude::*;
    use test_strategy::proptest;
    use tokio::runtime;

    fn any_uci_message() -> impl Strategy<Value = UciMessage> {
        prop_oneof![
            Just(UciMessage::Uci),
            Just(UciMessage::UciOk),
            Just(UciMessage::UciNewGame),
            Just(UciMessage::IsReady),
            Just(UciMessage::ReadyOk),
            Just(UciMessage::Stop),
            Just(UciMessage::Quit),
            Just(UciMessage::PonderHit),
            any::<(Move, Option<Move>)>().prop_map(|(m, p)| UciMessage::BestMove {
                best_move: m.into(),
                ponder: p.map(Into::into),
            }),
            any::<(Option<String>, Option<String>)>()
                .prop_map(|(name, author)| UciMessage::Id { name, author }),
            any::<(bool, Option<String>, Option<String>)>()
                .prop_map(|(later, name, code)| UciMessage::Register { later, name, code }),
            any::<(String, Option<String>)>()
                .prop_map(|(name, value)| UciMessage::SetOption { name, value }),
            any::<bool>().prop_map(UciMessage::Debug),
        ]
    }

    #[proptest]
    fn new_schedules_engine_for_lazy_initialization() {
        let io = MockIo::new();
        assert!(matches!(Uci::new(io).io, Lazy::Uninitialized(_)));
    }

    #[proptest]
    fn engine_is_lazily_initialized(g: Game, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Uci.to_string())
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Ok(UciMessage::UciOk.to_string()));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::UciNewGame.to_string())
            .returning(|_| Ok(()));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::IsReady.to_string())
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Ok(UciMessage::ReadyOk.to_string()));

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));
        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci::new(io);
        assert!(rt.block_on(uci.act(&g)).is_ok());
    }

    #[proptest]
    fn initialization_ignores_invalid_uci_messages(
        g: Game,
        m: Move,
        #[by_ref]
        #[filter(matches!(parse_one(#msg.trim()), UciMessage::Unknown(_, _)))]
        msg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv().once().returning(move || Ok(msg.clone()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::UciOk.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::ReadyOk.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci::new(io);
        assert!(rt.block_on(uci.act(&g)).is_ok());
    }

    #[proptest]
    fn initialization_ignores_unexpected_uci_messages(
        g: Game,
        m: Move,
        #[by_ref]
        #[filter(!matches!(#msg, UciMessage::UciOk))]
        #[strategy(any_uci_message())]
        msg: UciMessage,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(msg.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::UciOk.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::ReadyOk.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci::new(io);
        assert!(rt.block_on(uci.act(&g)).is_ok());
    }

    #[proptest]
    fn initialization_can_fail(g: Game, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let kind = e.kind();
        io.expect_send().once().return_once(move |_| Err(e));

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        let mut uci = Uci::new(io);
        assert_eq!(
            rt.block_on(uci.act(&g)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn drop_gracefully_quits_initialized_engine() {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Stop.to_string())
            .returning(|_| Ok(()));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Quit.to_string())
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        rt.block_on(async move {
            drop(Uci {
                io: Lazy::Initialized(io),
            });
        })
    }

    #[proptest]
    fn drop_gracefully_quits_uninitialized_engine() {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().times(3).returning(|_| Ok(()));
        io.expect_flush().times(2).returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::UciOk.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::ReadyOk.to_string()));

        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Stop.to_string())
            .returning(|_| Ok(()));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Quit.to_string())
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        rt.block_on(async move {
            drop(Uci::new(io));
        })
    }

    #[proptest]
    fn drop_recovers_from_errors(e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        io.expect_send().once().return_once(move |_| Err(e));

        rt.block_on(async move {
            drop(Uci {
                io: Lazy::Initialized(io),
            });
        })
    }

    #[proptest]
    fn drop_recovers_from_missing_runtime() {
        drop(Uci {
            io: Lazy::Initialized(MockIo::new()),
        });
    }

    #[proptest]
    fn play_instructs_engine_to_make_move(g: Game, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| matches!(parse_one(msg.trim()), UciMessage::Position { .. }))
            .returning(|_| Ok(()));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| matches!(parse_one(msg.trim()), UciMessage::Go { .. }))
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(rt.block_on(uci.act(&g))?, Action::Move(m));
    }

    #[proptest]
    fn play_ignores_invalid_uci_messages(
        g: Game,
        m: Move,
        #[by_ref]
        #[filter(matches!(parse_one(#msg.trim()), UciMessage::Unknown(_, _)))]
        msg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv().once().returning(move || Ok(msg.clone()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(rt.block_on(uci.act(&g))?, Action::Move(m));
    }

    #[proptest]
    fn play_ignores_unexpected_uci_messages(
        g: Game,
        m: Move,
        #[by_ref]
        #[filter(!matches!(#msg, UciMessage::BestMove { .. }))]
        #[strategy(any_uci_message())]
        msg: UciMessage,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(msg.to_string()));

        io.expect_recv()
            .once()
            .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(rt.block_on(uci.act(&g))?, Action::Move(m));
    }

    #[proptest]
    fn play_can_fail_writing(g: Game, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let kind = e.kind();
        io.expect_send().once().return_once(move |_| Err(e));

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(
            rt.block_on(uci.act(&g)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn play_can_fail_flushing(g: Game, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));

        let kind = e.kind();
        io.expect_flush().once().return_once(move || Err(e));
        io.expect_flush().returning(|| Ok(()));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(
            rt.block_on(uci.act(&g)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn play_can_fail_reading(g: Game, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        let kind = e.kind();
        io.expect_recv().once().return_once(move || Err(e));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
        };

        assert_eq!(
            rt.block_on(uci.act(&g)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }
}
