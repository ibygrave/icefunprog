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
        self.port.write_all(&[cmd])?;
        args.send_args(&mut self.port)?;
        let reply = REPLY::receive_reply(&mut self.port)?;
        self.port.clear(serialport::ClearBuffer::All)?;
        Ok(reply)
    }

    pub fn getver(&mut self) -> Result<cmds::GetVerReply> {
        self.send_cmd(cmds::CMD_GET_VER, ())
    }

    pub fn reset_fpga(&mut self) -> Result<[u8; 3]> {
        self.send_cmd(cmds::CMD_RESET, ())
    }

    pub fn prepare(&mut self) -> Result<()> {
        let ver = self.getver()?;
        info!("iceFUN v{}", ver.0);
        let reset_reply = self.reset_fpga()?;
        info!(
            "Flash ID {:#02x} {:#02x} {:#02x}",
            reset_reply[0], reset_reply[1], reset_reply[2]
        );
        Ok(())
    }

    pub fn erase64k(&mut self, page: u8) -> Result<()> {
        self.send_cmd(cmds::CMD_ERASE_64K, [page])
    }

    pub fn program_page(&mut self, cmd: u8, args: cmds::ProgData) -> Result<cmds::ProgResult> {
        self.send_cmd(cmd, args)
    }

    pub fn release_fpga(&mut self) -> Result<()> {
        self.send_cmd(cmds::CMD_RELEASE_FPGA, ())
    }
}
