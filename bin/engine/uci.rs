use super::Player;
use crate::io::Io;
use anyhow::{Context, Error as Anyhow};
use async_stream::try_stream;
use derive_more::{DebugCustom, Display, Error, From};
use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::eval::Value;
use lib::search::{Depth, Limits, Pv};
use std::{collections::HashMap, fmt::Debug, future::Future, io, pin::Pin, time::Instant};
use tokio::{runtime, task::block_in_place, time::timeout};
use tracing::{debug, error, instrument};
use vampirc_uci::{self as uci, UciFen, UciInfoAttribute, UciMessage, UciSearchControl};

pub type UciOptions = HashMap<String, Option<String>>;

#[derive(DebugCustom)]
#[debug(bound = "T: Debug")]
#[debug(fmt = "Lazy({})")]
enum Lazy<T, E> {
    #[debug(fmt = "{_0:?}")]
    Initialized(T),
    #[debug(fmt = "?")]
    Uninitialized(Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>),
}

impl<T, E> Lazy<T, E> {
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

/// The reason why an [`Move`] could not be received from the UCI server.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the UCI server encountered an error")]
pub struct UciError(#[from(forward)] io::Error);

/// A Universal Chess Interface client for a chess engine.
#[derive(Debug)]
pub struct Uci<T: Io> {
    io: Lazy<T, UciError>,
    limits: Limits,
}

impl<T: Io + Send + 'static> Uci<T> {
    /// Constructs [`Uci`] with the given search [`Limits`] and [`UciOptions`].
    pub fn new(mut io: T, limits: Limits, options: UciOptions) -> Self {
        Uci {
            limits,
            io: Lazy::Uninitialized(Box::pin(async move {
                io.send(&UciMessage::Uci.to_string()).await?;
                io.flush().await?;

                while !matches!(uci::parse_one(io.recv().await?.trim()), UciMessage::UciOk) {}

                for (name, value) in options {
                    let set_option = UciMessage::SetOption { name, value };
                    io.send(&set_option.to_string()).await?;
                }

                io.send(&UciMessage::UciNewGame.to_string()).await?;
                io.send(&UciMessage::IsReady.to_string()).await?;
                io.flush().await?;

                while !matches!(uci::parse_one(io.recv().await?.trim()), UciMessage::ReadyOk) {}

                Ok(io)
            })),
        }
    }

    async fn go(&mut self, pos: &Position) -> Result<(), UciError> {
        let position = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(pos.to_string())),
            moves: Vec::new(),
        };

        let go = match self.limits {
            Limits::None => UciMessage::go(),
            Limits::Depth(d) => UciMessage::Go {
                search_control: Some(UciSearchControl::depth(d.get())),
                time_control: None,
            },
            Limits::Time(t) => UciMessage::go_movetime(
                uci::Duration::from_std(t).unwrap_or_else(|_| uci::Duration::max_value()),
            ),
        };

        let io = self.io.get_or_init().await?;
        io.send(&position.to_string()).await?;
        io.send(&go.to_string()).await?;
        io.flush().await?;

        Ok(())
    }
}

impl<T: Io> Drop for Uci<T> {
    #[instrument(level = "trace", skip(self))]
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
            error!("{:?}", e);
        }
    }
}

