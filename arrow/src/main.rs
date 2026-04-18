#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::display::blocking::Display;
use microbit::hal::timer;
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

const LEFT_ARROW: [[u8; 5]; 5] = [
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
];

const RIGHT_ARROW: [[u8; 5]; 5] = [
    [0, 0, 1, 0, 0],
    [0, 0, 0, 1, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
];

const MID_LED: [[u8; 5]; 5] = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const BLINK: [[u8; 5]; 5] = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const TICK: u16 = 25;
const BLINK_TICK: u16 = 50;

#[derive(Clone, Copy)]
enum LightState {
    Middle,
    Left(Light),
    Right(Light),
}

#[derive(Clone, Copy)]
enum Light {
    Lit(u16),
    Blinking(u16),
}

impl Light {
    fn tick(&mut self) {
        match self {
            Light::Lit(tick) => {
                if *tick == 0 {
                    *self = Light::Blinking(BLINK_TICK);
                } else {
                    *tick -= 1;
                }
            }
            Light::Blinking(tick) => {
                if *tick == 0 {
                    *self = Light::Lit(TICK);
                } else {
                    *tick -= 1;
                }
            }
        }
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = microbit::Board::take().unwrap();
    let mut timer = timer::Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut light_state = LightState::Middle;

    loop {
        let left_pressed = button_a.is_low().unwrap();
        let right_pressed = button_b.is_low().unwrap();

        match light_state {
            LightState::Middle => {
                display.show(&mut timer, MID_LED, 10);
            }

            LightState::Left(ref mut state) => {
                state.tick();
                match state {
                    Light::Lit(_) => {
                        display.show(&mut timer, LEFT_ARROW, 10);
                    }
                    Light::Blinking(_) => {
                        display.show(&mut timer, BLINK, 10);
                    }
                }
            }

            LightState::Right(ref mut state) => {
                state.tick();
                match state {
                    Light::Lit(_) => {
                        display.show(&mut timer, RIGHT_ARROW, 10);
                    }
                    Light::Blinking(_) => {
                        display.show(&mut timer, BLINK, 10);
                    }
                }
            }
        }

        match (left_pressed, right_pressed) {
            (true, false) => match light_state {
                LightState::Left(_) => {}
                _ => {
                    light_state = LightState::Left(Light::Lit(TICK));
                }
            },
            (false, true) => match light_state {
                LightState::Right(_) => {}
                _ => {
                    light_state = LightState::Right(Light::Lit(TICK));
                }
            },
            (false, false) | (true, true) => {
                light_state = LightState::Middle;
            }
        }
    }
}
