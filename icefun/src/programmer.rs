use std::cmp::min;
use std::fs::File;
use std::{fs, path::Path, usize};

use log::info;

use crate::dev::{Dumpable, Programmable};
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

    fn do_pages(
        &self,
        action_name: &str,
        mut action: impl FnMut(usize, &[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut write_addr: usize = 0;
        let end_addr = self.data.len();

        let progress = |addr: usize| {
            info!("{} {}% ", action_name, (100 * addr) / end_addr);
        };

        progress(0);

        while write_addr < end_addr {
            action(write_addr, &self.data[write_addr..])?;
            write_addr += 256;
            if (write_addr % 10240) == 0 {
                progress(write_addr);
            }
        }
        progress(end_addr);
        Ok(())
    }

    pub fn program(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.do_pages("Programming", |addr, data| fpga.program_page(addr, data))
    }

    pub fn verify(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.do_pages("Verifying", |addr, data| fpga.verify_page(addr, data))
    }
}

pub struct FPGADump {
    file: File,
}

impl FPGADump {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = std::fs::File::create(path)?;
        Ok(Self { file })
    }

    pub fn dump(
        &mut self,
        fpga: &mut impl Dumpable,
        offset: usize,
        size: usize,
    ) -> Result<(), Error> {
        let mut read_addr = offset;
        let end_addr = offset + size;
        while read_addr < end_addr {
            let len = min(256usize, end_addr - read_addr);
            fpga.read_page(read_addr, len, &mut self.file)?;
            read_addr += 256;
        }
        Ok(())
    }
}
