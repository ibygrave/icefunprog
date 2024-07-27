use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use tracing::instrument;

use crate::err::Error;
use crate::serialport::SerialPort;

pub(crate) const PAGE_SIZE: usize = 256;
pub(crate) const CMD_GET_VER: Command<(), GetVerReply> = Command::new(0xb1);
pub(crate) const CMD_RESET: Command<(), [u8; 3]> = Command::new(0xb2);
pub(crate) const CMD_ERASE_64K: Command<[u8; 1], ()> = Command::new(0xb4);
pub(crate) const CMD_PROGRAM_PAGE: Command<ProgData, ProgResult> = Command::new(0xb5);
pub(crate) const CMD_READ_PAGE: Command<ReadData, ReadResult> = Command::new(0xb6);
pub(crate) const CMD_VERIFY_PAGE: Command<ProgData, ProgResult> = Command::new(0xb7);
pub(crate) const CMD_RELEASE_FPGA: Command<(), ()> = Command::new(0xb9);

pub(crate) trait CmdArgs: Debug {
    fn send_args(&self, port: &mut Box<dyn SerialPort>) -> Result<(), Error>;
}

impl CmdArgs for () {
    fn send_args(&self, _port: &mut Box<dyn SerialPort>) -> Result<(), Error> {
        // Sends zero bytes
        Ok(())
    }
}

impl<const LEN: usize> CmdArgs for [u8; LEN] {
    fn send_args(&self, port: &mut Box<dyn SerialPort>) -> Result<(), Error> {
        port.write_all(self)?;
        Ok(())
    }
}

pub(crate) trait CmdReply: Debug
where
    Self: Sized,
{
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error>;
}

impl CmdReply for () {
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error> {
        let mut buf = [0u8];
        port.read_exact(&mut buf)?;
        Ok(())
    }
}

impl<const LEN: usize> CmdReply for [u8; LEN] {
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error> {
        let mut buf = [0u8; LEN];
        port.read_exact(&mut buf)?;
        Ok(buf)
    }
}

pub(crate) struct Command<Args: CmdArgs, Reply: CmdReply> {
    cmd: u8,
    _args: PhantomData<Args>,
    _reply: PhantomData<Reply>,
}

impl<Args: CmdArgs, Reply: CmdReply> Debug for Command<Args, Reply> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command").field("cmd", &self.cmd).finish()
    }
}

impl<Args: CmdArgs, Reply: CmdReply> Command<Args, Reply> {
    const fn new(cmd: u8) -> Self {
        Self {
            cmd,
            _args: PhantomData,
            _reply: PhantomData,
        }
    }

    #[instrument(skip(port))]
    pub(crate) fn run_args(
        &self,
        port: &mut Box<dyn SerialPort>,
        args: &Args,
    ) -> Result<Reply, Error> {
        port.write_all(&[self.cmd])?;
        args.send_args(port)?;
        Reply::receive_reply(port)
    }
}

impl<Reply: CmdReply> Command<(), Reply> {
    #[instrument(skip(port))]
    pub(crate) fn run(&self, port: &mut Box<dyn SerialPort>) -> Result<Reply, Error> {
        self.run_args(port, &())
    }
}

#[derive(Debug)]
pub(crate) struct ProgData<'a> {
    pub addr: usize,
    pub data: &'a [u8],
}

impl CmdArgs for ProgData<'_> {
    fn send_args(&self, port: &mut Box<dyn SerialPort>) -> Result<(), Error> {
        let addr_bytes = self.addr.to_be_bytes();
        port.write_all(&addr_bytes[5..])?;
        let (data_seg, pad_len) = if self.data.len() > PAGE_SIZE {
            (&self.data[..PAGE_SIZE], 0)
        } else {
            (self.data, (PAGE_SIZE - self.data.len()))
        };
        port.write_all(data_seg)?;
        if pad_len > 0 {
            port.write_all(&vec![0u8; pad_len])?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct GetVerReply(pub u8);

impl Display for GetVerReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl CmdReply for GetVerReply {
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error> {
        let mut buf = [0u8; 2];
        port.read_exact(&mut buf)?;
        if buf[0] == 38 {
            Ok(GetVerReply(buf[1]))
        } else {
            Err(Error::Cmd("Error getting version".into()))
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProgResult;

impl CmdReply for ProgResult {
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error> {
        let mut reply = [0u8; 4];
        port.read_exact(&mut reply)?;
        if reply[0] == 0 {
            Ok(ProgResult)
        } else {
            let err_data = &reply[1..];
            Err(Error::Cmd(format!(
                "prog rc {:#02x} at page + {:#02x}, {:#02x} expected, {:#02x} read.",
                reply[0], err_data[0], err_data[1], err_data[2]
            )))
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReadData {
    /// address in bytes
    pub addr: usize,
}

impl CmdArgs for ReadData {
    fn send_args(&self, port: &mut Box<dyn SerialPort>) -> Result<(), Error> {
        let addr_bytes = self.addr.to_be_bytes();
        port.write_all(&addr_bytes[5..])?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ReadResult(pub [u8; PAGE_SIZE]);

impl CmdReply for ReadResult {
    fn receive_reply(port: &mut Box<dyn SerialPort>) -> Result<Self, Error> {
        let mut rr = ReadResult([0; PAGE_SIZE]);
        port.read_exact(&mut rr.0)?;
        Ok(rr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_mocks::TestCmd;

    #[test]
    fn test_get_ver_err() {
        let port = CMD_GET_VER.test_err(vec![0, 0], &());
        assert_eq!(port.written(), vec![CMD_GET_VER.cmd]);
    }

    #[test]
    fn test_get_ver() {
        let (port, reply) = CMD_GET_VER.test_ok(vec![38, 5], &());
        assert_eq!(port.written(), vec![CMD_GET_VER.cmd]);
        assert_eq!(reply.0, 5);
    }

    #[test]
    fn test_reset() {
        let (port, reply) = CMD_RESET.test_ok(vec![1, 2, 3], &());
        assert_eq!(port.written(), vec![CMD_RESET.cmd]);
        assert_eq!(reply, [1, 2, 3]);
    }

    #[test]
    fn test_erase() {
        let (port, ()) = CMD_ERASE_64K.test_ok(vec![38], &[42]);
        assert_eq!(port.written(), vec![CMD_ERASE_64K.cmd, 42]);
    }

    #[test]
    fn test_program() {
        let content = [0; 300];
        let prog_data = ProgData {
            addr: 0x2328,
            data: &content,
        };
        let (port, _) = CMD_PROGRAM_PAGE.test_ok(vec![0; 4], &prog_data);
        let written = port.written();
        assert_eq!(written[0..4], [CMD_PROGRAM_PAGE.cmd, 0, 0x23, 0x28]);
        assert_eq!(written[4..], content[..PAGE_SIZE]);
    }

    #[test]
    fn test_release() {
        let (port, ()) = CMD_RELEASE_FPGA.test_ok(vec![0], &());
        assert_eq!(port.written(), vec![CMD_RELEASE_FPGA.cmd]);
    }
}
