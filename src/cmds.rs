use anyhow::{bail, Result};
use log::debug;

pub const CMD_GET_VER: u8 = 0xb1;
pub const CMD_RESET: u8 = 0xb2;
pub const CMD_ERASE_64K: u8 = 0xb4;
pub const CMD_PROGRAM_PAGE: u8 = 0xb5;
pub const CMD_VERIFY_PAGE: u8 = 0xb7;
pub const CMD_RELEASE_FPGA: u8 = 0xb9;

pub trait CmdArgs {
    fn send_args(&self, _writer: &mut dyn std::io::Write) -> Result<()>;
}

impl CmdArgs for () {
    fn send_args(&self, _writer: &mut dyn std::io::Write) -> Result<()> {
        // Sends zero bytes
        Ok(())
    }
}

impl<const LEN: usize> CmdArgs for [u8; LEN] {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<()> {
        writer.write_all(self)?;
        Ok(())
    }
}

pub trait CmdReply
where
    Self: Sized,
{
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self>;
}

impl CmdReply for () {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8];
        reader.read_exact(&mut buf)?;
        Ok(())
    }
}

impl<const LEN: usize> CmdReply for [u8; LEN] {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; LEN];
        reader.read_exact(&mut buf)?;
        debug!("Reply bytes {:?}", buf);
        Ok(buf)
    }
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

pub struct GetVerReply(pub u8);

impl CmdReply for GetVerReply {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        if buf[0] == 38 {
            Ok(GetVerReply(buf[1]))
        } else {
            bail!("Error getting version");
        }
    }
}

pub struct ProgResult;

impl CmdReply for ProgResult {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self> {
        let mut rc = [0u8];
        reader.read_exact(&mut rc)?;
        match rc[0] {
            0 => Ok(ProgResult),
            _ => {
                let mut err_data = [0u8; 3];
                reader.read_exact(&mut err_data)?;
                bail!(
                    "prog rc {:#02x} at page + {:#02x}, {:#02x} expected, {:#02x} read.",
                    rc[0],
                    err_data[0],
                    err_data[1],
                    err_data[2]
                );
            }
        }
    }
}
