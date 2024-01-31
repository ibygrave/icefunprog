use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{fs, path::Path, usize};

use log::info;

use crate::dev::{Dumpable, Programmable};
use crate::err::Error;

#[derive(Copy, Clone, Debug)]
struct Range {
    start: usize,
    len: usize,
}

impl Range {
    fn sectors(&self) -> impl Iterator<Item = u8> {
        let start_sector = (self.start >> 16) as u8;
        let end_sector = ((self.start + self.len) >> 16) as u8;
        start_sector..=end_sector
    }

    fn pages<const N: usize>(&self) -> impl Iterator<Item = Range> + '_ {
        let count_pages = 1 + ((self.len - 1) / N);
        let end_addr = self.start + self.len;
        (0..count_pages).map(move |page| {
            let start = self.start + (page * N);
            Range {
                start,
                len: min(N, end_addr - start),
            }
        })
    }
}

pub struct FPGAProg<R: Read + Seek> {
    reader: R,
    range: Range,
}

impl FPGAProg<File> {
    pub fn from_path(path: impl AsRef<Path>, offset: usize) -> Result<Self, Error> {
        let meta = fs::metadata(&path)?;
        let file = File::open(&path)?;
        Ok(Self {
            reader: file,
            range: Range {
                start: offset,
                len: meta.len() as usize,
            },
        })
    }
}

impl<R: Read + Seek> FPGAProg<R> {
    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        for sector in self.range.sectors() {
            info!("Erasing sector {:#02x}0000", sector);
            fpga.erase64k(sector)?;
        }
        Ok(())
    }

    fn do_pages(
        &mut self,
        action_name: &str,
        mut action: impl FnMut(usize, &[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buf = [0u8; 256];

        let progress = |addr: usize| {
            info!("{} {}% ", action_name, (100 * addr) / self.range.len);
        };

        for Range { start, len } in self.range.pages::<256>() {
            let part_buf = &mut buf[..len];
            self.reader.read_exact(part_buf)?;
            action(start, part_buf)?;
            if (start % 10240) == 0 {
                progress(start);
            }
        }
        progress(self.range.len);
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
    range: Range,
}

impl FPGADump<File> {
    pub fn from_path(path: impl AsRef<Path>, offset: usize, size: usize) -> Result<Self, Error> {
        let file = std::fs::File::create(path)?;
        Ok(Self {
            writer: file,
            range: Range {
                start: offset,
                len: size,
            },
        })
    }
}

impl<W: Write> FPGADump<W> {
    pub fn dump(&mut self, fpga: &mut impl Dumpable) -> Result<(), Error> {
        for Range { start, len } in self.range.pages::<256>() {
            fpga.read_page(start, len, &mut self.writer)?;
        }
        Ok(())
    }
}
