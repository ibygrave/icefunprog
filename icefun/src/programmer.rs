use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{fs, path::Path, usize};

use log::info;

use crate::dev::{Dumpable, Programmable};
use crate::err::Error;

#[derive(Copy, Clone, Debug)]
struct Block {
    start: usize,
    len: usize,
}

#[derive(Copy, Clone, Debug)]
struct BlockRange<const N: usize>(Block);

struct BlockRangeIter<const N: usize> {
    addr: usize,
    end_addr: usize,
}

impl<const N: usize> IntoIterator for BlockRange<N> {
    type Item = Block;
    type IntoIter = BlockRangeIter<N>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            addr: self.0.start,
            end_addr: self.0.start + self.0.len,
        }
    }
}

impl<const N: usize> Iterator for BlockRangeIter<N> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if self.addr >= self.end_addr {
            None
        } else {
            let start = self.addr;
            let len = min(N, self.end_addr - start);
            self.addr += N;
            Some(Block { start, len })
        }
    }
}

pub struct FPGAProg<R: Read + Seek> {
    reader: R,
    blocks: BlockRange<256>,
}

impl FPGAProg<File> {
    pub fn from_path(path: impl AsRef<Path>, offset: usize) -> Result<Self, Error> {
        let meta = fs::metadata(&path)?;
        let file = File::open(&path)?;
        Ok(Self {
            reader: file,
            blocks: BlockRange(Block {
                start: offset,
                len: meta.len() as usize,
            }),
        })
    }
}

impl<R: Read + Seek> FPGAProg<R> {
    pub fn erase(&self, fpga: &mut impl Programmable) -> Result<(), Error> {
        let start_page = (self.blocks.0.start >> 16) as u8;
        let end_page = ((self.blocks.0.start + self.blocks.0.len) >> 16) as u8;
        for page in start_page..=end_page {
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

        let progress = |addr: usize| {
            info!("{} {}% ", action_name, (100 * addr) / self.blocks.0.len);
        };

        for Block { start, len } in self.blocks {
            let part_buf = &mut buf[..len];
            self.reader.read_exact(part_buf)?;
            action(start, part_buf)?;
            if (start % 10240) == 0 {
                progress(start);
            }
        }
        progress(self.blocks.0.len);
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
    blocks: BlockRange<256>,
}

impl FPGADump<File> {
    pub fn from_path(path: impl AsRef<Path>, offset: usize, size: usize) -> Result<Self, Error> {
        let file = std::fs::File::create(path)?;
        Ok(Self {
            writer: file,
            blocks: BlockRange(Block {
                start: offset,
                len: size,
            }),
        })
    }
}

impl<W: Write> FPGADump<W> {
    pub fn dump(&mut self, fpga: &mut impl Dumpable) -> Result<(), Error> {
        for Block { start, len } in self.blocks {
            fpga.read_page(start, len, &mut self.writer)?;
        }
        Ok(())
    }
}
