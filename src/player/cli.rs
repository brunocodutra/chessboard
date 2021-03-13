use crate::*;
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use clap::AppSettings::*;
use derive_more::{Constructor, From};
use std::{error::Error, str::FromStr};
use structopt::StructOpt;
use tracing::*;

#[derive(Debug, StructOpt)]
#[structopt(
    author,
    name = "Chessboard",
    usage = "<SUBCOMMAND> [ARGS]",
    after_help = "See 'help <SUBCOMMAND>' for more information on a specific command.",
    global_settings = &[NoBinaryName, DisableVersion, DisableHelpFlags],
)]
enum CliSpec {
    #[structopt(about = "Resign the match in favor of the opponent", no_version)]
    Resign,

    #[structopt(
        about = "Moves a piece on the board",
        after_help = r#"SYNTAX:
    <descriptor>    ::= <square:from><square:to>[<promotion>]
    <square>        ::= <file><rank>
    <file>          ::= a|b|c|d|e|f|g|h
    <rank>          ::= 1|2|3|4|5|6|7|8
    <promotion>     ::= q|r|b|n"#,
        no_version
    )]
    Move {
        #[structopt(help = "A chess move in pure coordinate notation", parse(try_from_str = try_parse))]
        descriptor: Move,
    },
}

fn try_parse<T>(s: &str) -> Result<T, String>
where
    T: FromStr,
    Anyhow: From<T::Err>,
{
    s.parse().map_err(|e| format!("{:?}", Anyhow::from(e)))
}

#[derive(Debug, From, Constructor)]
pub struct Cli<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    remote: R,
}

#[async_trait]
impl<R> Player for Cli<R>
where
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(skip(self, pos), /*err*/)]
    async fn act(&mut self, pos: Position) -> Result<Action, Self::Error> {
        self.remote.send(pos.placement()).await?;

        let spec = loop {
            self.remote.flush().await?;
            let line = self.remote.recv().await?;

            match CliSpec::from_iter_safe(line.split_whitespace()) {
                Ok(s) => break s,
                Err(e) => self.remote.send(e).await?,
            };
        };

        match spec {
            CliSpec::Resign => Ok(Action::Resign),
            CliSpec::Move { descriptor } => Ok(Action::Move(descriptor)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::MockRemote;
    use mockall::{predicate::*, Sequence};
    use proptest::prelude::*;
    use smol::block_on;
    use std::io;

    proptest! {
        #[test]
        fn player_can_take_any_action(pos: Position, a: Action) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(a.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_can_resign(pos: Position, cmd in "\\s*resign\\s*") {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .return_once(move || Ok(cmd));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), Action::Resign);
        }

        #[test]
        fn player_can_make_a_move(pos: Position, m: Move, cmd in "\\s*move\\s*") {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(format!("{} {}", cmd, m)));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn resign_takes_no_arguments(pos: Position, a: Action, arg in "[^\\s]+") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            let mut cmd = Some(format!("resign {}", arg));
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn move_does_not_accept_invalid_descriptors(pos: Position, a: Action, m: Move, arg in "[^a-h]*") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            let mut cmd = Some(format!("move {}", arg));
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_can_ask_for_help(pos: Position, a: Action, cmd in "|help|resign|move") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .with(function(|&clap::Error { kind, .. }| kind == clap::ErrorKind::HelpDisplayed))
                .returning(|_| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            let mut help = Some(format!("help {}", cmd));
            remote.expect_recv().times(2)
                .returning(move || Ok(help.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_is_prompted_again_after_invalid_command(pos: Position, a: Action, cmds in "[^resign]+") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(pos.placement()))
                .returning(|_| Ok(()));

            let mut cmds: Vec<_> = cmds.split_whitespace().map(String::from).collect();
            remote.expect_send().times(cmds.len())
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(cmds.len() + 1)
                .returning(|| Ok(()));

            remote.expect_recv().times(cmds.len() + 1)
                .returning(move || Ok(cmds.pop().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn play_can_fail_writing_to_the_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().return_once(move |_: Placement| Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_flushing_the_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: Placement| Ok(()));

            let kind = e.kind();
            remote.expect_flush().return_once(move || Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_reading_from_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: Placement| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            let kind = e.kind();
            remote.expect_recv().return_once(move || Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().kind(), kind);
        }
    }
}
