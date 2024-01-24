use std::{fs, path::Path, usize};

use log::info;

use crate::cmds;
use crate::dev::Programmable;
use crate::err::Error;

pub struct FPGAData {
    pub data: Vec<u8>,
    pub start_page: u8,
    pub end_page: u8,
}

impl FPGAData {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
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

    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        for page in self.start_page..=self.end_page {
            info!("Erasing sector {:#02x}0000", page);
            fpga.erase64k(page)?;
        }
        Ok(())
    }

    fn do_pages(&self, fpga: &mut impl Programmable, cmd: u8, action: &str) -> Result<(), Error> {
        let mut write_addr: usize = 0;
        let end_addr = self.data.len();

        let progress = |addr: usize| {
            info!("{} {}% ", action, (100 * addr) / end_addr);
        };

        progress(0);

        while write_addr < end_addr {
            fpga.program_page(cmd, write_addr, &self.data[write_addr..])?;
            write_addr += 256;
            if (write_addr % 10240) == 0 {
                progress(write_addr);
            }
        }
        progress(end_addr);
        Ok(())
    }

    pub fn program(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.do_pages(fpga, cmds::CMD_PROGRAM_PAGE, "Programming")
    }

    pub fn verify(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.do_pages(fpga, cmds::CMD_VERIFY_PAGE, "Verifying")
    }
}
