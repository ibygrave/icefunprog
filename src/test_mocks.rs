#![cfg(test)]
use std::cell::RefCell;
use std::fmt::Debug;
use std::io::{Cursor, Read, Write};
use std::rc::Rc;

use crate::{
    cmds::{CmdArgs, CmdReply, Command},
    err::Error,
    serialport::SerialPort,
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

#[derive(Clone)]
pub(crate) struct MockPort {
    reader: Rc<RefCell<ReadBuf>>,
    writer: Rc<RefCell<WriteBuf>>,
}

impl MockPort {
    fn new(data: Vec<u8>) -> Self {
        Self {
            reader: Rc::new(RefCell::new(ReadBuf(Cursor::new(data)))),
            writer: Rc::new(RefCell::new(WriteBuf(Cursor::new(vec![])))),
        }
    }
    fn test_port(&self) -> Box<dyn SerialPort> {
        Box::new(self.clone())
    }
    pub(crate) fn written(self) -> Vec<u8> {
        Rc::into_inner(self.writer)
            .unwrap()
            .into_inner()
            .0
            .into_inner()
    }
}

impl Read for MockPort {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.borrow_mut().read(buf)
    }
}

impl Write for MockPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.borrow_mut().flush()
    }
}

impl SerialPort for MockPort {}

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
        let port = MockPort::new(data);
        let result = self.run_args(&mut port.test_port(), args);
        (port, result)
    }
}
