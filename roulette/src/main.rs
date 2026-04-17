#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use microbit::hal::{gpio, timer};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = microbit::Board::take().unwrap();

    let row1 = board
        .display_pins
        .row1
        .into_push_pull_output(gpio::Level::Low);

    let row2 = board
        .display_pins
        .row2
        .into_push_pull_output(gpio::Level::Low);

    let row3 = board
        .display_pins
        .row3
        .into_push_pull_output(gpio::Level::Low);

    let row4 = board
        .display_pins
        .row4
        .into_push_pull_output(gpio::Level::Low);

    let row5 = board
        .display_pins
        .row5
        .into_push_pull_output(gpio::Level::Low);

    let col1 = board
        .display_pins
        .col1
        .into_push_pull_output(gpio::Level::Low);

    let col2 = board
        .display_pins
        .col2
        .into_push_pull_output(gpio::Level::Low);

    let col3 = board
        .display_pins
        .col3
        .into_push_pull_output(gpio::Level::Low);

    let col4 = board
        .display_pins
        .col4
        .into_push_pull_output(gpio::Level::Low);

    let col5 = board
        .display_pins
        .col5
        .into_push_pull_output(gpio::Level::Low);

    let mut timer0 = timer::Timer::new(board.TIMER0);

    let mut rows = [
        row1.degrade(),
        row2.degrade(),
        row3.degrade(),
        row4.degrade(),
        row5.degrade(),
    ];

    let mut cols = [
        col1.degrade(),
        col2.degrade(),
        col3.degrade(),
        col4.degrade(),
        col5.degrade(),
    ];

    let last_row_targets = [
        (4, 4),
        (3, 4),
        (2, 4),
        (1, 4),
        (0, 4),
        (0, 3),
        (0, 2),
        (0, 1),
    ];

    loop {
        for r in 0..5 {
            for c in 0..5 {
                if r == 0 || c == 4 {
                    rows[r].set_high().unwrap();
                    cols[c].set_low().unwrap();

                    timer0.delay_ms(100);

                    rows[r].set_low().unwrap();
                    cols[c].set_high().unwrap();
                }

                if r == 4 {
                    for (col, row) in last_row_targets {
                        cols[col].set_low().unwrap();
                        rows[row].set_high().unwrap();

                        timer0.delay_ms(100);

                        cols[col].set_high().unwrap();
                        rows[row].set_low().unwrap();
                    }

                    break;
                }
            }
        }
    }
}
