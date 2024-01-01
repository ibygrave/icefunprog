#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    SerialPort(serialport::Error),
    Cmd(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::SerialPort(err) => err.fmt(f),
            Self::Cmd(msg) => write!(f, "CmdError {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serialport::Error> for Error {
    fn from(value: serialport::Error) -> Self {
        Self::SerialPort(value)
    }
}
