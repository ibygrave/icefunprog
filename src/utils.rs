use std::{
    io::{Read, Write},
    time::Duration,
};

use anyhow::Result;
use serialport::{FlowControl, SerialPort, SerialPortBuilder, SerialPortType};
use tracing::trace;
use tracing_subscriber::filter::LevelFilter;

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

pub struct TracePort<Port: Read + Write>(Port);

impl<Port: Read + Write> Read for TracePort<Port> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_len = self.0.read(buf)?;
        let read_data = &buf[..read_len];
        trace!(?read_data, "read");
        Ok(read_len)
    }
}

impl<Port: Read + Write> Write for TracePort<Port> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        trace!(?buf, "write");
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[derive(clap::Args, Debug)]
pub struct CommonArgs {
    /// Use the specified USB device
    #[arg(short, long)]
    pub port: Option<String>,

    /// Logging level. `Off` for silent operation.
    #[arg(short, long, default_value = "Info")]
    pub log_level: LevelFilter,

    /// EEPROM start offset
    #[arg(short, long, default_value = "0", value_parser = parse_addr)]
    pub offset: usize,
}

impl CommonArgs {
    pub fn init_logger(&self) {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(self.log_level)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting tracing default failed");
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

    pub fn open_port(&self) -> Result<impl Read + Write> {
        let mut port = self.find_port()?.open_native()?;
        port.set_flow_control(FlowControl::None)?;
        port.set_timeout(Duration::from_secs(10))?;
        Ok(TracePort(port))
    }
}
