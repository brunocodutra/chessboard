use crate::{Action, File, Move, Placement, Player, Position, Rank, Remote, Square};
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use clap::Parser;
use derive_more::{Constructor, Deref, Display, From};
use std::fmt::{Debug, Display, Error as FmtError, Formatter};
use std::{error::Error, str::FromStr};
use tracing::instrument;

#[cfg(test)]
use test_strategy::Arbitrary;

/// Command Line Interface
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Parser)]
#[cfg_attr(test, derive(Arbitrary))]
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
    Anyhow: From<T::Err>,
{
    s.parse().map_err(|e| format!("{:?}", Anyhow::from(e)))
}

#[derive(Debug, From, Constructor)]
pub struct Cli<R>
where
    R: Remote + Debug,
    R::Error: Error + Send + Sync + 'static,
{
    remote: R,
}

#[async_trait]
impl<R> Player for Cli<R>
where
    R: Remote + Debug + Send,
    R::Error: Error + Send + Sync + 'static,
{
    type Error = R::Error;

    #[instrument(level = "trace", err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        self.remote.send(Board(pos.placement())).await?;

        loop {
            self.remote.flush().await?;
            let line = self.remote.recv().await?;

            match Cmd::try_parse_from(line.split_whitespace()) {
                Ok(s) => break Ok(s.into()),
                Err(e) => self.remote.send(e).await?,
            };
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
struct Board(Placement);

impl Display for Board {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "  ")?;
        for file in File::iter() {
            write!(f, "   {}", file)?;
        }

        writeln!(f)?;
        writeln!(f, "   +---+---+---+---+---+---+---+---+")?;

        for rank in Rank::iter().rev() {
            write!(f, " {} |", rank)?;

            for file in File::iter() {
                match self[Square(file, rank)] {
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
    use crate::remote::MockRemote;
    use clap::{Error as ClapError, ErrorKind as ClapErrorKind};
    use mockall::{predicate::*, Sequence};
    use smol::block_on;
    use std::io::Error as IoError;
    use test_strategy::proptest;

    #[proptest]
    fn player_can_execute_any_command(pos: Position, cmd: Cmd) {
        let mut remote = MockRemote::new();
        let mut seq = Sequence::new();

        remote
            .expect_send()
            .times(1)
            .in_sequence(&mut seq)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_flush()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn player_can_resign(pos: Position) {
        let mut remote = MockRemote::new();
        let mut seq = Sequence::new();

        remote
            .expect_send()
            .times(1)
            .in_sequence(&mut seq)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_flush()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(move || Ok(Cmd::Resign.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, Action::Resign);
    }

    #[proptest]
    fn player_can_make_a_move(pos: Position, m: Move) {
        let mut remote = MockRemote::new();
        let mut seq = Sequence::new();

        remote
            .expect_send()
            .times(1)
            .in_sequence(&mut seq)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_flush()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move || Ok(Cmd::Move { descriptor: m }.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, Action::Move(m));
    }

    #[proptest]
    fn resign_takes_no_arguments(pos: Position, cmd: Cmd, #[strategy("[^\\s]+")] arg: String) {
        let mut remote = MockRemote::new();

        remote
            .expect_send()
            .times(1)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_send()
            .times(1)
            .returning(|_: ClapError| Ok(()));

        remote.expect_flush().times(2).returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(format!("resign {}", arg)));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn move_does_not_accept_invalid_moves(
        pos: Position,
        cmd: Cmd,
        #[by_ref]
        #[filter(#arg.parse::<Move>().is_err())]
        arg: String,
    ) {
        let mut remote = MockRemote::new();

        remote
            .expect_send()
            .times(1)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_send()
            .times(1)
            .returning(|_: ClapError| Ok(()));

        remote.expect_flush().times(2).returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(format!("move {}", arg)));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn player_can_ask_for_help(
        pos: Position,
        cmd: Cmd,
        #[strategy("|help|resign|move")] arg: String,
    ) {
        let mut remote = MockRemote::new();

        remote
            .expect_send()
            .times(1)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_send()
            .times(1)
            .with(function(|e: &ClapError| {
                e.kind() == ClapErrorKind::DisplayHelp
            }))
            .returning(|_| Ok(()));

        remote.expect_flush().times(2).returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(format!("help {}", arg)));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn player_is_prompted_again_after_invalid_command(
        pos: Position,
        cmd: Cmd,
        #[by_ref]
        #[filter(Cmd::try_parse_from(#invalid_cmd.split_whitespace()).is_err())]
        invalid_cmd: String,
    ) {
        let mut remote = MockRemote::new();

        remote
            .expect_send()
            .times(1)
            .with(eq(Board(pos.placement())))
            .returning(|_| Ok(()));

        remote
            .expect_send()
            .times(1)
            .returning(|_: ClapError| Ok(()));

        remote.expect_flush().times(2).returning(|| Ok(()));

        remote
            .expect_recv()
            .times(1)
            .return_once(move || Ok(invalid_cmd));

        remote
            .expect_recv()
            .times(1)
            .returning(move || Ok(cmd.to_string()));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos))?, cmd.into());
    }

    #[proptest]
    fn play_can_fail_writing(pos: Position, e: IoError) {
        let mut remote = MockRemote::new();

        let kind = e.kind();
        remote.expect_send().return_once(move |_: Board| Err(e));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
    }

    #[proptest]
    fn play_can_fail_flushing(pos: Position, e: IoError) {
        let mut remote = MockRemote::new();

        remote.expect_send().returning(|_: Board| Ok(()));

        let kind = e.kind();
        remote.expect_flush().return_once(move || Err(e));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
    }

    #[proptest]
    fn play_can_fail_reading(pos: Position, e: IoError) {
        let mut remote = MockRemote::new();

        remote.expect_send().returning(|_: Board| Ok(()));
        remote.expect_flush().returning(|| Ok(()));

        let kind = e.kind();
        remote.expect_recv().return_once(move || Err(e));

        let mut cli = Cli::new(remote);
        assert_eq!(block_on(cli.act(&pos)).unwrap_err().kind(), kind);
    }
}
