use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Lines};
use tracing::instrument;

/// A generic io interface.
#[derive(Debug)]
pub struct Io<W: AsyncWrite, R: AsyncRead> {
    writer: W,
    reader: Lines<BufReader<R>>,
}

impl<W: AsyncWrite, R: AsyncRead> Io<W, R> {
    pub fn new(writer: W, reader: R) -> Self {
        Io {
            writer,
            reader: BufReader::new(reader).lines(),
        }
    }
}

impl<W: AsyncWrite + Send + Unpin, R: AsyncRead + Send + Unpin> Io<W, R> {
    /// Receive a message.
    #[instrument(level = "trace", skip(self), ret, err)]
    pub async fn recv(&mut self) -> io::Result<String> {
        use io::ErrorKind::UnexpectedEof;
        Ok(self.reader.next_line().await?.ok_or(UnexpectedEof)?)
    }

    /// Send a message.
    #[instrument(level = "trace", skip(self), err)]
    pub async fn send(&mut self, msg: &str) -> io::Result<()> {
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    /// Flush the internal buffers.
    #[instrument(level = "trace", skip(self), err)]
    pub async fn flush(&mut self) -> io::Result<()> {
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

    #[proptest(async = "tokio")]
    async fn recv_waits_for_line_break(#[strategy("[^\r\n]")] s: String) {
        let (stdin, _) = duplex(1);
        let (mut tx, stdout) = duplex(s.len() + 1);

        tx.write_all(s.as_bytes()).await?;
        tx.write_u8(b'\n').await?;

        let mut pipe = Io::new(stdin, BufReader::new(stdout));
        assert_eq!(pipe.recv().await?, s);
    }

    #[proptest(async = "tokio")]
    async fn send_appends_line_break(s: String) {
        let (stdin, mut rx) = duplex(s.len() + 1);
        let (_, stdout) = duplex(1);

        let expected = format!("{s}\n");

        let mut pipe = Io::new(stdin, BufReader::new(stdout));
        pipe.send(&s).await?;
        pipe.flush().await?;

        let mut buf = vec![0u8; expected.len()];
        rx.read_exact(&mut buf).await?;

        assert_eq!(str::from_utf8(&buf)?, expected);
    }
}
