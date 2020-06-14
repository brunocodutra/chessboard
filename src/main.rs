use anyhow::{Context, Error as Failure};
use chessboard::*;
use std::{error::Error, io::stderr};
use tracing::*;

#[instrument(err)]
#[allow(clippy::unit_arg)]
async fn chess() -> Result<(), Failure> {
    let mut game = Game::new();
    let mut white = actor::Cli::new(Terminal::new(Color::White.to_string()));
    let mut black = actor::Cli::new(Terminal::new(Color::Black.to_string()));

    let outcome = loop {
        match game.outcome() {
            Some(o) => break o,

            None => {
                let player = match game.player().color {
                    Color::Black => &mut black,
                    Color::White => &mut white,
                };

                let position = game.position();
                info!(%position);

                let action = player.act(position).await?;
                info!(player = %game.player().color, ?action);

                if let Err(e) = game.execute(action).context("invalid player action") {
                    warn!("{:?}", e);
                }
            }
        }
    };

    info!(%outcome);

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let (writer, _guard) = tracing_appender::non_blocking(stderr());

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_env_filter("warn,chessboard=info")
        .try_init()?;

    smol::run(chess()).context("the match was interrupted")?;

    Ok(())
}
