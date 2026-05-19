#![no_main]
#![no_std]

use core::ptr;
use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use nrf52833_pac::{self as _};
use panic_rtt_target as _;

const TWIM0: usize = 0x40003000;
const PSEL_SCL_OFFSET: usize = 0x508;
const PSEL_SDA_OFFSET: usize = 0x50C;
const ADDRESS: usize = 0x588;
const ENABLE: usize = 0x500;

const FREQUENCY: usize = 0x524;
const FREQUENCY_K100: u32 = 0x01980000;

const MAGNO_SLAVE_ADDRESS: u32 = 0x1E;

const GPIO0: usize = 0x50000000;
const P8_OFFSET: usize = 0x720;
const P16_OFFSET: usize = 0x740;

const RXD_PTR: usize = 0x534;
const RXD_MAXCNT: usize = 0x538;

const TXD_PTR: usize = 0x544;
const TXD_MAXCNT: usize = 0x548;

const TASKS_STARTTX: usize = 0x008;
const TASKS_STARTRX: usize = 0x000;

const EVENTS_RXSTARTED: usize = 0x14C;
const EVENTS_LASTTX: usize = 0x11C;

pub struct MagnoSensor {}

impl MagnoSensor {
    pub fn new() -> Self {
        let scl_pin = 8;
        let sda_pin = 16;
        let port = 0;
        let connect = 0;

        let scl_value = (connect << 31) | (port << 5) | scl_pin;
        let sda_value = (connect << 31) | (port << 5) | sda_pin;
        let pin_cnf_value = (3 << 2) | (6 << 8);

        let psel_scl = (TWIM0 + PSEL_SCL_OFFSET) as *mut u32;
        let psel_sda = (TWIM0 + PSEL_SDA_OFFSET) as *mut u32;

        let frequency = (TWIM0 + FREQUENCY) as *mut u32;
        let address = (TWIM0 + ADDRESS) as *mut u32;
        let enable = (TWIM0 + ENABLE) as *mut u32;
        let pin8 = (GPIO0 + P8_OFFSET) as *mut u32;
        let pin16 = (GPIO0 + P16_OFFSET) as *mut u32;

        unsafe {
            ptr::write_volatile(pin8, pin_cnf_value);
            ptr::write_volatile(pin16, pin_cnf_value);

            ptr::write_volatile(psel_scl, scl_value);
            ptr::write_volatile(psel_sda, sda_value);

            ptr::write_volatile(frequency, FREQUENCY_K100);

            ptr::write_volatile(address, MAGNO_SLAVE_ADDRESS);

            ptr::write_volatile(enable, 6);
        }

        Self {}
    }
}

#[entry]
fn main() -> ! {
    loop {
        wfi()
    }
}
