#![cfg(test)]
use std::fmt::Debug;
use std::io::{Cursor, Read, Write};

use crate::{
    cmds::{CmdArgs, CmdReply, Command},
    err::Error,
};

pub(crate) struct ReadBuf(Cursor<Vec<u8>>);

impl Read for ReadBuf {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

pub(crate) struct WriteBuf(Cursor<Vec<u8>>);

impl Write for WriteBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

pub(crate) struct MockPort {
    reader: ReadBuf,
    writer: WriteBuf,
}

impl MockPort {
    fn new(data: Vec<u8>) -> Self {
        Self {
            reader: ReadBuf(Cursor::new(data)),
            writer: WriteBuf(Cursor::new(vec![])),
        }
    }
    pub(crate) fn written(self) -> Vec<u8> {
        self.writer.0.into_inner()
    }
}

impl AsMut<ReadBuf> for MockPort {
    fn as_mut(&mut self) -> &mut ReadBuf {
        &mut self.reader
    }
}

impl AsMut<WriteBuf> for MockPort {
    fn as_mut(&mut self) -> &mut WriteBuf {
        &mut self.writer
    }
}

pub(crate) trait TestCmd<A, R> {
    type Error: Debug;
    fn test(&self, data: Vec<u8>, args: &A) -> (MockPort, Result<R, Self::Error>);
    fn test_ok(&self, data: Vec<u8>, args: &A) -> (MockPort, R) {
        let (port, result) = self.test(data, args);
        (port, result.unwrap())
    }
    fn test_err(&self, data: Vec<u8>, args: &A) -> MockPort {
        let (port, result) = self.test(data, args);
        assert!(result.is_err());
        port
    }
}

impl<A: CmdArgs, R: CmdReply> TestCmd<A, R> for Command<A, R> {
    type Error = Error;
    fn test(&self, data: Vec<u8>, args: &A) -> (MockPort, Result<R, Self::Error>) {
        let mut port = MockPort::new(data);
        let result = self.send::<_, ReadBuf, WriteBuf>(&mut port, args);
        (port, result)
    }
}
