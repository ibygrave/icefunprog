#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    SerialPortError(serialport::Error),
    CmdError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => err.fmt(f),
            Self::SerialPortError(err) => err.fmt(f),
            Self::CmdError(msg) => write!(f, "CmdError {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<serialport::Error> for Error {
    fn from(value: serialport::Error) -> Self {
        Self::SerialPortError(value)
    }
}
