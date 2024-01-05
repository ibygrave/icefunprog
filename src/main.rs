use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use clap::Parser;
use serialport::{FlowControl, SerialPort, SerialPortBuilder, SerialPortType};

/// Programming tool for Devantech iceFUN board.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use the specified USB device
    #[arg(short, long)]
    port: Option<String>,

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

fn find_port(args: &Args) -> Result<SerialPortBuilder> {
    if let Some(port) = &args.port {
        Ok(serialport::new(port, 9600))
    } else {
        for port_info in serialport::available_ports()? {
            if let SerialPortType::UsbPort(usb_port_info) = port_info.port_type {
                if usb_port_info.vid == 0x04d8 && usb_port_info.pid == 0xffee {
                    return Ok(serialport::new(port_info.port_name, 9600));
                }
            }
        }
        anyhow::bail!("No port")
    }
}

fn open_port(args: &Args) -> Result<Box<dyn SerialPort>> {
    let mut port = find_port(args)?.open()?;
    port.set_flow_control(FlowControl::None)?;
    port.set_timeout(Duration::from_secs(10))?;
    Ok(port)
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::builder().filter_level(args.log_level).init();

    let port = open_port(&args)?;
    let fpga_data = icefun::FPGAData::<Box<dyn SerialPort>>::from_path(args.input)?;
    let mut fpga = icefun::Device { port }.prepare()?;
    fpga_data.erase(&mut fpga)?;
    fpga_data.program(&mut fpga)?;
    if !args.skip_verification {
        fpga_data.verify(&mut fpga)?;
    }

    Ok(())
}
