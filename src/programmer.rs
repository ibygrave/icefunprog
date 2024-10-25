use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};
use std::{fs, path::Path};

use tracing::{info, instrument};

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

    #[instrument]
    fn pages<'a, const N: usize>(&'a self) -> impl Iterator<Item = Range> {
        fn inner(
            page_size: usize,
            page_count: usize,
            start_addr: usize,
            end_addr: usize,
        ) -> impl Iterator<Item = Range> {
            let mut last_tick = Instant::now();
            (0..page_count).map(move |page| {
                let now = Instant::now();
                if now.duration_since(last_tick) >= REPORT_PERIOD || (1 + page) == page_count {
                    let progress = format!("{}%", (100 * (1 + page)) / page_count);
                    info!(progress);
                    last_tick = now;
                }
                let start = start_addr + (page * page_size);
                Range::new(start, min(page_size, end_addr - start))
            })
        }
        let page_count = 1 + ((self.len - 1) / N);
        inner(N, page_count, self.start, self.start + self.len)
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
    #[instrument(skip_all)]
    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        for sector in self.range.sectors()? {
            info!(sector, "Erasing");
            fpga.erase64k(sector)?;
        }
        Ok(())
    }

    fn do_pages(
        &mut self,
        mut action: impl FnMut(usize, &[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buf = [0u8; PAGE_SIZE];
        for Range { start, len } in self.range.pages::<PAGE_SIZE>() {
            let part_buf = &mut buf[..len];
            self.reader.read_exact(part_buf)?;
            action(start, part_buf)?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip_all)]
    pub fn program(&mut self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.reader.seek(SeekFrom::Start(0))?;
        self.do_pages(|addr, data| fpga.program_page(addr, data))
    }

    /// # Errors
    ///
    /// Will return `Err` if commnication fails.
    #[instrument(skip_all)]
    pub fn verify(&mut self, fpga: &mut impl Programmable) -> Result<(), Error> {
        self.reader.seek(SeekFrom::Start(0))?;
        self.do_pages(|addr, data| fpga.verify_page(addr, data))
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
    #[instrument(skip_all)]
    pub fn dump(&mut self, fpga: &mut impl Dumpable) -> Result<(), Error> {
        for Range { start, len } in self.range.pages::<PAGE_SIZE>() {
            fpga.read_page(start, len, &mut self.writer)?;
        }
        Ok(())
    }
}
