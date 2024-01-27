use std::io::{Read, Write};

use log::info;

use crate::cmds;
use crate::err::Error;

pub struct Device<Port: Read + Write> {
    pub port: Port,
}

impl<Port: Read + Write> Device<Port> {
    fn getver(&mut self) -> Result<cmds::GetVerReply, Error> {
        cmds::CMD_GET_VER.send(&mut self.port, ())
    }

    pub fn reset_fpga(mut self) -> Result<([u8; 3], DeviceInReset<Port>), Error> {
        let ver = cmds::CMD_RESET.send(&mut self.port, ())?;
        Ok((ver, DeviceInReset(self)))
    }

    pub fn prepare(mut self) -> Result<DeviceInReset<Port>, Error> {
        let ver = self.getver()?;
        info!("iceFUN v{}", ver.0);
        let (reset_reply, dev_in_reset) = self.reset_fpga()?;
        info!(
            "Flash ID {:#02x} {:#02x} {:#02x}",
            reset_reply[0], reset_reply[1], reset_reply[2]
        );
        Ok(dev_in_reset)
    }
}

pub trait Programmable {
    fn erase64k(&mut self, page: u8) -> Result<(), Error>;
    fn program_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error>;
    fn verify_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error>;
}

pub trait Dumpable {
    fn read_page(&mut self, addr: usize, len: usize, output: &mut impl Write) -> Result<(), Error>;
}

pub struct DeviceInReset<Port: Read + Write>(pub Device<Port>);

impl<Port: Read + Write> Programmable for DeviceInReset<Port> {
    fn erase64k(&mut self, page: u8) -> Result<(), Error> {
        cmds::CMD_ERASE_64K.send(&mut self.0.port, [page])
    }

    fn program_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error> {
        cmds::CMD_PROGRAM_PAGE.send(&mut self.0.port, cmds::ProgData { addr, data })?;
        Ok(())
    }

    fn verify_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error> {
        cmds::CMD_VERIFY_PAGE.send(&mut self.0.port, cmds::ProgData { addr, data })?;
        Ok(())
    }
}

impl<Port: Read + Write> Dumpable for DeviceInReset<Port> {
    fn read_page(&mut self, addr: usize, len: usize, output: &mut impl Write) -> Result<(), Error> {
        if len > 256 {
            return Err(Error::Dump(format!(
                "Reading {} bytes of 256 byte page",
                len
            )));
        }
        if addr + len > (1024 * 1024) {
            return Err(Error::Dump("Reading beyond 1MB".to_string()));
        }
        let data = cmds::CMD_READ_PAGE.send(&mut self.0.port, cmds::ReadData { addr })?;
        output.write_all(&data.0[..len])?;
        Ok(())
    }
}

impl<Port: Read + Write> Drop for DeviceInReset<Port> {
    fn drop(&mut self) {
        cmds::CMD_RELEASE_FPGA.send(&mut self.0.port, ()).ok();
    }
}
