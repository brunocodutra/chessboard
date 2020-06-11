use crate::Remote;
use async_std::{io, prelude::*, sync::*};
use async_trait::async_trait;
use rustyline::{Config, Editor};
use std::fmt::Display;

/// An implementation of trait [`Remote`] as a terminal based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
pub struct Terminal {
    prompt: String,
    reader: Arc<Mutex<rustyline::Editor<()>>>,
    writer: io::Stdout,
}

impl Terminal {
    pub fn new<D: Display>(context: D) -> Self {
        Terminal {
            prompt: format!("{} > ", context),

            reader: Arc::new(Mutex::new(Editor::<()>::with_config(
                Config::builder().auto_add_history(true).build(),
            ))),

            writer: io::stdout(),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = anyhow::Error;

    async fn recv(&mut self) -> Result<String, Self::Error> {
        let reader = self.reader.clone();
        let prompt = self.prompt.clone();
        let line = smol::blocking!(reader.lock().await.readline(&prompt))?;
        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        self.writer.write_all(line.as_bytes()).await?;
        Ok(())
    }
}
