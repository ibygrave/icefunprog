use anyhow::Result;
use log::info;
use serialport::SerialPort;

use crate::cmds;

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
        REPLY::receive_reply(&mut self.port)
    }

    pub fn getver(&mut self) -> Result<u8> {
        Ok(self.send_cmd::<(), [u8; 1]>(cmds::CMD_GET_VER, ())?[0])
    }

    pub fn reset_fpga(&mut self) -> Result<[u8; 3]> {
        self.send_cmd(cmds::CMD_RESET, ())
    }

    pub fn prepare(&mut self) -> Result<()> {
        let ver = self.getver()?;
        info!("iceFUN v{}", ver);
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

    pub fn program_page(&mut self, cmd: u8, args: cmds::ProgData) -> Result<()> {
        let prog_result: [u8; 4] = self.send_cmd(cmd, args)?;
        check_prog_result(&prog_result)
    }

    pub fn release_fpga(&mut self) -> Result<()> {
        self.send_cmd(cmds::CMD_RELEASE_FPGA, ())
    }
}
