use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod utils;

use crate::utils::{open_port, parse_addr};

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use the specified USB device
    #[arg(short, long)]
    port: Option<String>,

    /// EEPROM offset (NYI)
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    offset: usize,

    /// Skip verification
    #[arg(short = 'v', long)]
    skip_verification: bool,

    /// Input file to program
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Logging level. `Off` for silent operation.
    #[arg(short, long, default_value = "Info")]
    log_level: log::LevelFilter,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::builder().filter_level(args.log_level).init();

    let port = open_port(&args.port)?;
    let mut programmer = icefun::FPGAProg::from_path(args.input)?;
    let mut fpga = icefun::Device { port }.prepare()?;
    programmer.erase(&mut fpga)?;
    programmer.program(&mut fpga)?;
    if !args.skip_verification {
        programmer.verify(&mut fpga)?;
    }

    Ok(())
}
