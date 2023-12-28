use core::cmp::min;
use std::{fs, io::Write, path::Path, usize};

use anyhow::Result;

use crate::cmds;
use crate::dev;

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

    pub fn erase(&self, fpga: &mut dev::Device) -> Result<()> {
        for page in self.start_page..=self.end_page {
            println!("Erasing sector {:#02x}0000", page);
            fpga.erase64k(page)?;
        }
        Ok(())
    }

    fn do_pages(&self, fpga: &mut dev::Device, cmd: u8, action: &str) -> Result<()> {
        let mut write_addr: usize = 0;
        let end_addr = self.data.len();

        print!("{} ", action);

        while write_addr < end_addr {
            let seg_len = min(256, end_addr - write_addr);
            let mut data_seg = [0u8; 256];
            data_seg[..seg_len].clone_from_slice(&self.data[write_addr..(write_addr + seg_len)]);
            let prog_data = cmds::ProgData {
                addr: write_addr,
                data: data_seg,
            };
            fpga.program_page(cmd, prog_data)?;
            write_addr += 256;
            if (write_addr % 2560) == 0 {
                print!(".");
                std::io::stdout().flush()?;
            }
        }
        println!();
        Ok(())
    }

    pub fn program(&self, fpga: &mut dev::Device) -> Result<()> {
        self.do_pages(fpga, cmds::CMD_PROGRAM_PAGE, "Programming")
    }

    pub fn verify(&self, fpga: &mut dev::Device) -> Result<()> {
        self.do_pages(fpga, cmds::CMD_VERIFY_PAGE, "Verifying")
    }
}
