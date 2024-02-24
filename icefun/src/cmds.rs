use std::{
    io::{BufRead, BufReader, Read, Write},
    marker::PhantomData,
};

use log::{debug, trace};

use crate::err::Error;

pub(crate) const CMD_GET_VER: Command<(), GetVerReply> = Command::new(0xb1);
pub(crate) const CMD_RESET: Command<(), [u8; 3]> = Command::new(0xb2);
pub(crate) const CMD_ERASE_64K: Command<[u8; 1], ()> = Command::new(0xb4);
pub(crate) const CMD_PROGRAM_PAGE: Command<ProgData, ProgResult> = Command::new(0xb5);
pub(crate) const CMD_READ_PAGE: Command<ReadData, ReadResult> = Command::new(0xb6);
pub(crate) const CMD_VERIFY_PAGE: Command<ProgData, ProgResult> = Command::new(0xb7);
pub(crate) const CMD_RELEASE_FPGA: Command<(), ()> = Command::new(0xb9);

pub(crate) trait CmdArgs {
    fn send_args(&self, writer: &mut impl Write) -> Result<(), Error>;
}

impl CmdArgs for () {
    fn send_args(&self, _writer: &mut impl Write) -> Result<(), Error> {
        // Sends zero bytes
        Ok(())
    }
}

impl<const LEN: usize> CmdArgs for [u8; LEN] {
    fn send_args(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(self)?;
        Ok(())
    }
}

pub(crate) trait CmdReply
where
    Self: Sized,
{
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error>;
}

impl CmdReply for () {
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0u8];
        reader.read_exact(&mut buf)?;
        Ok(())
    }
}

impl<const LEN: usize> CmdReply for [u8; LEN] {
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0u8; LEN];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

pub(crate) struct Command<Args: CmdArgs, Reply: CmdReply> {
    cmd: u8,
    _args: PhantomData<Args>,
    _reply: PhantomData<Reply>,
}

impl<Args: CmdArgs, Reply: CmdReply> Command<Args, Reply> {
    const fn new(cmd: u8) -> Self {
        Self {
            cmd,
            _args: PhantomData,
            _reply: PhantomData,
        }
    }

    pub(crate) fn send<Port, R, W>(&self, port: &mut Port, args: &Args) -> Result<Reply, Error>
    where
        R: Read,
        W: Write,
        Port: AsMut<R> + AsMut<W>,
    {
        let writer = <Port as AsMut<W>>::as_mut(port);
        writer.write_all(&[self.cmd])?;
        if log::log_enabled!(log::Level::Trace) {
            let mut arg_buf: Vec<u8> = vec![];
            args.send_args(&mut arg_buf)?;
            trace!("Send command: {:02x} {arg_buf:02x?}", self.cmd);
            writer.write_all(&arg_buf)?;
        } else {
            args.send_args(writer)?;
        }
        let mut reader = BufReader::new(<Port as AsMut<R>>::as_mut(port));
        let data = reader.fill_buf()?;
        trace!("Receive reply: {data:02x?}");
        let reply = Reply::receive_reply(&mut reader)?;
        let remain = reader.buffer();
        if !remain.is_empty() {
            debug!("Unread reply: {remain:02x?}");
        }
        Ok(reply)
    }
}

pub(crate) struct ProgData<'a> {
    pub addr: usize,
    pub data: &'a [u8],
}

impl CmdArgs for ProgData<'_> {
    fn send_args(&self, writer: &mut impl Write) -> Result<(), Error> {
        let addr_bytes = self.addr.to_be_bytes();
        writer.write_all(&addr_bytes[5..])?;
        let (data_seg, pad_len) = if self.data.len() > 256 {
            (&self.data[..256], 0)
        } else {
            (self.data, (256 - self.data.len()))
        };
        writer.write_all(data_seg)?;
        if pad_len > 0 {
            writer.write_all(&vec![0u8; pad_len])?;
        }
        Ok(())
    }
}

pub(crate) struct GetVerReply(pub u8);

impl CmdReply for GetVerReply {
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        if buf[0] == 38 {
            Ok(GetVerReply(buf[1]))
        } else {
            Err(Error::Cmd("Error getting version".into()))
        }
    }
}

pub(crate) struct ProgResult;

impl CmdReply for ProgResult {
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error> {
        let mut rc = [0u8];
        reader.read_exact(&mut rc)?;
        if rc[0] == 0 {
            Ok(ProgResult)
        } else {
            let mut err_data = [0u8; 3];
            reader.read_exact(&mut err_data)?;
            Err(Error::Cmd(format!(
                "prog rc {:#02x} at page + {:#02x}, {:#02x} expected, {:#02x} read.",
                rc[0], err_data[0], err_data[1], err_data[2]
            )))
        }
    }
}

pub(crate) struct ReadData {
    /// address in bytes
    pub addr: usize,
}

impl CmdArgs for ReadData {
    fn send_args(&self, writer: &mut impl Write) -> Result<(), Error> {
        let addr_bytes = self.addr.to_be_bytes();
        writer.write_all(&addr_bytes[5..])?;
        Ok(())
    }
}

pub(crate) struct ReadResult(pub [u8; 256]);

impl CmdReply for ReadResult {
    fn receive_reply(reader: &mut impl Read) -> Result<Self, Error> {
        let mut rr = ReadResult([0; 256]);
        reader.read_exact(&mut rr.0)?;
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
        let (port, _) = CMD_PROGRAM_PAGE.test_ok(vec![0], &prog_data);
        let written = port.written();
        assert_eq!(written[0..4], [CMD_PROGRAM_PAGE.cmd, 0, 0x23, 0x28]);
        assert_eq!(written[4..], content[..256]);
    }

    #[test]
    fn test_release() {
        let (port, ()) = CMD_RELEASE_FPGA.test_ok(vec![0], &());
        assert_eq!(port.written(), vec![CMD_RELEASE_FPGA.cmd]);
    }
}
