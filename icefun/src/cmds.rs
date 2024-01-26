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

    pub(crate) fn send(&self, port: &mut (impl Read + Write), args: Args) -> Result<Reply, Error> {
        port.write_all(&[self.cmd])?;
        if log::log_enabled!(log::Level::Trace) {
            let mut arg_buf: Vec<u8> = vec![];
            args.send_args(&mut arg_buf)?;
            trace!("Send command: {:02x} {:02x?}", self.cmd, arg_buf);
            port.write_all(&arg_buf)?;
        } else {
            args.send_args(port)?;
        }
        let mut reader = BufReader::new(port);
        let data = reader.fill_buf()?;
        trace!("Receive reply: {:02x?}", data);
        let reply = Reply::receive_reply(&mut reader)?;
        let remain = reader.buffer();
        if !remain.is_empty() {
            debug!("Unread reply: {:02x?}", remain);
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
