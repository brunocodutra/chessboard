use crate::Io;
use async_trait::async_trait;
use derive_more::DebugCustom;
use rustyline::{error::ReadlineError, Config, Editor};
use std::{fmt::Display, io, sync::Arc};
use tokio::io::{stdout, AsyncWriteExt, Stdout};
use tokio::{sync::Mutex, task::block_in_place};
use tracing::instrument;

/// A prompt interface based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
#[derive(DebugCustom)]
#[debug(fmt = "Terminal")]
pub struct Terminal {
    writer: Stdout,
    reader: Arc<Mutex<Editor<()>>>,
}

impl Terminal {
    /// Opens a terminal interface with the given prompt.
    #[instrument(level = "trace")]
    pub fn open() -> Self {
        Terminal {
            writer: stdout(),
            reader: Arc::new(Mutex::new(Editor::with_config(
                Config::builder().auto_add_history(true).build(),
            ))),
        }
    }
}

#[async_trait]
impl Io for Terminal {
    #[instrument(level = "trace", err, ret)]
    async fn recv(&mut self) -> io::Result<String> {
        let mut reader = self.reader.clone().lock_owned().await;
        match block_in_place(move || reader.readline("> ")) {
            Err(ReadlineError::Io(e)) => Err(e),
            Err(ReadlineError::Eof) => Err(io::ErrorKind::UnexpectedEof.into()),
            Err(ReadlineError::Interrupted) => Err(io::ErrorKind::Interrupted.into()),

            #[cfg(unix)]
            Err(ReadlineError::Utf8Error) => Err(io::ErrorKind::InvalidData.into()),

            #[cfg(windows)]
            Err(ReadlineError::Decode(e)) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),

            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),

            Ok(r) => Ok(r),
        }
    }

    #[instrument(level = "trace", err, skip(item), fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        let msg = item.to_string();
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await?;
        Ok(())
    }
}
