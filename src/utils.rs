use std::time::Duration;

use anyhow::Result;
use serialport::{FlowControl, SerialPort, SerialPortBuilder, SerialPortType};

struct AddrSuffix {
    suffix: char,
    multiplier: usize,
}

const ADDR_SUFFIXES: [AddrSuffix; 2] = [
    AddrSuffix {
        suffix: 'M',
        multiplier: 1024 * 1024,
    },
    AddrSuffix {
        suffix: 'K',
        multiplier: 1024,
    },
];

pub fn parse_addr(arg: &str) -> Result<usize> {
    for addr_suffix in ADDR_SUFFIXES {
        if let Some(prefix) = arg.strip_suffix(addr_suffix.suffix) {
            return Ok(addr_suffix.multiplier * parse_int::parse::<usize>(prefix)?);
        }
    }
    Ok(parse_int::parse::<usize>(arg)?)
}

#[derive(clap::Args, Debug)]
pub struct CommonArgs {
    /// Use the specified USB device
    #[arg(short, long)]
    pub port: Option<String>,

    /// Logging level. `Off` for silent operation.
    #[arg(short, long, default_value = "Info")]
    pub log_level: log::LevelFilter,

    /// EEPROM start offset
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    pub offset: usize,
}

impl CommonArgs {
    pub fn init_logger(&self) {
        env_logger::builder().filter_level(self.log_level).init();
    }

    fn find_port(&self) -> Result<SerialPortBuilder> {
        if let Some(port) = &self.port {
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

    pub fn open_port(&self) -> Result<impl SerialPort> {
        let mut port = self.find_port()?.open_native()?;
        port.set_flow_control(FlowControl::None)?;
        port.set_timeout(Duration::from_secs(10))?;
        Ok(port)
    }
}
