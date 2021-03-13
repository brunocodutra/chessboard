use crate::Remote;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use smol::io::{self, BufReader, Lines};
use smol::{block_on, lock::Mutex, prelude::*, process::*};
use std::{ffi::OsStr, fmt::Display};
use tracing::*;

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
pub struct Process {
    child: Child,
    reader: Mutex<Lines<BufReader<ChildStdout>>>,
    writer: Mutex<ChildStdin>,
}

impl Process {
    #[instrument(skip(program), err)]
    pub async fn spawn<S>(program: S) -> Result<Self, ProcessSpawnError>
    where
        S: AsRef<OsStr> + Send + 'static,
    {
        info!(program = ?program.as_ref());

        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().context("failed to open stdout")?;
        let stdin = child.stdin.take().context("failed to open stdin")?;

        Ok(Process {
            child,
            reader: Mutex::new(io::BufReader::new(stdout).lines()),
            writer: Mutex::new(stdin),
        })
    }
}

/// Waits for the child process to exit.
impl Drop for Process {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        let status = block_on(self.child.status());
        if let Err(e) = status.context("the child process exited with an error") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl Remote for Process {
    type Error = ProcessIoError;

    #[instrument(skip(self), /*err*/)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let next = self.reader.lock().await.next().await;
        let line = next.ok_or(io::ErrorKind::UnexpectedEof)??;
        trace!(%line);
        Ok(line)
    }

    #[instrument(skip(self, msg), /*err*/)]
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        trace!(%line);
        self.writer.lock().await.write_all(line.as_bytes()).await?;
        Ok(())
    }

    #[instrument(skip(self), /*err*/)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.lock().await.flush().await?;
        Ok(())
    }
}
