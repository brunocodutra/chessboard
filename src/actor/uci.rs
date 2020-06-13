use crate::*;
use anyhow::{anyhow, Error as Failure};
use async_std::{io, prelude::*};
use async_trait::async_trait;
use derivative::Derivative;
use smol::block_on;
use vampirc_uci::{parse_one, UciFen, UciMessage};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Uci<R: Remote> {
    #[derivative(Debug = "ignore")]
    remote: R,
}

impl<R> Uci<R>
where
    R: Remote,
    Failure: From<R::Error>,
{
    pub async fn init(mut remote: R) -> Result<Self, Failure> {
        remote.send(UciMessage::Uci).await?;

        loop {
            let answer = remote.recv().await?;
            match parse_one(&answer) {
                UciMessage::UciOk => break,
                m => Self::ignore(m).await?,
            }
        }

        Ok(Uci { remote })
    }
}

impl<R: Remote> Uci<R> {
    async fn ignore(msg: UciMessage) -> Result<(), io::Error> {
        let error = match msg {
            UciMessage::Unknown(m, cause) => {
                let error = anyhow!("warn: ignoring invalid UCI command '{}'", m);
                match cause {
                    Some(cause) => Into::<Failure>::into(cause).context(error),
                    None => error,
                }
            }

            msg => anyhow!("warn: ignoring unexpected UCI command '{}'", msg),
        };

        let warning = format!("{:?}\n", error);
        io::stderr().write_all(warning.as_bytes()).await
    }
}

impl<R: Remote> Drop for Uci<R> {
    fn drop(&mut self) {
        if block_on(self.remote.send(UciMessage::Stop)).is_err()
            || block_on(self.remote.send(UciMessage::Quit)).is_err()
        {
            eprintln!("warn: failed to quit the uci engine");
        }
    }
}

#[async_trait]
impl<R> Actor for Uci<R>
where
    R: Remote + Send + Sync,
    Failure: From<R::Error>,
{
    type Error = Failure;

    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error> {
        let setpos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(p.to_string())),
            moves: Vec::new(),
        };

        let go = UciMessage::Go {
            time_control: None,
            search_control: None,
        };

        self.remote.send(setpos).await?;
        self.remote.send(go).await?;

        let m = loop {
            let answer = self.remote.recv().await?;
            match parse_one(&answer) {
                UciMessage::BestMove { best_move: m, .. } => break m.into(),
                m => Self::ignore(m).await?,
            }
        };

        Ok(PlayerAction::MakeMove(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockRemote;
    use mockall::{predicate::*, Sequence};
    use proptest::prelude::*;
    use smol::block_on;

    fn unexpected_uci_command() -> impl Strategy<Value = UciMessage> {
        prop_oneof![
            Just(UciMessage::UciOk),
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
    }

    proptest! {
        #[test]
        fn init_shakes_hand_with_engine(name: String, author: String) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(UciMessage::Uci))
                .return_once(|_| Ok(()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(UciMessage::id_name(&name).to_string()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(UciMessage::id_author(&author).to_string()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(UciMessage::UciOk.to_string()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            assert!(block_on(Uci::init(remote)).is_ok());
        }

        #[test]
        fn init_can_fail(e: String) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(UciMessage::Uci))
                .return_once(|_| Ok(()));

            let failure = anyhow!(e.clone());
            remote.expect_recv().times(1)
                .return_once(move || Err(failure));

            assert_eq!(block_on(Uci::init(remote)).unwrap_err().to_string(), e);
        }

        #[test]
        fn engine_can_make_a_move(pos: Position, m: Move) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            let fen = pos.to_string();
            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().contains(&fen)))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().starts_with("go")))
                .return_once(|_| Ok(()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(format!("bestmove {}", m)));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(pos)).unwrap(), PlayerAction::MakeMove(m));
        }

        #[test]
        fn act_ignores_invalid_uci_commands(pos: Position, m: Move, cmd in "[^bestmove]+") {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            let fen = pos.to_string();
            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().contains(&fen)))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().starts_with("go")))
                .return_once(|_| Ok(()));

            let mut cmd = Some(cmd);
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| format!("bestmove {}", m))));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(pos)).unwrap(), PlayerAction::MakeMove(m));
        }

        #[test]
        fn act_ingnores_unexpected_uci_commands(pos: Position, m: Move, cmd in unexpected_uci_command()) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            let fen = pos.to_string();
            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().contains(&fen)))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(function(move |msg: &UciMessage| msg.to_string().starts_with("go")))
                .return_once(|_| Ok(()));

            let mut cmd = Some(cmd.to_string());
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| format!("bestmove {}", m))));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(pos)).unwrap(), PlayerAction::MakeMove(m));
        }

        #[test]
        fn act_can_fail_writing_to_remote(pos: Position, e: String) {
            let mut remote = MockRemote::new();
            let failure = anyhow!(e.clone());

            remote.expect_send().times(1)
                .return_once(move |_: UciMessage| Err(failure));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(pos)).unwrap_err().to_string(), e);
        }

        #[test]
        fn act_can_fail_reading_from_remote(pos: Position, e: String) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(2)
                .returning(|_: UciMessage| Ok(()));

            let failure = anyhow!(e.clone());
            remote.expect_recv().return_once(move || Err(failure));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Stop))
                .return_once(|_| Ok(()));

            remote.expect_send().times(1)
                .with(eq(UciMessage::Quit))
                .return_once(|_| Ok(()));

            let mut uci = Uci { remote };
            assert_eq!(block_on(uci.act(pos)).unwrap_err().to_string(), e);
        }
    }
}
