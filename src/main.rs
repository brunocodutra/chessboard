use anyhow::Error;
use async_std::{io, prelude::*};
use chessboard::*;

macro_rules! echo {
    ($io:expr, $($args:tt)*) => {
        $io.write_all(format!($($args)*).as_bytes())
    };
}

fn main() -> Result<(), Error> {
    smol::run(async {
        let mut game = Game::new();

        let mut black = actor::Cli::new(
            Terminal::new(Color::Black.to_string()),
            Player {
                color: Color::Black,
            },
        );

        let mut white = actor::Cli::new(
            Terminal::new(Color::White.to_string()),
            Player {
                color: Color::White,
            },
        );

        let outcome = loop {
            match game.outcome() {
                Some(o) => break o,

                None => {
                    let player = match game.player().color {
                        Color::Black => &mut black,
                        Color::White => &mut white,
                    };

                    let action = player.act(game.position()).await?;

                    if let Err(e) = game.execute(action) {
                        echo!(io::stderr(), "error: Invalid player action: {}\n\n", e).await?
                    }
                }
            }
        };

        echo!(io::stdout(), "The game has ended in a {}\n", outcome).await?;

        Ok(())
    })
}
