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
            return Ok(addr_suffix.multiplier * prefix.parse::<usize>()?);
        }
    }
    Ok(parse_int::parse(arg)?)
}

fn find_port(port: &Option<String>) -> Result<SerialPortBuilder> {
    if let Some(port) = &port {
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

pub fn open_port(port: &Option<String>) -> Result<impl SerialPort> {
    let mut port = find_port(port)?.open_native()?;
    port.set_flow_control(FlowControl::None)?;
    port.set_timeout(Duration::from_secs(10))?;
    Ok(port)
}
