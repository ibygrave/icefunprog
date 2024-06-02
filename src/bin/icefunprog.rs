use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use icefunprog::{CommonArgs, Device, FPGAProg};

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    common: CommonArgs,

    /// Skip verification
    #[arg(short = 'v', long)]
    skip_verification: bool,

    /// Input file to program
    #[arg(value_name = "INPUT")]
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    args.common.init_logger();

    let port = args.common.open_port()?;
    let mut programmer = FPGAProg::from_path(args.input, args.common.offset)?;
    let mut fpga = Device { port }.prepare()?;
    programmer.erase(&mut fpga)?;
    programmer.program(&mut fpga)?;
    if !args.skip_verification {
        programmer.verify(&mut fpga)?;
    }

    Ok(())
}
