use std::fmt::Display;
use std::io::{self, BufRead, BufReader, ErrorKind, Lines, Read, Write};
use tracing::instrument;

/// A generic io interface.
#[derive(Debug)]
pub struct Io<W: Write, R: Read> {
    writer: W,
    reader: Lines<BufReader<R>>,
}

impl<W: Write, R: Read> Io<W, R> {
    pub fn new(writer: W, reader: R) -> Self {
        Io {
            writer,
            reader: BufReader::new(reader).lines(),
        }
    }
}

impl<W: Write + Send + Unpin, R: Read + Send + Unpin> Io<W, R> {
    /// Receive a message.
    #[instrument(level = "trace", skip(self), ret, err)]
    pub fn recv(&mut self) -> io::Result<String> {
        self.reader.next().ok_or(ErrorKind::UnexpectedEof)?
    }

    /// Send a message.
    #[instrument(level = "trace", skip(self, msg), err, fields(%msg))]
    pub fn send<T: Display>(&mut self, msg: T) -> io::Result<()> {
        writeln!(&mut self.writer, "{}", msg)
    }

    /// Flush the internal buffers.
    #[instrument(level = "trace", skip(self), err)]
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, io::empty, str};
    use test_strategy::proptest;

    #[proptest]
    fn recv_waits_for_line_break(#[strategy("[^\r\n]")] s: String) {
        let mut buf = VecDeque::new();
        writeln!(&mut buf, "{}", s)?;
        let mut pipe = Io::new(empty(), &mut buf);
        assert_eq!(pipe.recv()?, s);
    }

    #[proptest]
    fn send_appends_line_break(s: String) {
        let mut buf = Vec::new();
        let mut pipe = Io::new(&mut buf, empty());
        pipe.send(&s)?;
        pipe.flush()?;
        drop(pipe);
        assert_eq!(str::from_utf8(&buf)?, format!("{s}\n"));
    }
}
