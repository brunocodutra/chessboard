use crate::*;
use async_trait::async_trait;
use clap::{App, AppSettings, Arg, SubCommand};
use derivative::Derivative;
use std::error::Error;
use tracing::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Cli<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    #[derivative(Debug = "ignore")]
    remote: R,
}

impl<R> Cli<R>
where
    R: Remote,
    R::Error: Error + Send + Sync + 'static,
{
    pub fn new(remote: R) -> Self {
        Cli { remote }
    }

    fn spec() -> App<'static, 'static> {
        App::new("chessboard")
            .setting(AppSettings::NoBinaryName)
            .setting(AppSettings::DisableVersion)
            .setting(AppSettings::VersionlessSubcommands)
            .setting(AppSettings::SubcommandRequired)
            .usage("<SUBCOMMAND> [ARGS]")
            .after_help("See 'help <SUBCOMMAND>' for more information on a specific command.")
            .subcommand(
                SubCommand::with_name("resign").about("Resign the match in favor of the opponent"),
            )
            .subcommand(
                SubCommand::with_name("move")
                    .about("Moves a piece on the board")
                    .arg(
                        Arg::with_name("descriptor")
                            .help("A chess move in pure coordinate notation")
                            .required(true)
                            .validator(|d| {
                                d.parse()
                                    .map(|_: Move| ())
                                    .map_err(|e| format!("{:?}", anyhow::Error::from(e)))
                            }),
                    )
                    .after_help(
                        r#"SYNTAX:
    <descriptor>    ::= <square:from><square:to>[<promotion>]
    <square>        ::= <file><rank>
    <file>          ::= a|b|c|d|e|f|g|h
    <rank>          ::= 1|2|3|4|5|6|7|8
    <promotion>     ::= q|r|b|n"#,
                    ),
            )
    }
}

#[async_trait]
impl<R> Actor for Cli<R>
where
    R: Remote + Send + Sync,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(skip(self, p), err)]
    async fn act(&mut self, p: Position) -> Result<PlayerAction, Self::Error> {
        self.remote.send(p.placement()).await?;

        let matches = loop {
            let line = self.remote.recv().await?;
            let args = Self::spec().get_matches_from_safe(line.split_whitespace());

            match args {
                Ok(m) => break m,
                Err(e) => self.remote.send(e).await?,
            };
        };

        let action = match matches.subcommand() {
            ("resign", _) => PlayerAction::Resign,

            ("move", Some(args)) => {
                let descriptor = args.value_of("descriptor").expect("missing required arg");
                PlayerAction::MakeMove(descriptor.parse().unwrap())
            }

            (cmd, _) => panic!("unexpected subcommand '{}'", cmd),
        };

        Ok(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockRemote;
    use mockall::predicate::*;
    use proptest::prelude::*;
    use smol::block_on;
    use std::io;

    proptest! {
        #[test]
        fn player_can_make_any_action(pos: Position, a: PlayerAction) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .return_once(|_| Ok(()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(a.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_can_resign(pos: Position, cmd in "\\s*resign\\s*") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .return_once(|_| Ok(()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(cmd));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::Resign);
        }

        #[test]
        fn player_can_make_a_move(pos: Position, m: Move, cmd in "\\s*move\\s*") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .return_once(|_| Ok(()));

            remote.expect_recv().times(1)
                .return_once(move || Ok(format!("{} {}", cmd, m)));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::MakeMove(m));
        }

        #[test]
        fn resign_takes_no_arguments(pos: Position, a: PlayerAction, arg in "[^\\s]+") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("resign {}", arg));
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn move_does_not_accept_invalid_descriptors(pos: Position, a: PlayerAction, m: Move, arg in "[^a-h]*") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("move {}", arg));
            remote.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_can_ask_for_help(pos: Position, a: PlayerAction, cmd in "|help|resign|move") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .with(function(|&clap::Error { kind, .. }| kind == clap::ErrorKind::HelpDisplayed))
                .returning(|_| Ok(()));

            let mut help = Some(format!("help {}", cmd));
            remote.expect_recv().times(2)
                .returning(move || Ok(help.take().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn player_is_prompted_again_after_invalid_command(pos: Position, a: PlayerAction, cmds in "[^resign]+") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1).with(eq(pos.placement()))
                .returning(|_| Ok(()));

            let mut cmds: Vec<_> = cmds.split_whitespace().map(String::from).collect();
            remote.expect_send().times(cmds.len())
                .returning(|_: clap::Error| Ok(()));

            remote.expect_recv().times(cmds.len() + 1)
                .returning(move || Ok(cmds.pop().unwrap_or_else(|| a.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap(), a);
        }

        #[test]
        fn writing_to_remote_can_fail(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().return_once(move |_: Placement| Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn reading_from_remote_can_fail(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: Placement| Ok(()));

            let kind = e.kind();
            remote.expect_recv().return_once(move || Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().kind(), kind);
        }
    }
}
