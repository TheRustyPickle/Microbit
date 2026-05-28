#![no_main]
#![no_std]

use core::ptr;
use cortex_m::asm::wfi;
use cortex_m_rt::{self, entry};
use critical_section_lock_mut::LockMut;
use microbit::hal::pac::interrupt;
use nrf52833_pac::{self as _};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

static SENSOR: LockMut<MagnoSensor> = LockMut::new();

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
const P25: *mut u32 = (0x764 + GPIO0) as *mut u32;

const WHO_AM_I_M: u8 = 0x4F;

const RXD_PTR: *mut u32 = (0x534 + TWIM0) as *mut u32;
const TXD_PTR: *mut u32 = (0x544 + TWIM0) as *mut u32;

const RXD_MAXCNT: *mut u32 = (0x538 + TWIM0) as *mut u32;
const TXD_MAXCNT: *mut u32 = (0x548 + TWIM0) as *mut u32;

const TASKS_STARTTX: *mut u32 = (0x008 + TWIM0) as *mut u32;

const EVENTS_LASTTX: *mut u32 = (0x11C + TWIM0) as *mut u32;
const EVENTS_LASTRX: *mut u32 = (0x15C + TWIM0) as *mut u32;

const SHORTS: *mut u32 = (0x200 + TWIM0) as *mut u32;

const EVENTS_ERROR: *mut u32 = (0x124 + TWIM0) as *mut u32;
const ERROR_SRC: *mut u32 = (0x4C4 + TWIM0) as *mut u32;

const EVENTS_STOPPED: *mut u32 = (0x104 + TWIM0) as *mut u32;

const MAGNO_CONF_A: u8 = 0x60;
const MAGNO_CONF_C: u8 = 0x63;

const AUTO_INCREMENT: u8 = 0x80;
const MAGNO_X_L: u8 = 0x68;

const SENSITIVITY: i16 = 150;

const GPIOTE_BASE: usize = 0x40006000;
const GPIOTE_CONFIG0: *mut u32 = (0x510 + GPIOTE_BASE) as *mut u32;
const GPIOTE_INTERRUPT: *mut u32 = (0x304 + GPIOTE_BASE) as *mut u32;

const GPIOTE_IN0: *mut u32 = (0x100 + GPIOTE_BASE) as *mut u32;

const LATCH: *mut u32 = (0x520 + TWIM0) as *mut u32;

pub struct MagnoAxis {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

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

        if let Err(e) = sensor.set_default_magno() {
            panic!("Error: 0x{:X}", e);
        };

