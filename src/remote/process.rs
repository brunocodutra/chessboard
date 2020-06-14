use crate::Remote;
use anyhow::Context;
use async_std::{io, prelude::*, sync::*};
use async_trait::async_trait;
use smol::{blocking, reader, writer};
use std::{ffi::OsStr, fmt::Display, process::*};
use tracing::*;

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
    pub async fn spawn<S>(program: S) -> Result<Self, anyhow::Error>
    where
        S: AsRef<OsStr> + Send + 'static,
    {
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
    fn drop(&mut self) {
        let status = self.child.wait();
        if let Err(e) = status.context("the child process exited with an error") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl Remote for Process {
    type Error = anyhow::Error;

    async fn recv(&mut self) -> Result<String, Self::Error> {
        let next = self.reader.lock().await.next().await;
        let line = next.context("broken pipe")??;
        Ok(line)
    }

    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        self.writer.lock().await.write_all(line.as_bytes()).await?;
        Ok(())
    }
}
