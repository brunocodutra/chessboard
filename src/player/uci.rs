use crate::{Action, Player, Position, Remote};
use anyhow::{anyhow, Context, Error as Anyhow};
use async_trait::async_trait;
use smol::block_on;
use std::error::Error;
use tracing::{debug, error, info, instrument, warn};
use vampirc_uci::{parse_one, UciFen, UciMessage};

#[derive(Debug)]
pub struct Uci<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    remote: R,
}

impl<R> Uci<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    #[instrument(skip(remote), err)]
    pub async fn init(mut remote: R) -> Result<Self, R::Error> {
        remote.send(UciMessage::Uci).await?;
        remote.flush().await?;

        loop {
            debug!("expecting 'uciok'");
            match parse_one(&remote.recv().await?) {
                UciMessage::UciOk => break,
                UciMessage::Id { name, author } => {
                    if let Some(engine) = name {
                        info!(?engine)
                    }

                    if let Some(author) = author {
                        info!(?author)
                    }
                }
                m => Self::ignore(m),
            }
        }

        Ok(Uci { remote })
    }

    fn ignore(msg: UciMessage) {
        let e = match msg {
            UciMessage::Unknown(m, cause) => {
                let error = anyhow!("ignoring invalid UCI message '{}'", m);
                match cause {
                    Some(cause) => Anyhow::from(cause).context(error),
                    None => error,
                }
            }

            msg => anyhow!("ignoring unexpected UCI message '{}'", msg),
        };

        warn!("{:?}", e);
    }
}

impl<R> Drop for Uci<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    #[instrument(skip(self))]
    fn drop(&mut self) {
        let result: Result<(), Anyhow> = block_on(async {
            self.remote.send(UciMessage::Stop).await?;
            self.remote.send(UciMessage::Quit).await?;
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
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(skip(self), err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        let setpos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(pos.to_string())),
            moves: Vec::new(),
        };

        self.remote.send(setpos).await?;
        self.remote.send(UciMessage::go()).await?;
        self.remote.flush().await?;

        let m = loop {
            debug!("expecting 'bestmove'");
            match parse_one(&self.remote.recv().await?) {
                UciMessage::BestMove { best_move: m, .. } => break m.into(),
                i @ UciMessage::Info(_) => debug!("{}", i),
                m => Self::ignore(m),
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
    use std::io;

    fn invalid_uci_message() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("valid uci message", |s| {
            matches!(parse_one(s), UciMessage::Unknown(_, _))
        })
    }

    fn unexpected_uci_message() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(UciMessage::UciNewGame),
            Just(UciMessage::ReadyOk),
            Just(UciMessage::Stop),
            Just(UciMessage::Quit),
            Just(UciMessage::PonderHit),
            any::<bool>().prop_map(UciMessage::Debug),
            any::<(Option<String>, Option<String>)>()
                .prop_map(|(name, author)| UciMessage::Id { name, author }),
            any::<(bool, Option<String>, Option<String>)>()
                .prop_map(|(later, name, code)| UciMessage::Register { later, name, code }),
            any::<(String, Option<String>)>()
                .prop_map(|(name, value)| UciMessage::SetOption { name, value }),
        ]
        .prop_map(|msg| msg.to_string())
    }

    proptest! {
        #[test]
        fn init_shakes_hand_with_engine(name: String, author: String) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Uci))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::id_name(&name).to_string()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::id_author(&author).to_string()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(UciMessage::UciOk.to_string()));

            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Stop))
                .returning(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(UciMessage::Quit))
                .returning(|_| Ok(()));

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

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_ingnores_unexpected_uci_messages(msg in unexpected_uci_message()) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(msg.clone()));

            remote.expect_recv().times(1)
                .returning(move || Ok(UciMessage::UciOk.to_string()));

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_can_fail(e: io::Error) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().times(1).return_once(move |_: UciMessage| Err(e));
            remote.expect_send().returning(|_: UciMessage| Ok(()));

            assert_eq!(block_on(Uci::init(remote)).err().unwrap().kind(), kind);
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
        fn play_ingnores_unexpected_uci_messages(pos: Position, m: Move, msg in unexpected_uci_message()) {
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
        fn play_can_fail_writing_to_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().times(1).return_once(move |_: UciMessage| Err(e));
            remote.expect_send().returning(|_: UciMessage| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_flushing_the_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: UciMessage| Ok(()));

            let kind = e.kind();
            remote.expect_flush().times(1).return_once(move || Err(e));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_reading_from_remote(pos: Position, e: io::Error) {
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
