extern crate cidr_utils;
mod armada;
pub mod utils;

pub use crate::armada::config::{host::HostIterator, port::PortIterator};
pub use crate::armada::work::ArmadaWorkMessage;
pub use crate::armada::Armada;