        sensor
    }

    fn verify_who_am_i(&self) -> Result<u8, u32> {
        let mut value = [0; 1];
        self.twim_read_write(WHO_AM_I_M, None, &mut value)?;
        Ok(value[0])
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

    pub fn get_magno_value_blocking(&self) -> Result<MagnoAxis, u32> {
        loop {
            let mut status = [0; 1];
            self.twim_read_write(0x67, None, &mut status)?;

            if (status[0] & 0x08) != 0 {
                break;
            }
        }

        self.get_magno_value_no_check()
    }

    pub fn get_magno_value_no_check(&self) -> Result<MagnoAxis, u32> {
        // Auto read 6 registers at once
        let mut value_buf = [0; 6];
        self.twim_read_write(MAGNO_X_L | AUTO_INCREMENT, None, &mut value_buf)?;

        let x_value = i16::from_le_bytes([value_buf[0], value_buf[1]]) * SENSITIVITY;
        let y_value = i16::from_le_bytes([value_buf[2], value_buf[3]]) * SENSITIVITY;
        let z_value = i16::from_le_bytes([value_buf[4], value_buf[5]]) * SENSITIVITY;

        rprintln!(
            "Magno values: X: {}, Y: {}, Z: {}",
            x_value,
            y_value,
            z_value
        );

        let axis = MagnoAxis {
            x: x_value,
            y: y_value,
            z: z_value,
        };

        Ok(axis)
    }

    fn set_default_magno(&self) -> Result<(), u32> {
        let low_power = 0;

        // 100 hz
        let odr0 = 1;
        let odr1 = 1;

        // Continuous
        let md_value = 0;

        let magno_value = md_value | (md_value << 1) | (odr0 << 2) | (odr1 << 3) | (low_power << 4);

        self.twim_read_write(MAGNO_CONF_A, Some(magno_value), &mut [0; 0])
            .map(|_| ())
    }

    fn twim_read_write(
        &self,
        address: u8,
        write_value: Option<u8>,
        rx_buf: &mut [u8],
    ) -> Result<(), u32> {
        self.clear_events();

        let mut tx_buf = [0u8; 2];
        tx_buf[0] = address;

        let tx_len: u32;
        let rx_len = rx_buf.len() as u32;

        let mut shorts_value = (1 << 7) | (1 << 12);

        if let Some(val) = write_value {
            tx_buf[1] = val;
            tx_len = 2;
            shorts_value = 1 << 9;
        } else {
            tx_len = 1;
        }

        unsafe {
            ptr::write_volatile(TXD_PTR, tx_buf.as_ptr() as u32);
            ptr::write_volatile(RXD_PTR, rx_buf.as_mut_ptr() as u32);

            ptr::write_volatile(TXD_MAXCNT, tx_len);
            ptr::write_volatile(RXD_MAXCNT, rx_len);

            ptr::write_volatile(SHORTS, shorts_value);
            ptr::write_volatile(TASKS_STARTTX, 1);
        }

        self.loop_until_stop()?;

        Ok(())
    }

    fn clear_events(&self) {
        unsafe {
            ptr::write_volatile(EVENTS_LASTTX, 0);
            ptr::write_volatile(EVENTS_LASTRX, 0);
            ptr::write_volatile(EVENTS_STOPPED, 0);

            ptr::write_volatile(EVENTS_ERROR, 0);
            ptr::write_volatile(ERROR_SRC, 0);

            ptr::write_volatile(GPIOTE_IN0, 0);
        }
    }

    fn loop_until_stop(&self) -> Result<(), u32> {
        loop {
            let current_value = unsafe { ptr::read_volatile(EVENTS_STOPPED) };

            if current_value == 1 {
                break;
            }

            if let Some(error) = self.check_errors() {
                return Err(error);
            };
        }

        Ok(())
    }

    pub fn enable_interrupt_continuous(&self) -> Result<(), u32> {
        // 0 = INT_MAG = DRDY
        // 4 = BDU = Corruption block
        let value = (1 << 0) | (1 << 4);

        self.twim_read_write(MAGNO_CONF_C, Some(value), &mut [0; 0])?;

        // 0 = input pin
        // 0 = input connect
        // 3 = pull up
        // 3 = Sense low level
        let value = (3 << 2) | (3 << 16);

        unsafe {
            ptr::write_volatile(P25, value);
        }

        // 1 = event mode
        // 25 = pin 25
        // 0 = port 0
        // 1 = Low to high
        let value = (1 << 0) | (25 << 8) | (1 << 16);

        unsafe {
            ptr::write_volatile(GPIOTE_CONFIG0, value);
        }

        // 0 = interrupt on IN0
        // 1 = Enable event PORT interrupt
        let value = (1 << 0) | (1 << 31);

        // Enable interrupt on IN0
        unsafe {
            ptr::write_volatile(GPIOTE_INTERRUPT, value);
        }

        Ok(())
    }

    pub fn acknowledge_interrupt(&self) {
        unsafe {
            ptr::write_volatile(GPIOTE_IN0, 0);

            let value = 1 << 25;

            ptr::write_volatile(LATCH, value);
        }
    }
}

#[interrupt]
fn GPIOTE() {
    SENSOR.with_lock(|s| {
        let values = s.get_magno_value_no_check().unwrap();
        rprintln!(
            "Magno values: X: {}, Y: {}, Z: {}",
            values.x,
            values.y,
            values.z
        );

        s.acknowledge_interrupt();
    });
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let sensor = MagnoSensor::new();

    rprintln!("Magno sensor initialized");
    let _ = sensor.enable_interrupt_continuous();
    let _ = sensor.get_magno_value_blocking();

    SENSOR.init(sensor);

    unsafe {
        cortex_m::peripheral::NVIC::unmask(nrf52833_pac::Interrupt::GPIOTE);
    }

    loop {
        wfi();
    }
}
