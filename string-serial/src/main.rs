#![no_main]
#![no_std]

use core::fmt::Write;
use cortex_m_rt::entry;
use microbit::hal::uarte;
use microbit::hal::uarte::{Baudrate, Parity};
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

    write!(serial, "I'm waiting for input").unwrap();
    serial.flush().unwrap();

    let mut buffer = [0u8; 64];
    let mut count = 0;

    loop {
        let byte = serial.read().unwrap();

        let char = char::from(byte);

        if char != '\r' {
            buffer[count] = byte;
            count += 1;
        }

        if char == '\r' || count == buffer.len() {
            let to_print = core::str::from_utf8(&buffer[..count]).unwrap();

            for c in to_print.chars().rev() {
                write!(serial, "{}", c).unwrap();
            }
            write!(serial, "\r\n").unwrap();

            rprintln!("{:?}", to_print);
            buffer = [0u8; 64];
            count = 0;
        }
    }
}
