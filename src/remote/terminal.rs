use crate::Remote;
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use futures::{AsyncWriteExt, StreamExt};
use rustyline::{error::ReadlineError, Config, Editor};
use smol::io::{Error as IoError, ErrorKind as IoErrorKind};
use smol::Unblock;
use std::{fmt::Display, io::stdout, io::Stdout};
use tracing::instrument;

/// The reason why writing to or reading from the terminal failed.
#[derive(Debug, Display, Error, From)]
pub struct TerminalIoError(IoError);

impl From<IoErrorKind> for TerminalIoError {
    fn from(k: IoErrorKind) -> Self {
        IoError::from(k).into()
    }
}

impl From<ReadlineError> for TerminalIoError {
    fn from(e: ReadlineError) -> Self {
        match e {
            ReadlineError::Io(e) => e.into(),
            ReadlineError::Eof => IoErrorKind::UnexpectedEof.into(),
            ReadlineError::Interrupted => IoErrorKind::Interrupted.into(),

            #[cfg(unix)]
            ReadlineError::Utf8Error => IoErrorKind::InvalidData.into(),

            #[cfg(windows)]
            ReadlineError::Decode(e) => IoError::new(IoErrorKind::InvalidData, e).into(),

            e => IoError::new(IoErrorKind::Other, e).into(),
        }
    }
}

#[derive(Debug)]
struct Prompter {
    prompt: String,
    editor: Editor<()>,
}

impl Iterator for Prompter {
    type Item = Result<String, ReadlineError>;

    #[instrument]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.editor.readline(&self.prompt))
    }
}

/// A prompt interface based on [rustyline].
///
/// [rustyline]: https://crates.io/crates/rustyline
#[derive(Debug)]
pub struct Terminal {
    reader: Unblock<Prompter>,
    writer: Unblock<Stdout>,
}

impl Terminal {
    /// Opens a terminal interface with the given prompt.
    #[instrument(skip(prompt), fields(%prompt))]
    pub fn new<P: Display>(prompt: P) -> Self {
        Terminal {
            reader: Unblock::new(Prompter {
                prompt: format!("{} > ", prompt),
                editor: Editor::with_config(Config::builder().auto_add_history(true).build()),
            }),

            writer: Unblock::new(stdout()),
        }
    }
}

#[async_trait]
impl Remote for Terminal {
    type Error = TerminalIoError;

    #[instrument(err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        use IoErrorKind::UnexpectedEof;
        Ok(self.reader.next().await.ok_or(UnexpectedEof)??)
    }

    #[instrument(skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", item);
        Ok(self.writer.write_all(line.as_bytes()).await?)
    }

    #[instrument(err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.writer.flush().await?)
    }
}
