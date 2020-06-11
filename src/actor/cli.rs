use crate::*;
use anyhow::Error as Failure;
use async_trait::async_trait;
use clap::{App, AppSettings, Arg, SubCommand};

pub struct Cli<T: Remote> {
    terminal: T,
    player: Player,
}

impl<T: Remote> Cli<T> {
    pub fn new(terminal: T, player: Player) -> Self {
        Cli { terminal, player }
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
                                    .map_err(|e| format!("{:?}", Failure::from(e)))
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
impl<T> Actor for Cli<T>
where
    T: Remote + Send + Sync,
    Failure: From<T::Error>,
{
    type Error = Failure;

    async fn act(&mut self, p: Position) -> Result<PlayerAction, Failure> {
        self.terminal.send(p).await?;

        let matches = loop {
            let line = self.terminal.recv().await?;
            let args = Self::spec().get_matches_from_safe(line.split_whitespace());

            match args {
                Ok(m) => break m,
                Err(e) => self.terminal.send(e).await?,
            };
        };

        let action = match matches.subcommand() {
            ("resign", _) => PlayerAction::Resign(self.player),

            ("move", Some(args)) => {
                let descriptor = args.value_of("descriptor").expect("missing required arg");
                PlayerAction::MakeMove(self.player, descriptor.parse().unwrap())
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
    use anyhow::anyhow;
    use mockall::predicate::*;
    use proptest::prelude::*;
    use smol::block_on;

    proptest! {
        #[test]
        fn player_can_resign(p: Player, pos: Position, cmd in "\\s*resign\\s*") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().times(1).with(eq(pos))
                .return_once(|_| Ok(()));

            terminal.expect_recv().times(1)
                .return_once(move || Ok(cmd));

            let mut cli = Cli::new(terminal, p);
            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::Resign(p));
        }

        #[test]
        fn player_can_make_a_move(p: Player, pos: Position, m: Move, cmd in "\\s*move\\s*") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().times(1).with(eq(pos))
                .return_once(|_| Ok(()));

            terminal.expect_recv().times(1)
                .return_once(move || Ok(format!("{} {}", cmd, m)));

            let mut cli = Cli::new(terminal, p);
            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::MakeMove(p, m));
        }

        #[test]
        fn resign_takes_no_arguments(p: Player, pos: Position, arg in "[^\\s]+") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().with(eq(pos))
                .returning(|_| Ok(()));

            terminal.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("resign {}", arg));
            terminal.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or_else(|| "resign".into())));

            let mut cli = Cli::new(terminal, p);
            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn move_does_not_accept_invalid_descriptors(p: Player, pos: Position, m: Move, arg in "[^a-h]*") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().with(eq(pos))
                .returning(|_| Ok(()));

            terminal.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("move {}", arg));
            terminal.expect_recv().times(2)
                .returning(move || Ok(cmd.take().unwrap_or(format!("move {}", m))));

            let mut cli = Cli::new(terminal, p);
            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn player_can_ask_for_help(p: Player, pos: Position, cmd in "|help|resign|move") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().with(eq(pos))
                .returning(|_| Ok(()));

            terminal.expect_send().times(1)
                .with(function(|&clap::Error { kind, .. }| kind == clap::ErrorKind::HelpDisplayed))
                .returning(|_| Ok(()));

            let mut help = Some(format!("help {}", cmd));
            terminal.expect_recv().times(2)
                .returning(move || Ok(help.take().unwrap_or_else(|| "resign".into())));

            let mut cli = Cli::new(terminal, p);
            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn player_is_prompted_again_after_invalid_command(p: Player, pos: Position, cmds in "[^resign]+") {
            let mut terminal = MockRemote::new();

            terminal.expect_send().with(eq(pos))
                .returning(|_| Ok(()));

            let mut cmds: Vec<_> = cmds.split_whitespace().map(String::from).collect();
            terminal.expect_send().times(cmds.len())
                .returning(|_: clap::Error| Ok(()));

            terminal.expect_recv().times(cmds.len() + 1)
                .returning(move || Ok(cmds.pop().unwrap_or_else(|| "resign".into())));

            let mut cli = Cli::new(terminal, p);
            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn writing_to_terminal_can_fail(p: Player, pos: Position, e: String) {
            let mut terminal = MockRemote::new();
            let failure = anyhow!(e.clone());
            terminal.expect_send().times(1).with(eq(pos))
                .return_once(move |_| Err(failure));

            let mut cli = Cli::new(terminal, p);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().to_string(), e);
        }

        #[test]
        fn reading_from_terminal_can_fail(p: Player, pos: Position, e: String) {
            let mut terminal = MockRemote::new();

            terminal.expect_send().with(eq(pos))
                .returning(|_| Ok(()));

            let failure = anyhow!(e.clone());
            terminal.expect_recv().times(1)
                .return_once(move || Err(failure));

            let mut cli = Cli::new(terminal, p);
            assert_eq!(block_on(cli.act(pos)).unwrap_err().to_string(), e);
        }
    }
}
