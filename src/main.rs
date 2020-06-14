use anyhow::{Context, Error as Failure};
use chessboard::*;
use std::{error::Error, io::stderr};
use tracing::*;

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing_subscriber::fmt()
        .with_writer(stderr)
        .with_max_level(Level::INFO)
        .try_init()?;

    let result: Result<Outcome, Failure> = smol::run(async {
        let mut game = Game::new();
        let mut black = actor::Cli::new(Terminal::new(Color::Black.to_string()));
        let mut white = actor::Cli::new(Terminal::new(Color::White.to_string()));

        loop {
            match game.outcome() {
                Some(o) => break Ok(o),

                None => {
                    let player = match game.player().color {
                        Color::Black => &mut black,
                        Color::White => &mut white,
                    };

                    let action = player.act(game.position()).await?;

                    if let Err(e) = game.execute(action).context("invalid player action") {
                        warn!("{:?}", e);
                    }
                }
            }
        }
    });

    let outcome = result.context("the match was interrupted")?;
    info!("the match ended in a {}", outcome);

    Ok(())
}
