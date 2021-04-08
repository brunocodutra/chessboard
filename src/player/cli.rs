use crate::{Action, Color, File, Move, Placement, Player, Position, Rank, Remote, Square};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use clap::AppSettings::*;
use derive_more::{Constructor, Deref, Display, From};
use std::{error::Error, fmt, str::FromStr};
use structopt::StructOpt;
use tracing::instrument;

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, StructOpt)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[structopt(
    author,
    name = "Chessboard",
    usage = "<SUBCOMMAND> [ARGS]",
    after_help = "See 'help <SUBCOMMAND>' for more information on a specific command.",
    global_settings = &[NoBinaryName, DisableVersion, DisableHelpFlags],
)]
enum Cmd {
    #[display(fmt = "resign")]
    #[structopt(about = "Resign the game in favor of the opponent", no_version)]
    Resign,

    #[display(fmt = "move {}", descriptor)]
    #[structopt(
        about = "Move a piece on the board",
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

impl Cmd {
    fn into_action(self, p: Color) -> Action {
        match self {
            Cmd::Resign => Action::Resign(p),
            Cmd::Move { descriptor } => Action::Move(descriptor),
        }
    }
}

#[instrument(err)]
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

    #[instrument(skip(self), err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        self.remote.send(Board(pos.placement())).await?;

        loop {
            self.remote.flush().await?;
            let line = self.remote.recv().await?;

            match Cmd::from_iter_safe(line.split_whitespace()) {
                Ok(s) => break Ok(s.into_action(pos.turn())),
                Err(e) => self.remote.send(e).await?,
            };
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
struct Board(Placement);

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "   ")?;

        for &file in File::VARIANTS {
            write!(f, "  {} ", file)?;
        }

        writeln!(f)?;
        writeln!(f, "   +---+---+---+---+---+---+---+---+")?;

        for &rank in Rank::VARIANTS.iter().rev() {
            write!(f, " {} |", rank)?;

            for &file in File::VARIANTS {
                match self[Square(file, rank)] {
                    Some(piece) => write!(f, " {:#} |", piece)?,
                    None => write!(f, "   |")?,
                }
            }

            writeln!(f, " {}", rank)?;
            writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        }

        write!(f, "   ")?;
        for &file in File::VARIANTS {
            write!(f, "  {} ", file)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::MockRemote;
    use mockall::{predicate::*, Sequence};
    use proptest::{collection::vec, prelude::*};
    use smol::block_on;
    use std::io;

    fn invalid_move() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("valid move", |s| s.parse::<Move>().is_err())
    }

    fn invalid_command() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("valid command", |s| {
            Cmd::from_iter_safe(s.split_whitespace()).is_err()
        })
    }

    proptest! {
        #[test]
        fn player_can_execute_any_command(pos: Position, cmd: Cmd) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(cmd.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), cmd.into_action(pos.turn()));
        }

        #[test]
        fn player_can_resign(pos: Position) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .return_once(move || Ok(Cmd::Resign.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), Action::Resign(pos.turn()));
        }

        #[test]
        fn player_can_make_a_move(pos: Position, m: Move) {
            let mut remote = MockRemote::new();
            let mut seq = Sequence::new();

            remote.expect_send().times(1).in_sequence(&mut seq)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_flush().times(1).in_sequence(&mut seq)
                .returning(|| Ok(()));

            remote.expect_recv().times(1).in_sequence(&mut seq)
                .returning(move || Ok(Cmd::Move { descriptor: m }.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn resign_takes_no_arguments(pos: Position, cmd: Cmd, arg in "[^\\s]+") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(format!("resign {}", arg)));

            remote.expect_recv().times(1)
                .returning(move || Ok(cmd.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), cmd.into_action(pos.turn()));
        }

        #[test]
        fn move_does_not_accept_invalid_moves(pos: Position, cmd: Cmd, arg in invalid_move()) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(format!("move {}", arg)));

            remote.expect_recv().times(1)
                .returning(move || Ok(cmd.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), cmd.into_action(pos.turn()));
        }

        #[test]
        fn player_can_ask_for_help(pos: Position, cmd: Cmd, arg in "|help|resign|move") {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_send().times(1)
                .with(function(|&clap::Error { kind, .. }| kind == clap::ErrorKind::HelpDisplayed))
                .returning(|_| Ok(()));

            remote.expect_flush().times(2)
                .returning(|| Ok(()));

            remote.expect_recv().times(1)
                .returning(move || Ok(format!("help {}", arg)));

            remote.expect_recv().times(1)
                .returning(move || Ok(cmd.to_string()));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), cmd.into_action(pos.turn()));
        }

        #[test]
        fn player_is_prompted_again_after_invalid_command(pos: Position, cmd: Cmd, mut cmds in vec(invalid_command(), 1..10)) {
            let mut remote = MockRemote::new();

            remote.expect_send().times(1)
                .with(eq(Board(pos.placement())))
                .returning(|_| Ok(()));

            remote.expect_send().times(cmds.len())
                .returning(|_: clap::Error| Ok(()));

            remote.expect_flush().times(cmds.len() + 1)
                .returning(|| Ok(()));

            remote.expect_recv().times(cmds.len() + 1)
                .returning(move || Ok(cmds.pop().unwrap_or_else(|| cmd.to_string())));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap(), cmd.into_action(pos.turn()));
        }

        #[test]
        fn play_can_fail_writing_to_the_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            let kind = e.kind();
            remote.expect_send().return_once(move |_: Board| Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_flushing_the_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: Board| Ok(()));

            let kind = e.kind();
            remote.expect_flush().return_once(move || Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
        }

        #[test]
        fn play_can_fail_reading_from_remote(pos: Position, e: io::Error) {
            let mut remote = MockRemote::new();

            remote.expect_send().returning(|_: Board| Ok(()));
            remote.expect_flush().returning(|| Ok(()));

            let kind = e.kind();
            remote.expect_recv().return_once(move || Err(e));

            let mut cli = Cli::new(remote);
            assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
        }
    }
}
