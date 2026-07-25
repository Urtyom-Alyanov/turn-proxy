pub mod addr_target;
pub mod command;
pub mod error;
pub mod frame;

pub const MAGIC_BYTE: u8 = 0x67;
pub const HEADER_SIZE: usize = 12;
