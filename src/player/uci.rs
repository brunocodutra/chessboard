use crate::{Action, Player, Position, Remote};
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use smol::block_on;
use std::{error::Error, fmt::Debug};
use tracing::{debug, error, instrument, warn};
use vampirc_uci::{parse_one, Duration, UciFen, UciMessage, UciSearchControl, UciTimeControl};

#[derive(Debug)]
pub struct Uci<R>
where
    R: Remote + Debug,
    R::Error: Error + Send + Sync + 'static,
{
    remote: R,
}

impl<R> Uci<R>
where
    R: Remote + Debug,
    R::Error: Error + Send + Sync + 'static,
{
    /// Establishes communication with a remote UCI server.
    #[instrument(level = "trace", err)]
    pub async fn init(remote: R) -> Result<Self, R::Error> {
        let mut uci = Uci { remote };

        uci.remote.send(UciMessage::Uci).await?;
        uci.remote.flush().await?;

        while !matches!(uci.next_message().await?, UciMessage::UciOk) {}

        uci.remote.send(UciMessage::UciNewGame).await?;
        uci.remote.send(UciMessage::IsReady).await?;
        uci.remote.flush().await?;

        while !matches!(uci.next_message().await?, UciMessage::ReadyOk) {}

        Ok(uci)
    }

    #[instrument(level = "trace", err)]
    async fn next_message(&mut self) -> Result<UciMessage, R::Error> {
        loop {
            match parse_one(&self.remote.recv().await?) {
                UciMessage::Unknown(m, Some(cause)) => {
                    let error = Anyhow::from(cause).context(format!("invalid UCI message '{}'", m));
                    warn!("{:?}", error);
                }

                UciMessage::Unknown(m, None) => {
                    warn!("invalid UCI message '{}'", m);
                }

                msg => {
                    debug!(received = %msg);
                    return Ok(msg);
                }
            }
        }
    }
}

impl<R> Drop for Uci<R>
where
    R: Remote + Debug,
    R::Error: Error + Send + Sync + 'static,
{
    #[instrument(level = "trace")]
    fn drop(&mut self) {
        let result: Result<(), R::Error> = block_on(async {
            self.remote.send(UciMessage::Stop).await?;
            self.remote.send(UciMessage::Quit).await?;
            self.remote.flush().await?;
            Ok(())
        });

        if let Err(e) = result.context("failed to gracefully shutdown the uci engine") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl<R> Player for Uci<R>
where
    R: Remote + Debug + Send,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(level = "trace", err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        let setpos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(pos.to_string())),
            moves: Vec::new(),
        };

        let go = UciMessage::Go {
            time_control: Some(UciTimeControl::MoveTime(Duration::milliseconds(100))),
            search_control: Some(UciSearchControl::depth(13)),
        };

        self.remote.send(setpos).await?;
        self.remote.send(go).await?;
        self.remote.flush().await?;

        let m = loop {
            match self.next_message().await? {
                UciMessage::BestMove { best_move: m, .. } => break m.into(),
                _ => continue,
            }
        };

        Ok(Action::Move(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{remote::MockRemote, Move};
    use mockall::{predicate::*, Sequence};
    use proptest::prelude::*;
    use std::io::Error as IoError;

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

    fn invalid_uci_message() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("valid uci message", |s| {
            matches!(parse_one(s), UciMessage::Unknown(_, _))
        })
    }

    proptest! {
        #[test]
        fn init_shakes_hand_with_engine(_: ()) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Uci))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(UciMessage::UciOk.to_string()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::UciNewGame))
                .returning(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::IsReady))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(UciMessage::ReadyOk.to_string()));

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_ignores_invalid_uci_messages(msg in invalid_uci_message()) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(msg.clone()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::UciOk.to_string()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::ReadyOk.to_string()));

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_ingnores_unexpected_uci_messages(msg in any_uci_message()) {
            prop_assume!(!matches!(msg, UciMessage::UciOk));

            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(msg.to_string()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::UciOk.to_string()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::ReadyOk.to_string()));

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_can_fail(e: IoError) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().times(1).return_once(move |_: UciMessage| Err(e));
            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            assert_eq!(block_on(Uci::init(remote)).err().unwrap().kind(), kind);
        }

        #[test]
        fn drop_gracefully_stops_the_remote_engine(_: ()) {
            let mut remote = MockRemote::new();

            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Stop))
                .returning(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Quit))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            drop(Uci { remote });
        }

        #[test]
        fn drop_recovers_from_errors(e: IoError) {
            let mut remote = MockRemote::new();
            remote.expect_send().times(1).return_once(move |_: UciMessage| Err(e));
            drop(Uci { remote });
        }

        #[test]
        fn engine_can_make_a_move(pos: Position, m: Move) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(|msg: &UciMessage| matches!(msg, UciMessage::Position { .. })))
                .returning(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| matches!(msg, UciMessage::Go { .. })))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Stop))
                .returning(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Quit))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn play_ignores_invalid_uci_messages(pos: Position, m: Move, msg in invalid_uci_message()) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(msg.clone()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn play_ingnores_unexpected_uci_messages(pos: Position, m: Move, msg in any_uci_message()) {
            prop_assume!(!matches!(msg, UciMessage::BestMove { .. }));

            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(msg.to_string()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::best_move(m.into()).to_string()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn play_can_fail_writing(pos: Position, e: IoError) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().times(1).return_once(move |_: UciMessage| Err(e));
            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_flushing(pos: Position, e: IoError) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));

            let kind = e.kind();
            remote.expect_flush().times(1).return_once(move || Err(e));
            remote.expect_flush().returning(|| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_reading(pos: Position, e: IoError) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            let kind = e.kind();
            remote.expect_recv().times(1).return_once(move || Err(e));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap_err().kind(), kind);
        }
    }
}
