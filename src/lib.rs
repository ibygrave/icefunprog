#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod cmds;
mod dev;
mod err;
mod programmer;
mod serialport;
mod test_mocks;
mod utils;

pub use dev::Device;
pub use programmer::{FPGADump, FPGAProg};
pub use utils::{parse_addr, CommonArgs};
