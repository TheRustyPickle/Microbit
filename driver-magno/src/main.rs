#![no_main]
#![no_std]

use core::ptr;
use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use nrf52833_pac::{self as _};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

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

const WHO_AM_I_M: usize = 0x4F;

const RXD_PTR: usize = 0x534;
const RXD_MAXCNT: usize = 0x538;

const TXD_PTR: usize = 0x544;
const TXD_MAXCNT: usize = 0x548;

const TASKS_STARTTX: usize = 0x008;
const TASKS_STARTRX: usize = 0x000;

const EVENTS_LASTTX: usize = 0x11C;
const EVENTS_LASTRX: usize = 0x15C;

const EVENTS_ERROR: usize = 0x124;

const SHORTS_OFFSET: usize = 0x200;
const ERROR_SRC: usize = 0x4C4;

#[derive(Default)]
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
        let shorts_value: u32 = (1 << 7) | (1 << 12);

        let psel_scl = (TWIM0 + PSEL_SCL_OFFSET) as *mut u32;
        let psel_sda = (TWIM0 + PSEL_SDA_OFFSET) as *mut u32;

        let frequency = (TWIM0 + FREQUENCY) as *mut u32;
        let address = (TWIM0 + ADDRESS) as *mut u32;
        let enable = (TWIM0 + ENABLE) as *mut u32;
        let pin8 = (GPIO0 + P8_OFFSET) as *mut u32;
        let pin16 = (GPIO0 + P16_OFFSET) as *mut u32;

        let txd_ptr = (TWIM0 + TXD_PTR) as *mut u32;
        let rxd_ptr = (TWIM0 + RXD_PTR) as *mut u32;

        let txd_maxcnt = (TWIM0 + TXD_MAXCNT) as *mut u32;
        let rxd_maxcnt = (TWIM0 + RXD_MAXCNT) as *mut u32;

        let tx_buf = [WHO_AM_I_M];

        let tasks_starttx = (TWIM0 + TASKS_STARTTX) as *mut u32;
        let tasks_startrx = (TWIM0 + TASKS_STARTRX) as *mut u32;

        let events_lasttx = (TWIM0 + EVENTS_LASTTX) as *mut u32;
        let events_error = (TWIM0 + EVENTS_ERROR) as *mut u32;
        let events_lastrx = (TWIM0 + EVENTS_LASTRX) as *mut u32;

        let error_src = (TWIM0 + ERROR_SRC) as *mut u32;
        let shorts = (TWIM0 + SHORTS_OFFSET) as *mut u32;

        let mut rx_buf = [0u8; 1];

        unsafe {
            ptr::write_volatile(shorts, shorts_value);
            ptr::write_volatile(pin8, pin_cnf_value);
            ptr::write_volatile(pin16, pin_cnf_value);

            ptr::write_volatile(psel_scl, scl_value);
            ptr::write_volatile(psel_sda, sda_value);

            ptr::write_volatile(frequency, FREQUENCY_K100);

            ptr::write_volatile(address, MAGNO_SLAVE_ADDRESS);

            ptr::write_volatile(enable, 6);

            ptr::write_volatile(txd_ptr, tx_buf.as_ptr() as u32);

            ptr::write_volatile(txd_maxcnt, 1);

            ptr::write_volatile(tasks_starttx, 1);

            loop {
                let current_value = ptr::read_volatile(events_lasttx);

                if current_value == 1 {
                    ptr::write_volatile(events_lasttx, 0);
                    ptr::write_volatile(rxd_maxcnt, 1);
                    ptr::write_volatile(rxd_ptr, rx_buf.as_mut_ptr() as u32);
                    ptr::write_volatile(tasks_startrx, 1);
                    break;
                }

                let tx_error = ptr::read_volatile(events_error);

                if tx_error == 1 {
                    rprintln!("I2C Error Detected!");
                    ptr::write_volatile(events_error, 0);
                    break;
                }

                let error_src_val = ptr::read_volatile(error_src);

                if error_src_val == 1 {
                    rprintln!("I2C Error Source Detected!");
                    ptr::write_volatile(error_src, 0);
                    break;
                }
            }

            loop {
                let current_value = ptr::read_volatile(events_lastrx);

                if current_value == 1 {
                    ptr::write_volatile(events_lastrx, 0);
                    break;
                }

                let tx_error = ptr::read_volatile(events_error);

                if tx_error == 1 {
                    rprintln!("I2C Error Detected!");
                    ptr::write_volatile(events_error, 0);
                    break;
                }

                let error_src_val = ptr::read_volatile(error_src);

                if error_src_val == 1 {
                    rprintln!("I2C Error Source Detected!");
                    ptr::write_volatile(error_src, 0);
                    break;
                }
            }

            rprintln!("{:?}", rx_buf);
        }

        Self {}
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let _ = MagnoSensor::new();
    loop {
        wfi()
    }
}
