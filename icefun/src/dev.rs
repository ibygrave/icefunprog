use std::io::{BufRead, BufReader, Read, Write};

use log::{debug, info, trace};

use crate::cmds;
use crate::err::Error;

pub struct Device<PORT>
where
    PORT: Read + Write,
{
    pub port: PORT,
}

impl<PORT> Device<PORT>
where
    PORT: Read + Write,
{
    fn send_cmd<ARG: cmds::CmdArgs, REPLY: cmds::CmdReply>(
        &mut self,
        cmd: u8,
        args: ARG,
    ) -> Result<REPLY, Error> {
        self.port.write_all(&[cmd])?;
        if log::log_enabled!(log::Level::Trace) {
            let mut arg_buf: Vec<u8> = vec![];
            args.send_args(&mut arg_buf)?;
            trace!("Send command: {:02x} {:02x?}", cmd, arg_buf);
            self.port.write_all(&arg_buf)?;
        } else {
            args.send_args(&mut self.port)?;
        }
        let mut reader = BufReader::new(&mut self.port);
        let data = reader.fill_buf()?;
        trace!("Receive reply: {:02x?}", data);
        let reply = REPLY::receive_reply(&mut reader)?;
        let remain = reader.buffer();
        if !remain.is_empty() {
            debug!("Unread reply: {:02x?}", remain);
        }
        Ok(reply)
    }

    fn getver(&mut self) -> Result<cmds::GetVerReply, Error> {
        self.send_cmd(cmds::CMD_GET_VER, ())
    }

    pub fn reset_fpga(mut self) -> Result<([u8; 3], DeviceInReset<PORT>), Error> {
        let ver = self.send_cmd(cmds::CMD_RESET, ())?;
        Ok((ver, DeviceInReset(self)))
    }

    pub fn prepare(mut self) -> Result<DeviceInReset<PORT>, Error> {
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
    fn program_page(&mut self, cmd: u8, addr: usize, data: &[u8]) -> Result<(), Error>;
}

pub struct DeviceInReset<PORT>(Device<PORT>)
where
    PORT: Read + Write;

impl<PORT> Programmable for DeviceInReset<PORT>
where
    PORT: Read + Write,
{
    fn erase64k(&mut self, page: u8) -> Result<(), Error> {
        self.0.send_cmd(cmds::CMD_ERASE_64K, [page])
    }

    fn program_page(&mut self, cmd: u8, addr: usize, data: &[u8]) -> Result<(), Error> {
        let _: cmds::ProgResult = self.0.send_cmd(cmd, cmds::ProgData { addr, data })?;
        Ok(())
    }
}

impl<PORT> Drop for DeviceInReset<PORT>
where
    PORT: Read + Write,
{
    fn drop(&mut self) {
        self.0.send_cmd::<(), ()>(cmds::CMD_RELEASE_FPGA, ()).ok();
    }
}
