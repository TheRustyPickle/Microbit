#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::display::blocking::Display;
use microbit::hal::timer;
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

const LEFT_ARROW: [(usize, usize); 9] = [
    (0, 2),
    (1, 1),
    (2, 0),
    (3, 1),
    (4, 2),
    (2, 1),
    (2, 2),
    (2, 3),
    (2, 4),
];

const RIGHT_ARROW: [(usize, usize); 9] = [
    (0, 2),
    (1, 3),
    (2, 4),
    (3, 3),
    (4, 2),
    (2, 1),
    (2, 2),
    (2, 3),
    (2, 0),
];

const MID_LED: (usize, usize) = (2, 2);

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = microbit::Board::take().unwrap();
    let mut timer = timer::Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut leds = [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
    ];

    let clean_led = leds;

    loop {
        let left_pressed = button_a.is_low().unwrap();
        let right_pressed = button_b.is_low().unwrap();

        match (left_pressed, right_pressed) {
            (true, false) => {
                leds = clean_led;
                for (x, y) in LEFT_ARROW.iter() {
                    leds[*x][*y] = 1;
                }
                display.show(&mut timer, leds, 10);
            }
            (false, true) => {
                leds = clean_led;
                for (x, y) in RIGHT_ARROW.iter() {
                    leds[*x][*y] = 1;
                }
                display.show(&mut timer, leds, 10);
            }
            (false, false) | (true, true) => {
                leds = clean_led;
                leds[MID_LED.0][MID_LED.1] = 1;
                display.show(&mut timer, leds, 10);
            }
        }
    }
}
