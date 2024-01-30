use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{fs, path::Path, usize};

use log::info;

use crate::dev::{Dumpable, Programmable};
use crate::err::Error;

pub struct FPGAProg<R: Read + Seek> {
    reader: R,
    len: usize,
    start_page: u8,
    end_page: u8,
}

impl FPGAProg<File> {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let meta = fs::metadata(&path)?;
        let file = File::open(&path)?;
        let length = meta.len();
        let start_page = 0u8;
        let end_page = ((length >> 16) + 1) as u8;
        Ok(Self {
            reader: file,
            len: length as usize,
            start_page,
            end_page,
        })
    }
}

impl<R: Read + Seek> FPGAProg<R> {
    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        for page in self.start_page..=self.end_page {
            info!("Erasing sector {:#02x}0000", page);
            fpga.erase64k(page)?;
        }
        Ok(())
    }

    fn do_pages(
        &mut self,
        action_name: &str,
        mut action: impl FnMut(usize, &[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buf = [0u8; 256];
        let mut write_addr: usize = 0;
        let end_addr = self.len;

        let progress = |addr: usize| {
            info!("{} {}% ", action_name, (100 * addr) / end_addr);
        };

        progress(0);

        while write_addr < end_addr {
            let read_len = min(256, end_addr - write_addr);
            let part_buf = &mut buf[..read_len];
            self.reader.read_exact(part_buf)?;
            action(write_addr, part_buf)?;
            write_addr += 256;
            if (write_addr % 10240) == 0 {
                progress(write_addr);
            }
        }
        progress(end_addr);
        Ok(())
    }

    pub fn program(&mut self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.reader.seek(SeekFrom::Start(0))?;
        self.do_pages("Programming", |addr, data| fpga.program_page(addr, data))
    }

    pub fn verify(&mut self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.reader.seek(SeekFrom::Start(0))?;
        self.do_pages("Verifying", |addr, data| fpga.verify_page(addr, data))
    }
}

pub struct FPGADump<W: Write> {
    writer: W,
}

impl FPGADump<File> {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = std::fs::File::create(path)?;
        Ok(Self { writer: file })
    }
}

impl<W: Write> FPGADump<W> {
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
            fpga.read_page(read_addr, len, &mut self.writer)?;
            read_addr += 256;
        }
        Ok(())
    }
}
