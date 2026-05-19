#![no_main]
#![no_std]

use core::ptr;
use cortex_m::asm::nop;
use nrf52833_pac::{self as _, TEMP};
use panic_rtt_target as _;

pub struct TemperatureSensorRaw {
    tasks_start: *mut u32,
    events_datardy: *mut u32,
    temp: *const i32,
}

impl TemperatureSensorRaw {
    /// # Safety
    ///
    /// Must only be stored/called once
    pub unsafe fn new() -> Self {
        Self {
            tasks_start: 0x4000_C000 as *mut u32,
            events_datardy: 0x4000_C100 as *mut u32,
            temp: 0x4000_C508 as *const i32,
        }
    }

    pub fn read_temp_blocking(&mut self) -> i32 {
        unsafe {
            ptr::write_volatile(self.tasks_start, 1);

            while ptr::read_volatile(self.events_datardy) == 0 {
                nop();
            }

            let raw_temp = ptr::read_volatile(self.temp) / 4;

            ptr::write_volatile(self.events_datardy, 0);

            raw_temp
        }
    }
}

pub struct TemperatureSensorPac {
    periph: TEMP,
}

impl TemperatureSensorPac {
    pub fn new(periph: TEMP) -> Self {
        Self { periph }
    }

    pub fn read_temp_blocking(&mut self) -> i32 {
        self.periph.tasks_start.write(|w| unsafe { w.bits(1) });

        while self.periph.events_datardy.read().bits() == 0 {
            nop();
        }

        let raw_temp = self.periph.temp.read().bits() as i32 / 4;

        self.periph.events_datardy.write(|w| unsafe { w.bits(0) });

        raw_temp
    }
}
