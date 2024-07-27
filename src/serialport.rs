use std::io::{Read, Write};

pub trait SerialPort: Read + Write {}
