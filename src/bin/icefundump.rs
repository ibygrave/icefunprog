use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use icefunprog::{parse_addr, CommonArgs, Device, FPGADump};

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    common: CommonArgs,

    /// Read size
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    size: usize,

    /// Output file
    #[arg(value_name = "INPUT")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    args.common.init_logger();

    let port = args.common.open_port()?;
    let mut fpga = Device { port }.prepare()?;
    let mut dumper = FPGADump::from_path(args.output, args.common.offset, args.size)?;
    dumper.dump(&mut fpga)?;

    Ok(())
}
