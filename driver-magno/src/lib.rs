#![no_main]
#![no_std]

#[cfg(feature = "hal")]
pub mod hal;

#[cfg(feature = "raw")]
pub mod raw;

#[cfg(feature = "hal")]
pub use hal::*;

#[cfg(feature = "raw")]
pub use raw::*;

pub(crate) const MAGNO_SLAVE_ADDRESS: u8 = 0x1E;
pub(crate) const WHO_AM_I_M: u8 = 0x4F;
pub(crate) const MAGNO_CONF_A: u8 = 0x60;

pub(crate) const AUTO_INCREMENT: u8 = 0x80;
pub(crate) const MAGNO_X_L: u8 = 0x68;

pub(crate) const SENSITIVITY: i32 = 150;

#[derive(Default)]
pub struct MagnoAxis {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
