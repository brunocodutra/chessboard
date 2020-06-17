use crate::Remote;
use anyhow::{Context, Error as Anyhow};
use async_std::{io, prelude::*, sync::*};
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use smol::{blocking, reader, writer};
use std::{ffi::OsStr, fmt::Display, process::*};
use tracing::*;

/// The reason why spawning, writing to or reading from the remote process failed.
#[derive(Debug, Display, Error, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct RemoteProcessIoError(io::Error);

impl From<io::ErrorKind> for RemoteProcessIoError {
    fn from(k: io::ErrorKind) -> Self {
        io::Error::from(k).into()
    }
}

impl From<Anyhow> for RemoteProcessIoError {
    fn from(e: Anyhow) -> Self {
        io::Error::new(io::ErrorKind::Other, e).into()
    }
}

/// An implementation of trait [`Remote`] as a child process.
///
/// # Warning
/// Dropping this type blocks until the child process exits.
pub struct Process {
    child: Child,
    reader: Box<Mutex<dyn Stream<Item = io::Result<String>> + Send + Unpin>>,
    writer: Box<Mutex<dyn io::Write + Send + Unpin>>,
}

impl Process {
    #[instrument(skip(program), err)]
    pub async fn spawn<S>(program: S) -> Result<Self, RemoteProcessIoError>
    where
        S: AsRef<OsStr> + Send + 'static,
    {
        info!(program = ?program.as_ref());

        let mut child = blocking!(Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn())?;

        let stdout = child.stdout.take().context("failed to open stdout")?;
        let stdin = child.stdin.take().context("failed to open stdin")?;

        Ok(Process {
            child,
            reader: Box::new(Mutex::new(io::BufReader::new(reader(stdout)).lines())),
            writer: Box::new(Mutex::new(writer(stdin))),
        })
    }
}

/// Waits for the child process to exit.
impl Drop for Process {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        let status = self.child.wait();
        if let Err(e) = status.context("the child process exited with an error") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl Remote for Process {
    type Error = RemoteProcessIoError;

    #[instrument(skip(self), err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let next = self.reader.lock().await.next().await;
        let line = next.ok_or(io::ErrorKind::UnexpectedEof)??;
        trace!(%line);
        Ok(line)
    }

    #[instrument(skip(self, msg), err)]
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        trace!(%line);
        self.writer.lock().await.write_all(line.as_bytes()).await?;
        Ok(())
    }
}