impl<T: Io + Send + 'static> Player for Uci<T> {
    type Error = UciError;

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos))]
    fn play<'a, 'b, 'c>(&'a mut self, pos: &'b Position) -> BoxFuture<'c, Result<Move, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(async move {
            self.go(pos).await?;

            let io = self.io.get_or_init().await?;

            loop {
                if let UciMessage::BestMove { best_move: m, .. } =
                    uci::parse_one(io.recv().await?.trim())
                {
                    break Ok(m.into());
                }
            }
        })
    }

    #[instrument(level = "debug", skip(self, pos), fields(%pos))]
    fn analyze<'a, 'b, 'c, const N: usize>(
        &'a mut self,
        pos: &'b Position,
    ) -> BoxStream<'c, Result<Pv<N>, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(try_stream! {
            self.go(pos).await?;
            let timer = Instant::now();
            let io = self.io.get_or_init().await?;

            loop {
                let limit = self.limits.time().saturating_sub(timer.elapsed());
                let msg = match timeout(limit, io.recv()).await {
                    Ok(msg) => msg?,
                    Err(_) => break,
                };

                if let UciMessage::Info(info) = uci::parse_one(msg.trim()) {
                    let (mut depth, mut score, mut moves) = Default::default();

                    for i in info {
                        if let UciInfoAttribute::Depth(d) = i {
                            depth = Some(d);
                        }

                        if let UciInfoAttribute::Score { mate: Some(d), .. } = i {
                            if d > 0 {
                                score = Some(Value::upper());
                            } else {
                                score = Some(Value::lower());
                            }
                        }

                        if let UciInfoAttribute::Score { cp: Some(s), .. } = i {
                            score = Some(Value::saturate(s));
                        }

                        if let UciInfoAttribute::Pv(m) = i {
                            moves = m.into_iter().map(|m| m.into()).collect::<Vec<_>>();
                        }
                    }

                    if let Some((d, s)) = Option::zip(depth, score) {
                        if self.limits.depth() < d {
                            break;
                        } else {
                            yield Pv::new(Depth::saturate(d), s, moves);
                            if self.limits.depth() == d {
                                break;
                            }
                        }
                    }
                }
            }

            io.send(&UciMessage::Stop.to_string()).await?;
            io.flush().await?;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::MockIo;
    use futures_util::TryStreamExt;
    use lib::chess::Move;
    use mockall::Sequence;
    use proptest::prelude::*;
    use std::{future::ready, time::Duration};
    use test_strategy::proptest;
    use tokio::{runtime, time::sleep};

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
    fn new_schedules_engine_for_lazy_initialization(l: Limits, o: UciOptions) {
        assert!(matches!(
            Uci::new(MockIo::new(), l, o),
            Uci {
                io: Lazy::Uninitialized(_),
                ..
            }
        ));
    }

    #[proptest]
    fn engine_is_lazily_initialized_with_the_options_configured(
        l: Limits,
        o: UciOptions,
        pos: Position,
        m: Move,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Uci.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Box::pin(ready(Ok(UciMessage::UciOk.to_string()))));

        for (name, value) in o.clone() {
            let set_option = UciMessage::SetOption { name, value };
            io.expect_send()
                .once()
                .in_sequence(&mut seq)
                .withf(move |msg| msg == set_option.to_string())
                .returning(|_| Box::pin(ready(Ok(()))));
        }

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::UciNewGame.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::IsReady.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Box::pin(ready(Ok(UciMessage::ReadyOk.to_string()))));

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));
        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci::new(io, l, o);

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn initialization_ignores_invalid_uci_messages(
        l: Limits,
        o: UciOptions,
        pos: Position,
        m: Move,
        #[by_ref]
        #[filter(matches!(uci::parse_one(#msg.trim()), UciMessage::Unknown(_, _)))]
        msg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Ok(msg))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::UciOk.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::ReadyOk.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci::new(io, l, o);

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn initialization_ignores_unexpected_uci_messages(
        l: Limits,
        o: UciOptions,
        pos: Position,
        m: Move,
        #[by_ref]
        #[filter(!matches!(#msg, UciMessage::UciOk))]
        #[strategy(any_uci_message())]
        msg: UciMessage,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Ok(msg.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::UciOk.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::ReadyOk.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci::new(io, l, o);

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn initialization_can_fail(l: Limits, o: UciOptions, pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let kind = e.kind();
        io.expect_send()
            .once()
            .return_once(move |_| Box::pin(ready(Err(e))));

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let mut uci = Uci::new(io, l, o);

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn drop_gracefully_quits_initialized_engine(l: Limits) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Stop.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Quit.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Box::pin(ready(Ok(()))));

        rt.block_on(async move {
            drop(Uci {
                io: Lazy::Initialized(io),
                limits: l,
            });
        })
    }

    #[proptest]
    fn drop_gracefully_quits_uninitialized_engine(l: Limits) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send()
            .times(3)
            .returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush()
            .times(2)
            .returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::UciOk.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::ReadyOk.to_string()))));

        let mut seq = Sequence::new();

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Stop.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(|msg| msg == UciMessage::Quit.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Box::pin(ready(Ok(()))));

        rt.block_on(async move {
            drop(Uci::new(io, l, UciOptions::default()));
        })
    }

    #[proptest]
    fn drop_recovers_from_errors(l: Limits, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        io.expect_send()
            .once()
            .return_once(move |_| Box::pin(ready(Err(e))));

        rt.block_on(async move {
            drop(Uci {
                io: Lazy::Initialized(io),
                limits: l,
            });
        })
    }

    #[proptest]
    fn drop_recovers_from_missing_runtime(l: Limits) {
        drop(Uci {
            io: Lazy::Initialized(MockIo::new()),
            limits: l,
        });
    }

    #[proptest]
    fn go_instructs_engine_search_for_best_move(l: Limits, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        let mut seq = Sequence::new();

        let p = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(pos.to_string())),
            moves: Vec::new(),
        };

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(move |msg| msg == p.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        let go = match l {
            Limits::None => UciMessage::go(),
            Limits::Depth(d) => UciMessage::Go {
                search_control: Some(UciSearchControl::depth(d.get())),
                time_control: None,
            },
            Limits::Time(t) => UciMessage::go_movetime(
                uci::Duration::from_std(t).unwrap_or_else(|_| uci::Duration::max_value()),
            ),
        };

        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(move |msg| msg == go.to_string())
            .returning(|_| Box::pin(ready(Ok(()))));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Box::pin(ready(Ok(()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.go(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(())
        );
    }

    #[proptest]
    fn go_can_fail_writing(l: Limits, pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let kind = e.kind();
        io.expect_send()
            .once()
            .return_once(move |_| Box::pin(ready(Err(e))));

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.go(&pos)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn go_can_fail_flushing(l: Limits, pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));

        let kind = e.kind();
        io.expect_flush()
            .once()
            .return_once(move || Box::pin(ready(Err(e))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.go(&pos)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn search_waits_for_engine_to_find_best_move(l: Limits, pos: Position, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn search_ignores_invalid_uci_messages(
        l: Limits,
        pos: Position,
        m: Move,
        #[by_ref]
        #[filter(matches!(uci::parse_one(#msg.trim()), UciMessage::Unknown(_, _)))]
        msg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Ok(msg))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn search_ignores_unexpected_uci_messages(
        l: Limits,
        pos: Position,
        m: Move,
        #[by_ref]
        #[filter(!matches!(#msg, UciMessage::BestMove { .. }))]
        #[strategy(any_uci_message())]
        msg: UciMessage,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Ok(msg.to_string()))));

        io.expect_recv()
            .once()
            .returning(move || Box::pin(ready(Ok(UciMessage::best_move(m.into()).to_string()))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Ok(m)
        );
    }

    #[proptest]
    fn search_can_fail_reading(l: Limits, pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let kind = e.kind();
        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Err(e))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn analyze_stops_when_target_depth_is_reached(pos: Position, pvs: Vec<Pv<4>>) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let (n, limits) = pvs
            .iter()
            .enumerate()
            .filter_map(|(i, pv)| Some((i, pv.depth()?)))
            .max_by_key(|(i, d)| (*d, !*i))
            .map(|(i, d)| (i + 1, Limits::Depth(d)))
            .unwrap_or_default();

        prop_assume!(limits != Limits::None);

        for pv in pvs.iter().take(n) {
            let mut attrs = vec![];

            if let Some((d, s)) = Option::zip(pv.depth(), pv.score()) {
                attrs.push(UciInfoAttribute::Depth(d.get()));
                attrs.push(UciInfoAttribute::from_centipawns(s.get().into()));
            }

            attrs.push(UciInfoAttribute::Pv(
                pv.iter().copied().map(|m| m.into()).collect(),
            ));

            let info = UciMessage::Info(attrs);

            io.expect_recv()
                .once()
                .return_once(move || Box::pin(ready(Ok(info.to_string()))));
        }

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits,
        };

        assert_eq!(
            rt.block_on(uci.analyze(&pos).try_collect::<Vec<_>>())
                .map_err(|UciError(e)| e.kind()),
            Ok(pvs
                .into_iter()
                .take(n)
                .filter(|pv| pv.depth().is_some())
                .collect())
        );
    }

    #[proptest]
    fn analyze_can_be_limited_by_time(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));
        io.expect_recv().returning(|| {
            Box::pin(async move {
                sleep(Duration::from_secs(1)).await;
                Ok(String::new())
            })
        });

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: Limits::Time(Duration::ZERO),
        };

        assert_eq!(
            rt.block_on(uci.analyze::<4>(&pos).try_collect::<Vec<_>>())
                .map_err(|UciError(e)| e.kind()),
            Ok(Vec::new())
        );
    }

    #[proptest]
    fn analyze_can_fail_reading(l: Limits, pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Box::pin(ready(Ok(()))));
        io.expect_flush().returning(|| Box::pin(ready(Ok(()))));

        let kind = e.kind();
        io.expect_recv()
            .once()
            .return_once(move || Box::pin(ready(Err(e))));

        let mut uci = Uci {
            io: Lazy::Initialized(io),
            limits: l,
        };

        assert_eq!(
            rt.block_on(uci.play(&pos)).map_err(|UciError(e)| e.kind()),
            Err(kind)
        );
    }
}
