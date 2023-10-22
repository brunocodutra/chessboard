use anyhow::Result;
use clap::Parser;
use lib::uci::Uci;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[arg(skip)]
    server: Uci,
}

impl Cli {
    fn run(mut self) -> Result<()> {
        self.server.run()?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Cli::parse().run()
}
