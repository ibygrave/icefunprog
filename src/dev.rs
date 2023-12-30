use anyhow::Result;
use log::info;
use serialport::SerialPort;

use crate::cmds;

pub struct Device {
    pub port: Box<dyn SerialPort>,
}

impl Device {
    fn send_cmd<ARG: cmds::CmdArgs, REPLY: cmds::CmdReply>(
        &mut self,
        cmd: u8,
        args: ARG,
    ) -> Result<REPLY> {
        self.port.clear(serialport::ClearBuffer::All)?;
        self.port.write_all(&[cmd])?;
        args.send_args(&mut self.port)?;
        let reply = REPLY::receive_reply(&mut self.port)?;
        Ok(reply)
    }

    pub fn getver(&mut self) -> Result<cmds::GetVerReply> {
        self.send_cmd(cmds::CMD_GET_VER, ())
    }

    pub fn reset_fpga(mut self) -> Result<([u8; 3], DeviceInReset)> {
        let ver = self.send_cmd(cmds::CMD_RESET, ())?;
        Ok((ver, DeviceInReset(self)))
    }

    pub fn prepare(mut self) -> Result<DeviceInReset> {
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

pub struct DeviceInReset(Device);

impl DeviceInReset {
    pub fn erase64k(&mut self, page: u8) -> Result<()> {
        self.0.send_cmd(cmds::CMD_ERASE_64K, [page])
    }

    pub fn program_page(&mut self, cmd: u8, args: cmds::ProgData) -> Result<cmds::ProgResult> {
        self.0.send_cmd(cmd, args)
    }
}

impl Drop for DeviceInReset {
    fn drop(&mut self) {
        self.0.send_cmd::<(), ()>(cmds::CMD_RELEASE_FPGA, ()).ok();
    }
}
