#![no_main]
#![no_std]

use core::ptr;
use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use nrf52833_pac::{self as _};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

const TWIM0: usize = 0x40003000;

const PSEL_SCL: *mut u32 = (0x508 + TWIM0) as *mut u32;
const PSEL_SDA: *mut u32 = (0x50C + TWIM0) as *mut u32;

const ADDRESS: *mut u32 = (0x588 + TWIM0) as *mut u32;
const ENABLE: *mut u32 = (0x500 + TWIM0) as *mut u32;

const FREQUENCY: *mut u32 = (0x524 + TWIM0) as *mut u32;
const FREQUENCY_K100: u32 = 0x01980000;

const MAGNO_SLAVE_ADDRESS: u32 = 0x1E;

const GPIO0: usize = 0x50000000;
const P8: *mut u32 = (0x720 + GPIO0) as *mut u32;
const P16: *mut u32 = (0x740 + GPIO0) as *mut u32;

const WHO_AM_I_M: usize = 0x4F;

const RXD_PTR: *mut u32 = (0x534 + TWIM0) as *mut u32;
const TXD_PTR: *mut u32 = (0x544 + TWIM0) as *mut u32;

const RXD_MAXCNT: *mut u32 = (0x538 + TWIM0) as *mut u32;
const TXD_MAXCNT: *mut u32 = (0x548 + TWIM0) as *mut u32;

const TASKS_STARTTX: *mut u32 = (0x008 + TWIM0) as *mut u32;
const TASKS_STARTRX: *mut u32 = (0x000 + TWIM0) as *mut u32;

const EVENTS_LASTTX: *mut u32 = (0x11C + TWIM0) as *mut u32;
const EVENTS_LASTRX: *mut u32 = (0x15C + TWIM0) as *mut u32;

const SHORTS: *mut u32 = (0x200 + TWIM0) as *mut u32;

const EVENTS_ERROR: *mut u32 = (0x124 + TWIM0) as *mut u32;
const ERROR_SRC: *mut u32 = (0x4C4 + TWIM0) as *mut u32;

const EVENTS_STOPPED: *mut u32 = (0x104 + TWIM0) as *mut u32;

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

        unsafe {
            ptr::write_volatile(EVENTS_LASTTX, 0);
            ptr::write_volatile(EVENTS_LASTRX, 0);
            ptr::write_volatile(EVENTS_STOPPED, 0);

            ptr::write_volatile(P8, pin_cnf_value);
            ptr::write_volatile(P16, pin_cnf_value);

            ptr::write_volatile(PSEL_SCL, scl_value);
            ptr::write_volatile(PSEL_SDA, sda_value);

            ptr::write_volatile(FREQUENCY, FREQUENCY_K100);

            ptr::write_volatile(ADDRESS, MAGNO_SLAVE_ADDRESS);

            ptr::write_volatile(ENABLE, 6);
        }

        let sensor = Self {};

        sensor.clear_events();
        let value = sensor.verify_who_am_i();

        match value {
            Ok(value) => {
                if value != 64 {
                    panic!("Wrong WHO_AM_I value: 0x{:X}", value);
                }
            }
            Err(error) => {
                panic!("Error: 0x{:X}", error);
            }
        }

        sensor
    }

    fn verify_who_am_i(&self) -> Result<u8, u32> {
        let shorts_value: u32 = (1 << 7) | (1 << 12);

        let tx_buf = [WHO_AM_I_M];
        let mut rx_buf = [0u8; 1];

        unsafe {
            ptr::write_volatile(TXD_PTR, tx_buf.as_ptr() as u32);
            ptr::write_volatile(RXD_PTR, rx_buf.as_mut_ptr() as u32);

            ptr::write_volatile(TXD_MAXCNT, 1);
            ptr::write_volatile(RXD_MAXCNT, 1);

            ptr::write_volatile(SHORTS, shorts_value);
            ptr::write_volatile(TASKS_STARTTX, 1);
        }
        loop {
            let current_value = unsafe { ptr::read_volatile(EVENTS_STOPPED) };

            if current_value == 1 {
                break;
            }

            if let Some(error) = self.check_errors() {
                return Err(error);
            };
        }

        Ok(rx_buf[0])
    }

    fn check_errors(&self) -> Option<u32> {
        unsafe {
            let tx_error = ptr::read_volatile(EVENTS_ERROR);

            if tx_error == 1 {
                ptr::write_volatile(EVENTS_ERROR, 0);

                let error_src_val = ptr::read_volatile(ERROR_SRC);

                if error_src_val != 0 {
                    ptr::write_volatile(ERROR_SRC, 0);
                }

                return Some(error_src_val);
            }
        }

        None
    }

    fn clear_events(&self) {
        unsafe {
            ptr::write_volatile(EVENTS_LASTTX, 0);
            ptr::write_volatile(EVENTS_LASTRX, 0);
            ptr::write_volatile(EVENTS_STOPPED, 0);

            ptr::write_volatile(EVENTS_ERROR, 0);
            ptr::write_volatile(ERROR_SRC, 0);
        }
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let _ = MagnoSensor::new();
    rprintln!("Magno sensor initialized");
    loop {
        wfi()
    }
}
