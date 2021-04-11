use crate::Remote;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use futures::io::{BufReader, BufWriter, Lines};
use futures::{AsyncBufReadExt, AsyncWriteExt, StreamExt};
use smol::block_on;
use smol::io::{Error as IoError, ErrorKind as IoErrorKind};
use smol::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::{ffi::OsStr, fmt::Display};
use tracing::{error, info, instrument};

/// The reason why spawning the remote process failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "failed to spawn the remote process")]
pub struct ProcessSpawnError(IoError);

impl From<Anyhow> for ProcessSpawnError {
    fn from(e: Anyhow) -> Self {
        IoError::new(IoErrorKind::Other, e).into()
    }
}

/// The reason why writing to or reading from the remote process failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the remote process failed during IO")]
pub struct ProcessIoError(#[from(forward)] IoError);

/// An implementation of trait [`Remote`] as a child process.
///
/// # Warning
/// Dropping this type blocks until the child process exits.
#[derive(DebugCustom)]
#[debug(fmt = "Process({})", "child.id()")]
pub struct Process {
    child: Child,
    reader: Lines<BufReader<ChildStdout>>,
    writer: BufWriter<ChildStdin>,
}

impl Process {
    /// Spawns a child process.
    #[instrument(skip(program), err, fields(program = ?program.as_ref()))]
    pub async fn spawn<S: AsRef<OsStr>>(program: S) -> Result<Self, ProcessSpawnError> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().context("failed to open stdout")?;
        let stdin = child.stdin.take().context("failed to open stdin")?;

        info!(pid = child.id());

        Ok(Process {
            child,
            reader: BufReader::new(stdout).lines(),
            writer: BufWriter::new(stdin),
        })
    }
}

/// Flushes the outbound buffer and waits for the child process to exit.
impl Drop for Process {
    #[instrument]
    fn drop(&mut self) {
        if let Err(e) = block_on(self.writer.flush()).context("failed to flush the buffer") {
            error!("{:?}", e);
        }

        match block_on(self.child.status()).context("the child process exited with an error") {
            Ok(s) => info!("{}", s),
            Err(e) => error!("{:?}", e),
        }
    }
}

#[async_trait]
impl Remote for Process {
    type Error = ProcessIoError;

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
