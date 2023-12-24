use core::cmp::min;
use std::{fs, path::Path, usize};

use anyhow::Result;
use log::{error, info};
use serialport::SerialPort;

use crate::cmds;
use crate::cmds::Cmd;

pub struct FPGAData {
    pub data: Vec<u8>,
    pub start_page: u8,
    pub end_page: u8,
}

impl FPGAData {
    pub fn from_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let data = fs::read(path)?;
        let length = data.len();
        let start_page = 0u8;
        let end_page = ((length >> 16) + 1) as u8;
        Ok(Self {
            data,
            start_page,
            end_page,
        })
    }

    pub fn prepare(&self, port: &mut Box<dyn SerialPort>) -> Result<()> {
        let ver = cmds::GetVer::send(port, ())?;
        info!("iceFUN v{}", ver.0);
        let reset_reply = cmds::ResetFPGA::send(port, ())?;
        info!(
            "Flash ID {:#02x} {:#02x} {:#02x}",
            reset_reply.0[0], reset_reply.0[1], reset_reply.0[2]
        );
        Ok(())
    }

    pub fn erase(&self, port: &mut Box<dyn SerialPort>) -> Result<()> {
        for page in self.start_page..=self.end_page {
            println!("Erasing sector {:#02x}0000", page);
            cmds::Erase64K::send(port, cmds::ErasePage(page))?;
        }
        Ok(())
    }

    fn do_pages<T: cmds::Cmd<Args = cmds::ProgData>>(
        &self,
        port: &mut Box<dyn SerialPort>,
        action: &str,
    ) -> Result<()> {
        let mut write_addr: usize = 0;
        let end_addr = self.data.len();

        println!("{}", action);

        while write_addr < end_addr {
            let seg_len = min(256, end_addr - write_addr);
            let mut data_seg = [0u8; 256];
            data_seg[..seg_len].clone_from_slice(&self.data[write_addr..(write_addr + seg_len)]);
            let prog_data = cmds::ProgData {
                addr: write_addr,
                data: data_seg,
            };
            T::send(port, prog_data).map_err(|err| {
                error!("{} failed at {:#08x}", action, write_addr);
                err
            })?;
            write_addr += 256;
        }
        Ok(())
    }

    pub fn program(&self, port: &mut Box<dyn SerialPort>) -> Result<()> {
        self.do_pages::<cmds::ProgPage>(port, "Programming")
    }

    pub fn verify(&self, port: &mut Box<dyn SerialPort>) -> Result<()> {
        self.do_pages::<cmds::VerifyPage>(port, "Verifying")
    }
}
