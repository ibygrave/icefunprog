use crate::err::Error;

pub const CMD_GET_VER: u8 = 0xb1;
pub const CMD_RESET: u8 = 0xb2;
pub const CMD_ERASE_64K: u8 = 0xb4;
pub const CMD_PROGRAM_PAGE: u8 = 0xb5;
pub const CMD_VERIFY_PAGE: u8 = 0xb7;
pub const CMD_RELEASE_FPGA: u8 = 0xb9;

pub trait CmdArgs {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<(), Error>;
}

impl CmdArgs for () {
    fn send_args(&self, _writer: &mut dyn std::io::Write) -> Result<(), Error> {
        // Sends zero bytes
        Ok(())
    }
}

impl<const LEN: usize> CmdArgs for [u8; LEN] {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<(), Error> {
        writer.write_all(self)?;
        Ok(())
    }
}

pub trait CmdReply
where
    Self: Sized,
{
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self, Error>;
}

impl CmdReply for () {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self, Error> {
        let mut buf = [0u8];
        reader.read_exact(&mut buf)?;
        Ok(())
    }
}

impl<const LEN: usize> CmdReply for [u8; LEN] {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self, Error> {
        let mut buf = [0u8; LEN];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

pub struct ProgData<'a> {
    pub addr: usize,
    pub data: &'a [u8],
}

impl CmdArgs for ProgData<'_> {
    fn send_args(&self, writer: &mut dyn std::io::Write) -> Result<(), Error> {
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

pub struct GetVerReply(pub u8);

impl CmdReply for GetVerReply {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self, Error> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        if buf[0] == 38 {
            Ok(GetVerReply(buf[1]))
        } else {
            Err(Error::Cmd("Error getting version".into()))
        }
    }
}

pub struct ProgResult;

impl CmdReply for ProgResult {
    fn receive_reply(reader: &mut dyn std::io::Read) -> Result<Self, Error> {
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
