use std::io::{Read, Write};

use tracing::{info, instrument};

use crate::cmds::{self, PAGE_SIZE};
use crate::err::Error;

pub struct Device<Port: Read + Write> {
    pub port: Port,
}

impl<Port: Read + Write> AsMut<Port> for Device<Port> {
    fn as_mut(&mut self) -> &mut Port {
        &mut self.port
    }
}

impl<Port: Read + Write> Device<Port> {
    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip(self))]
    fn getver(&mut self) -> Result<cmds::GetVerReply, Error> {
        cmds::CMD_GET_VER.send(self, &())
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    pub fn reset_fpga(mut self) -> Result<([u8; 3], DeviceInReset<Port>), Error> {
        let ver = cmds::CMD_RESET.send(&mut self, &())?;
        Ok((ver, DeviceInReset(self)))
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip(self))]
    pub fn prepare(mut self) -> Result<DeviceInReset<Port>, Error> {
        let ver = self.getver()?;
        info!(?ver, "iceFUN version");
        let (reset_reply, dev_in_reset) = self.reset_fpga()?;
        info!(?reset_reply, "Flash ID");
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

impl<Port: Read + Write> AsMut<Port> for DeviceInReset<Port> {
    fn as_mut(&mut self) -> &mut Port {
        &mut self.0.port
    }
}

impl<Port: Read + Write> Programmable for DeviceInReset<Port> {
    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip(self))]
    fn erase64k(&mut self, page: u8) -> Result<(), Error> {
        cmds::CMD_ERASE_64K.send(self, &[page])
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip(self, data))]
    fn program_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error> {
        cmds::CMD_PROGRAM_PAGE.send(self, &cmds::ProgData { addr, data })?;
        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip(self, data))]
    fn verify_page(&mut self, addr: usize, data: &[u8]) -> Result<(), Error> {
        cmds::CMD_VERIFY_PAGE.send(self, &cmds::ProgData { addr, data })?;
        Ok(())
    }
}

impl<Port: Read + Write> Dumpable for DeviceInReset<Port> {
    /// # Errors
    ///
    /// Will return `Err` if commnication fails, or if `addr` and `len` are out of range.
    #[instrument(skip(self, output))]
    fn read_page(&mut self, addr: usize, len: usize, output: &mut impl Write) -> Result<(), Error> {
        if len > PAGE_SIZE {
            return Err(Error::Dump(format!(
                "Reading {len} bytes of {PAGE_SIZE} byte page"
            )));
        }
        if addr + len > (1024 * 1024) {
            return Err(Error::Dump("Reading beyond 1MB".to_string()));
        }
        let data = cmds::CMD_READ_PAGE.send(self, &cmds::ReadData { addr })?;
        output.write_all(&data.0[..len])?;
        Ok(())
    }
}

impl<Port: Read + Write> Drop for DeviceInReset<Port> {
    fn drop(&mut self) {
        cmds::CMD_RELEASE_FPGA.send(self, &()).ok();
    }
}
