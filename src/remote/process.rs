use crate::Remote;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use std::{fmt::Display, io, process::Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::{runtime, task::block_in_place};
use tracing::{error, info, instrument, warn};

/// The reason why spawning the remote process failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "failed to spawn the remote process")]
pub struct ProcessSpawnError(io::Error);

impl From<Anyhow> for ProcessSpawnError {
    fn from(e: Anyhow) -> Self {
        io::Error::new(io::ErrorKind::Other, e).into()
    }
}

/// The reason why writing to or reading from the remote process failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the remote process failed during IO")]
pub struct ProcessIoError(#[from(forward)] io::Error);

/// An implementation of trait [`Remote`] as a child process.
///
/// # Warning
/// Dropping this type blocks until the child process exits.
#[derive(DebugCustom)]
#[debug(fmt = "Process({})", "child.id().map(i64::from).unwrap_or(-1)")]
pub struct Process {
    child: Child,
    reader: Lines<BufReader<ChildStdout>>,
    writer: BufWriter<ChildStdin>,
}

impl Process {
    /// Spawns a child process.
    #[instrument(level = "trace", err)]
    pub async fn spawn(program: &str) -> Result<Self, ProcessSpawnError> {
        let mut child = tokio::process::Command::new(program)
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
    #[instrument(level = "trace")]
    fn drop(&mut self) {
        let pid = self.child.id();

        let result: Result<_, Anyhow> = block_in_place(|| {
            runtime::Handle::try_current()?.block_on(async {
                self.writer.flush().await?;
                Ok(self.child.wait().await?)
            })
        });

        match result.context("failed to gracefully terminate the child process") {
            Ok(s) => info!(pid, "{}", s),
            Err(e) => error!(pid, "{:?}", e),
        }
    }
}

#[async_trait]
impl Remote for Process {
    type Error = ProcessIoError;

    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        use io::ErrorKind::UnexpectedEof;
        Ok(self.reader.next_line().await?.ok_or(UnexpectedEof)?)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        let msg = item.to_string();
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush().await?;
        Ok(())
    }
}
