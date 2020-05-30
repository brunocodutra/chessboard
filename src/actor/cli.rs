use crate::*;
use anyhow::Error as Failure;
use async_trait::async_trait;
use clap::{App, AppSettings, Arg, SubCommand};
use std::fmt::Display;

#[cfg(test)]
mockall::mock! {
    Terminal {
        async fn read<D: Display + Send + 'static>(&self, prompt: D) -> Result<String, Failure>;
        async fn write<D: Display + Send + 'static>(&self, line: D) -> Result<(), Failure>;
    }
}

#[cfg(test)]
use MockTerminal as Terminal;

#[cfg(not(test))]
use async_std::{io::stdout, prelude::*, sync::*};

#[cfg(not(test))]
struct Terminal(Arc<Mutex<rustyline::Editor<()>>>);

#[cfg(not(test))]
impl Terminal {
    fn new() -> Self {
        use rustyline::{Config, Editor};
        Terminal(Arc::new(Mutex::new(Editor::<()>::with_config(
            Config::builder().auto_add_history(true).build(),
        ))))
    }

    async fn read<D: Display + Send + 'static>(&self, prompt: D) -> Result<String, Failure> {
        let editor = self.0.clone();
        let line = smol::blocking!(editor.lock().await.readline(&format!("{} > ", prompt)))?;
        Ok(line)
    }

    async fn write<D: Display + Send + 'static>(&self, line: D) -> Result<(), Failure> {
        stdout().write_all(format!("{}\n", line).as_bytes()).await?;
        Ok(())
    }
}

pub struct Cli {
    player: Player,
    terminal: Terminal,
}

impl Cli {
    pub fn new(player: Player) -> Self {
        Cli {
            player,
            terminal: Terminal::new(),
        }
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
impl Actor for Cli {
    type Error = Failure;

    async fn act(&mut self, p: Position) -> Result<PlayerAction, Failure> {
        self.terminal.write(p).await?;

        let matches = loop {
            let line = self.terminal.read(self.player.color).await?;
            let args = Cli::spec().get_matches_from_safe(line.split_whitespace());

            match args {
                Ok(m) => break m,
                Err(e) => self.terminal.write(e).await?,
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
    use anyhow::anyhow;
    use mockall::predicate::*;
    use proptest::prelude::*;
    use smol::block_on;

    proptest! {
        #[test]
        fn player_can_resign(p: Player, pos: Position, cmd in "\\s*resign\\s*") {
            let mut cli = Cli::new(p);

            cli.terminal.expect_write().times(1).with(eq(pos))
                .return_once(|_| Ok(()));

            cli.terminal.expect_read().times(1).with(eq(p.color))
                .return_once(move |_| Ok(cmd));

            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::Resign(p));
        }

        #[test]
        fn player_can_make_a_move(p: Player, pos: Position, m: Move, cmd in "\\s*move\\s*") {
            let mut cli = Cli::new(p);

            cli.terminal.expect_write().times(1).with(eq(pos))
                .return_once(|_| Ok(()));

            cli.terminal.expect_read().times(1).with(eq(p.color))
                .return_once(move |_| Ok(format!("{} {}", cmd, m)));

            assert_eq!(block_on(cli.act(pos)).unwrap(), PlayerAction::MakeMove(p, m));
        }

        #[test]
        fn resign_takes_no_arguments(p: Player, pos: Position, arg in "[^\\s]+") {
            let mut cli = Cli::new(p);
            cli.terminal.expect_write().with(eq(pos))
                .returning(|_| Ok(()));

            cli.terminal.expect_write().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("resign {}", arg));
            cli.terminal.expect_read().times(2).with(eq(p.color))
                .returning(move |_| Ok(cmd.take().unwrap_or_else(|| "resign".into())));

            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn move_does_not_accept_invalid_descriptors(p: Player, pos: Position, m: Move, arg in "[^a-h]*") {
            let mut cli = Cli::new(p);
            cli.terminal.expect_write().with(eq(pos))
                .returning(|_| Ok(()));

            cli.terminal.expect_write().times(1)
                .returning(|_: clap::Error| Ok(()));

            let mut cmd = Some(format!("move {}", arg));
            cli.terminal.expect_read().times(2).with(eq(p.color))
                .returning(move |_| Ok(cmd.take().unwrap_or(format!("move {}", m))));

            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn player_can_ask_for_help(p: Player, pos: Position, cmd in "|help|resign|move") {
            let mut cli = Cli::new(p);
            cli.terminal.expect_write().with(eq(pos))
                .returning(|_| Ok(()));

            cli.terminal.expect_write().times(1)
                .with(function(|&clap::Error { kind, .. }| kind == clap::ErrorKind::HelpDisplayed))
                .returning(|_| Ok(()));

            let mut help = Some(format!("help {}", cmd));
            cli.terminal.expect_read().times(2).with(eq(p.color))
                .returning(move |_| Ok(help.take().unwrap_or_else(|| "resign".into())));

            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn player_is_prompted_again_after_invalid_command(p: Player, pos: Position, cmds in "[^resign]+") {
            let mut cli = Cli::new(p);
            cli.terminal.expect_write().with(eq(pos))
                .returning(|_| Ok(()));

            let mut cmds: Vec<_> = cmds.split_whitespace().map(String::from).collect();
            cli.terminal.expect_write().times(cmds.len())
                .returning(|_: clap::Error| Ok(()));

            cli.terminal.expect_read().times(cmds.len() + 1).with(eq(p.color))
                .returning(move |_| Ok(cmds.pop().unwrap_or_else(|| "resign".into())));

            assert!(block_on(cli.act(pos)).is_ok());
        }

        #[test]
        fn writing_to_terminal_can_fail(p: Player, pos: Position, e: String) {
            let mut cli = Cli::new(p);
            let failure = anyhow!(e.clone());
            cli.terminal.expect_write().times(1).with(eq(pos))
                .return_once(move |_| Err(failure));

            assert_eq!(block_on(cli.act(pos)).unwrap_err().to_string(), e);
        }

        #[test]
        fn reading_from_terminal_can_fail(p: Player, pos: Position, e: String) {
            let mut cli = Cli::new(p);
            cli.terminal.expect_write().with(eq(pos))
                .returning(|_| Ok(()));

            let failure = anyhow!(e.clone());
            cli.terminal.expect_read().times(1).with(eq(p.color))
                .return_once(move |_| Err(failure));

            assert_eq!(block_on(cli.act(pos)).unwrap_err().to_string(), e);
        }
    }
}
