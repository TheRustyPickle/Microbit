#![no_main]
#![no_std]

use core::fmt::Write;
use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, MagMode, MagOutputDataRate};
use microbit::hal::uarte;
use microbit::hal::uarte::{Baudrate, Parity};
use microbit::hal::{Timer, twim};
use microbit::pac::twim0::frequency::FREQUENCY_A;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use serial_setup::UartePort;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = microbit::Board::take().unwrap();

    let mut serial = {
        let serial = uarte::Uarte::new(
            board.UARTE0,
            board.uart.into(),
            Parity::EXCLUDED,
            Baudrate::BAUD115200,
        );
        UartePort::new(serial)
    };

    let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
    let mut timer = Timer::new(board.TIMER0);

    write!(serial, "I'm waiting for input").unwrap();
    write!(serial, "\r\n").unwrap();

    serial.flush().unwrap();

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor
        .set_accel_mode_and_odr(
            &mut timer,
            AccelMode::HighResolution,
            AccelOutputDataRate::Hz50,
        )
        .unwrap();

    sensor
        .set_mag_mode_and_odr(&mut timer, MagMode::LowPower, MagOutputDataRate::Hz100)
        .unwrap();

    let mut buffer = [0u8; 16];
    let mut count = 0;

    loop {
        let byte = serial.read().unwrap();

        let char = char::from(byte);

        if char != '\r' {
            buffer[count] = byte;
            count += 1;
        }

        if char == '\r' || count == buffer.len() {
            let Ok(command) = core::str::from_utf8(&buffer[..count]) else {
                write!(serial, "I don't know that command").unwrap();
                write!(serial, "\r\n").unwrap();
                continue;
            };

            match command {
                "mag" => loop {
                    if let Ok(field) = sensor.magnetic_field() {
                        let (x, y, z) = field.xyz_nt();
                        write!(serial, "Magnetic field: x {} y {} z {}", x, y, z).unwrap();
                        serial.flush().unwrap();
                        break;
                    } else {
                        timer.delay_ms(1u32);
                    }
                },
                "acc" => {
                    while !sensor.accel_status().unwrap().xyz_new_data() {
                        timer.delay_ms(1u32);
                    }

                    let (x, y, z) = sensor.acceleration().unwrap().xyz_mg();
                    write!(serial, "Acceleration: x {} y {} z {}", x, y, z).unwrap();
                    serial.flush().unwrap();
                }
                _ => {
                    write!(serial, "I don't know that command").unwrap();
                    serial.flush().unwrap();
                }
            }
            write!(serial, "\r\n").unwrap();

            rprintln!("{}", command);
            buffer = [0u8; 16];
            count = 0;
        }
    }
}
