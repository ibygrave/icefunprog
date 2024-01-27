use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[path = "../utils.rs"]
mod utils;

use crate::utils::{open_port, parse_addr};

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use the specified USB device
    #[arg(short, long)]
    port: Option<String>,

    /// Read offset
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    offset: usize,

    /// Read size
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    size: usize,

    /// Output file
    #[arg(value_name = "INPUT")]
    output: PathBuf,

    /// Logging level. `Off` for silent operation.
    #[arg(short, long, default_value = "Info")]
    log_level: log::LevelFilter,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::builder().filter_level(args.log_level).init();

    let port = open_port(&args.port)?;
    let mut fpga = icefun::Device { port }.prepare()?;
    let mut dumper = icefun::FPGADump::from_path(args.output)?;
    dumper.dump(&mut fpga, args.offset, args.size)?;

    Ok(())
}
