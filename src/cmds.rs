use anyhow::Result;

pub const CMD_GET_VER: u8 = 0xb1;
pub const CMD_RESET: u8 = 0xb2;
pub const CMD_ERASE_64K: u8 = 0xb4;
pub const CMD_PROGRAM_PAGE: u8 = 0xb5;
pub const CMD_VERIFY_PAGE: u8 = 0xb7;
pub const CMD_RELEASE_FPGA: u8 = 0xb9;

pub trait CmdArgs {
    fn send_args(&self, _writer: &mut dyn std::io::Write) -> Result<()> {
        // Default implementation sends zero bytes
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

// Unit type used as Cmd::Args by commands with no arguments
impl CmdArgs for () {}

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
