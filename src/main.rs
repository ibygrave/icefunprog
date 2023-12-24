use std::path::PathBuf;

use clap::Parser;

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use the specified USB device
    #[arg(short, long, default_value = "/dev/ttyACM0")]
    port: PathBuf,

    /// Start address for writes
    #[arg(short, long, default_value_t = 0)]
    offset: usize,

    /// Input file to program
    #[arg(value_name = "INPUT")]
    input: PathBuf,
}

fn main() {
    let args = Args::parse();

    println!("port: {}", args.port.display());
    println!("offset: {}", args.offset);
    println!("input file: {}", args.input.display());
}
