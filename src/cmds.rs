use std::usize;

use anyhow::Result;
use serialport::SerialPort;

pub trait CmdArgs {
    fn send_args(&self, _writer: &mut dyn std::io::Write) -> Result<()> {
        // Default implementation sends zero bytes
        Ok(())
    }
}

pub trait CmdReply
where
    Self: Sized,
{
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self>;
}

// Default reply reads one byte and ignores it
impl<T> CmdReply for T
where
    T: Default,
{
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8];
        reader.read_exact(&mut buf)?;
        Ok(Self::default())
    }
}

pub trait Cmd: CmdReply {
    const CMD_CODE: u8;
    type Args: CmdArgs;

    fn send(port: &mut Box<dyn SerialPort>, args: Self::Args) -> Result<Self> {
        port.write_all(&[Self::CMD_CODE; 1])?;
        args.send_args(port)?;
        Self::receive_reply(port)
    }
}

// Unit type used as Cmd::Args by commands with no arguments
impl CmdArgs for () {}

pub struct GetVer(pub u8);

impl CmdReply for GetVer {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        if buf[0] != 38 {
            anyhow::bail!("Error getting version");
        }
        Ok(Self(buf[1]))
    }
}

impl Cmd for GetVer {
    const CMD_CODE: u8 = 0xb1;
    type Args = ();
}

pub struct ResetFPGA(pub [u8; 3]);

impl CmdReply for ResetFPGA {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

impl Cmd for ResetFPGA {
    const CMD_CODE: u8 = 0xb2;
    type Args = ();
}

pub struct ErasePage(pub u8);

impl CmdArgs for ErasePage {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<()> {
        writer.write_all(&[self.0])?;
        Ok(())
    }
}

#[derive(Default)]
pub struct Erase64K;

impl Cmd for Erase64K {
    const CMD_CODE: u8 = 0xb4;
    type Args = ErasePage;
}

pub struct ProgData {
    pub addr: usize,
    pub data: [u8; 256],
}

impl CmdArgs for ProgData {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<()> {
        let addr_bytes = self.addr.to_be_bytes();
        writer.write_all(&addr_bytes[5..])?;
        writer.write_all(&self.data)?;
        Ok(())
    }
}

fn check_prog_result(reply_buf: &[u8; 4]) -> Result<()> {
    if reply_buf[0] != 0 {
        anyhow::bail!(
            "at page + {:02x}, {:#02x} expected, {:#02x} read.",
            reply_buf[1],
            reply_buf[2],
            reply_buf[3]
        );
    }
    Ok(())
}

pub struct ProgPage;

impl CmdReply for ProgPage {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        check_prog_result(&buf)?;
        Ok(Self)
    }
}

impl Cmd for ProgPage {
    const CMD_CODE: u8 = 0xb5;
    type Args = ProgData;
}

#[derive(Default)]
pub struct ReleaseFPGA;

impl Cmd for ReleaseFPGA {
    const CMD_CODE: u8 = 0xb9;
    type Args = ();
}

pub struct VerifyPage;

impl CmdReply for VerifyPage {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        check_prog_result(&buf)?;
        Ok(Self)
    }
}

impl Cmd for VerifyPage {
    const CMD_CODE: u8 = 0xb7;
    type Args = ProgData;
}
