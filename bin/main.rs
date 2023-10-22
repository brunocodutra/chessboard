use anyhow::Error as Anyhow;
use clap::Parser;
use lib::uci::Uci;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[arg(skip)]
    server: Uci,
}

impl Cli {
    pub fn run(mut self) -> Result<(), Anyhow> {
        self.server.run()
    }
}

fn main() -> Result<(), Anyhow> {
    Cli::parse().run()
}
