use crate::{Act, Action, File, Io, Move, Position, Rank, Square};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use clap::Parser;
use derive_more::{Constructor, Deref, Display, Error, From};
use std::fmt::{self, Display};
use std::{error::Error, io, str::FromStr};
use tracing::instrument;

/// Command Line Interface
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Parser)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[clap(
    name = "",
    multicall = true,
    arg_required_else_help = true,
    disable_help_flag = true,
    disable_version_flag = true,
    allow_hyphen_values = true
)]
enum Cmd {
    /// Resign the game in favor of the opponent.
    #[display(fmt = "resign")]
    #[clap(allow_hyphen_values = true)]
    Resign,

    /// Move a piece on the board.
    #[display(fmt = "move {}", descriptor)]
    #[clap(
        allow_hyphen_values = true,
        after_help = r#"SYNTAX:
    <DESCRIPTOR>    ::= <SQUARE:from><SQUARE:to>[<PROMOTION>]
    <SQUARE>        ::= <FILE><RANK>
    <FILE>          ::= a|b|c|d|e|f|g|h
    <RANK>          ::= 1|2|3|4|5|6|7|8
    <PROMOTION>     ::= q|r|b|n"#
    )]
    Move {
        /// A chess move in pure coordinate notation.
        #[clap(parse(try_from_str = try_parse_descriptor))]
        descriptor: Move,
    },
}

impl From<Cmd> for Action {
    fn from(cmd: Cmd) -> Self {
        match cmd {
            Cmd::Resign => Action::Resign,
            Cmd::Move { descriptor } => Action::Move(descriptor),
        }
    }
}

fn try_parse_descriptor<T>(s: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    s.parse().map_err(|e| format!("{:?}", Anyhow::from(e)))
}

/// The reason why an [`Action`] could not be received through the CLI.
#[derive(Debug, Display, Error, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "the CLI encountered an error")]
pub struct CliError(#[from(forward)] io::Error);

/// A Command Line Interface for a human player.
#[derive(Debug, From, Constructor)]
pub struct Cli<T: Io> {
    io: T,
}

#[async_trait]
impl<T: Io + Send> Act for Cli<T> {
    type Error = CliError;

    /// Prompt the user for an action.
    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos))]
    async fn act(&mut self, pos: &Position) -> Result<Action, CliError> {
        self.io.send(&Board(pos).to_string()).await?;

        loop {
            self.io.flush().await?;
            let line = self.io.recv().await?;

            match Cmd::try_parse_from(line.split_whitespace()) {
                Ok(s) => break Ok(s.into()),
                Err(e) => self.io.send(&e.to_string()).await?,
            }
        }
    }
}

#[derive(Deref)]
struct Board<'a>(&'a Position);

impl<'a> Display for Board<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "  ")?;
        for file in File::iter() {
            write!(f, "   {}", file)?;
        }

        writeln!(f)?;
        writeln!(f, "   +---+---+---+---+---+---+---+---+")?;

        for rank in Rank::iter().rev() {
            write!(f, " {} |", rank)?;

            for file in File::iter() {
                match self[Square::new(file, rank)] {
                    Some(piece) => write!(f, " {:#} |", piece)?,
                    None => write!(f, "   |")?,
                }
            }

            writeln!(f, " {}", rank)?;
            writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        }

        write!(f, "  ")?;
        for file in File::iter() {
            write!(f, "   {}", file)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockIo;
    use clap::IntoApp;
    use mockall::Sequence;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn board_is_displayed_before_prompting_player_for_action(pos: Position, cmd: Cmd) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();
        let mut seq = Sequence::new();

        let board = Board(&pos).to_string();
        io.expect_send()
            .once()
            .in_sequence(&mut seq)
            .withf(move |msg| msg == board)
            .returning(|_| Ok(()));

        io.expect_flush()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        io.expect_recv()
            .once()
            .in_sequence(&mut seq)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn player_can_resign(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .return_once(move || Ok(Cmd::Resign.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, Action::Resign);
    }

    #[proptest]
    fn player_can_make_a_move(pos: Position, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(Cmd::Move { descriptor: m }.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, Action::Move(m));
    }

    #[proptest]
    fn player_can_ask_for_help(pos: Position, cmd: Cmd) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().once().returning(|_| Ok(()));

        let mut buffer = Vec::new();
        Cmd::command().write_help(&mut buffer)?;
        let help = String::from_utf8(buffer)?;

        io.expect_send()
            .once()
            .withf(move |msg| msg == help)
            .returning(|_| Ok(()));

        io.expect_flush().returning(|| Ok(()));

        io.expect_recv().once().returning(move || Ok("help".into()));

        io.expect_recv()
            .once()
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn help_is_displayed_if_no_command_is_given(
        pos: Position,
        cmd: Cmd,
        #[strategy("\\s+")] arg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().once().returning(|_| Ok(()));

        let mut buffer = Vec::new();
        Cmd::command().write_help(&mut buffer)?;
        let help = String::from_utf8(buffer)?;

        io.expect_send()
            .once()
            .withf(move |msg| msg == help)
            .returning(|_| Ok(()));

        io.expect_flush().returning(|| Ok(()));

        io.expect_recv().once().return_once(move || Ok(arg));

        io.expect_recv()
            .once()
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn resign_takes_no_arguments(pos: Position, cmd: Cmd, #[strategy("[^\\s]+")] arg: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(format!("resign {}", arg)));

        io.expect_recv()
            .once()
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn move_does_not_accept_invalid_moves(
        pos: Position,
        cmd: Cmd,
        #[by_ref]
        #[filter(#arg.parse::<Move>().is_err())]
        arg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv()
            .once()
            .returning(move || Ok(format!("move {}", arg)));

        io.expect_recv()
            .once()
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn player_is_prompted_again_after_invalid_command(
        pos: Position,
        cmd: Cmd,
        #[by_ref]
        #[filter(Cmd::try_parse_from(#arg.split_whitespace()).is_err())]
        arg: String,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        io.expect_recv().once().return_once(move || Ok(arg));

        io.expect_recv()
            .once()
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(io);
        assert_eq!(rt.block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn play_can_fail_writing(pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        let kind = e.kind();
        io.expect_send().return_once(move |_| Err(e));

        let mut cli = Cli::new(io);
        assert_eq!(
            rt.block_on(cli.act(&pos)).map_err(|CliError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn play_can_fail_flushing(pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));

        let kind = e.kind();
        io.expect_flush().return_once(move || Err(e));

        let mut cli = Cli::new(io);
        assert_eq!(
            rt.block_on(cli.act(&pos)).map_err(|CliError(e)| e.kind()),
            Err(kind)
        );
    }

    #[proptest]
    fn play_can_fail_reading(pos: Position, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut io = MockIo::new();

        io.expect_send().returning(|_| Ok(()));
        io.expect_flush().returning(|| Ok(()));

        let kind = e.kind();
        io.expect_recv().return_once(move || Err(e));

        let mut cli = Cli::new(io);
        assert_eq!(
            rt.block_on(cli.act(&pos)).map_err(|CliError(e)| e.kind()),
            Err(kind)
        );
    }
}
