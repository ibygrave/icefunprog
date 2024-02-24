use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};
use std::{fs, path::Path, usize};

use log::info;

use crate::cmds::PAGE_SIZE;
use crate::dev::{Dumpable, Programmable};
use crate::err::Error;

#[derive(Copy, Clone, Debug)]
struct Range {
    start: usize,
    len: usize,
}

const REPORT_PERIOD: Duration = Duration::from_secs(1);

impl Range {
    fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }
    /// # Errors
    ///
    /// Will return `Err` if addresses are out of range.
    fn sectors(&self) -> Result<impl Iterator<Item = u8>, Error> {
        let start_sector = u8::try_from(self.start >> 16)?;
        let end_sector = u8::try_from((self.start + self.len) >> 16)?;
        Ok(start_sector..=end_sector)
    }

    fn pages<'a, const N: usize>(&'a self, action: &'a str) -> impl Iterator<Item = Range> + '_ {
        let count_pages = 1 + ((self.len - 1) / N);
        let end_addr = self.start + self.len;
        let mut last_tick = Instant::now();
        (0..count_pages).map(move |page| {
            let now = Instant::now();
            if now.duration_since(last_tick) >= REPORT_PERIOD || (1 + page) == count_pages {
                info!("{action} {}%", (100 * (1 + page)) / count_pages);
                last_tick = now;
            }
            let start = self.start + (page * N);
            Range::new(start, min(N, end_addr - start))
        })
    }
}

pub struct FPGAProg<R: Read + Seek> {
    reader: R,
    range: Range,
}

impl FPGAProg<File> {
    /// # Errors
    ///
    /// Will return `Err` if the path cannot be accessed.
    pub fn from_path(path: impl AsRef<Path>, offset: usize) -> Result<Self, Error> {
        let meta = fs::metadata(&path)?;
        let file = File::open(&path)?;
        Ok(Self {
            reader: file,
            range: Range::new(offset, usize::try_from(meta.len())?),
        })
    }
}

impl<R: Read + Seek> FPGAProg<R> {
    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        for sector in self.range.sectors()? {
            info!("Erasing sector {sector:#02x}0000");
            fpga.erase64k(sector)?;
        }
        Ok(())
    }

    fn do_pages(
        &mut self,
        action_name: &str,
        mut action: impl FnMut(usize, &[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buf = [0u8; PAGE_SIZE];
        for Range { start, len } in self.range.pages::<PAGE_SIZE>(action_name) {
            let part_buf = &mut buf[..len];
            self.reader.read_exact(part_buf)?;
            action(start, part_buf)?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    pub fn program(&mut self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.reader.seek(SeekFrom::Start(0))?;
        self.do_pages("Programming", |addr, data| fpga.program_page(addr, data))
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
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
    /// # Errors
    ///
    /// Will return `Err` if the path cannot be accessed.
    pub fn from_path(path: impl AsRef<Path>, offset: usize, size: usize) -> Result<Self, Error> {
        let file = std::fs::File::create(path)?;
        Ok(Self {
            writer: file,
            range: Range::new(offset, size),
        })
    }
}

impl<W: Write> FPGADump<W> {
    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    pub fn dump(&mut self, fpga: &mut impl Dumpable) -> Result<(), Error> {
        for Range { start, len } in self.range.pages::<PAGE_SIZE>("Dumping") {
            fpga.read_page(start, len, &mut self.writer)?;
        }
        Ok(())
    }
}
