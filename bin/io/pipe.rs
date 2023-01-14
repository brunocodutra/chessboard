use super::Io;
use async_trait::async_trait;
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Lines};
use tracing::instrument;

/// A generic [`Io`] interface.
#[derive(Debug)]
pub struct Pipe<W: AsyncWrite, R: AsyncRead> {
    writer: W,
    reader: Lines<BufReader<R>>,
}

impl<W: AsyncWrite, R: AsyncRead> Pipe<W, R> {
    pub fn new(writer: W, reader: R) -> Self {
        Pipe {
            writer,
            reader: BufReader::new(reader).lines(),
        }
    }
}

impl<W: AsyncWrite, R: AsyncRead> From<(W, R)> for Pipe<W, R> {
    fn from((writer, reader): (W, R)) -> Self {
        Pipe::new(writer, reader)
    }
}

#[async_trait]
impl<W: AsyncWrite + Send + Unpin, R: AsyncRead + Send + Unpin> Io for Pipe<W, R> {
    #[instrument(level = "trace", skip(self), ret, err)]
    async fn recv(&mut self) -> io::Result<String> {
        use io::ErrorKind::UnexpectedEof;
        Ok(self.reader.next_line().await?.ok_or(UnexpectedEof)?)
    }

    #[instrument(level = "trace", skip(self), err)]
    async fn send(&mut self, msg: &str) -> io::Result<()> {
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self), err)]
    async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use test_strategy::proptest;
    use tokio::io::{duplex, AsyncReadExt, BufReader};
    use tokio::runtime;

    #[proptest]
    fn recv_waits_for_line_break(#[strategy("[^\r\n]")] s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let (stdin, _) = duplex(1);
        let (mut tx, stdout) = duplex(s.len() + 1);

        rt.block_on(tx.write_all(s.as_bytes()))?;
        rt.block_on(tx.write_u8(b'\n'))?;

        let mut pipe = Pipe::new(stdin, BufReader::new(stdout));
        assert_eq!(rt.block_on(pipe.recv())?, s);
    }

    #[proptest]
    fn send_appends_line_break(s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let (stdin, mut rx) = duplex(s.len() + 1);
        let (_, stdout) = duplex(1);

        let expected = format!("{s}\n");

        let mut pipe = Pipe::new(stdin, BufReader::new(stdout));
        rt.block_on(pipe.send(&s))?;
        rt.block_on(pipe.flush())?;

        let mut buf = vec![0u8; expected.len()];
        rt.block_on(rx.read_exact(&mut buf))?;

        assert_eq!(str::from_utf8(&buf)?, expected);
    }
}
