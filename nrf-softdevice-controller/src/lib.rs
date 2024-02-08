#![no_std]

// Must be a top!
mod fmt;

pub use nrf_softdevice_controller_sys as raw;

pub mod mpsl;
pub mod sdc;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    InvalidArg,
    NotPermitted,
    Other,
}

impl From<i32> for Error {
    fn from(val: i32) -> Self {
        match val {
            -1 => Self::NotPermitted,
            -22 => Self::InvalidArg,
            _ => Self::Other,
        }
    }
}
