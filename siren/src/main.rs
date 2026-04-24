#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use embedded_hal::delay::DelayNs as _;
use embedded_hal::digital::{InputPin as _, OutputPin};
use microbit::Board;
use microbit::hal::gpio::{Level, Output, Pin, PushPull};
use microbit::hal::pac::{self, interrupt};
use microbit::hal::timer;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

static SIREN: LockMut<Siren> = LockMut::new();

type Timer = timer::Timer<pac::TIMER0>;
type Speaker = Pin<Output<PushPull>>;

const BASE_FREQ: u32 = 440;
const MAX_RISE: u32 = 220;
const MAX_FREQ_CYCLE: u32 = 500_000;
const FREQ_TIME: u32 = MAX_FREQ_CYCLE / MAX_RISE;

struct Siren {
    timer: Timer,
    speaker: Speaker,
    is_high: bool,
    current_freq: u32,
    going_up: bool,
    accumulated_time: u32,
}

impl Siren {
    fn new(timer: Timer, speaker: Speaker) -> Self {
        Self {
            timer,
            speaker,
            is_high: false,
            current_freq: BASE_FREQ,
            going_up: true,
            accumulated_time: 0,
        }
    }

    fn start(&mut self) {
        self.timer.enable_interrupt();
        self.tick();
    }

    fn stop(&mut self) {
        self.timer.disable_interrupt();
    }

    fn tick(&mut self) {
        if self.is_high {
            self.speaker.set_low().unwrap();
            self.is_high = false;
        } else {
            self.speaker.set_high().unwrap();
            self.is_high = true;
        }

        let half_period = MAX_FREQ_CYCLE / self.current_freq;

        self.accumulated_time += half_period;

        if self.accumulated_time >= FREQ_TIME {
            self.accumulated_time -= FREQ_TIME;

            if self.going_up {
                self.current_freq += 1;
                if self.current_freq >= 660 {
                    self.going_up = false;
                }
            } else {
                self.current_freq -= 1;
                if self.current_freq <= 440 {
                    self.going_up = true;
                }
            }
        }

        self.timer.start(half_period);
    }
}

#[interrupt]
fn TIMER0() {
    SIREN.with_lock(|siren| {
        siren.timer.reset_event();
        siren.tick();
    });
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let speaker = board
        .speaker_pin
        .into_push_pull_output(Level::Low)
        .degrade();

    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let timer0 = timer::Timer::new(board.TIMER0);
    let mut timer1 = timer::Timer::new(board.TIMER1);

    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER0) };
    pac::NVIC::unpend(pac::Interrupt::TIMER0);

    let siren = Siren::new(timer0, speaker);
    SIREN.init(siren);

    SIREN.with_lock(|siren| {
        siren.start();
    });

    for t in (1..=10).rev() {
        rprintln!("{}", t);
        timer1.delay_ms(1_000);

        if button_a.is_low().unwrap() || button_b.is_low().unwrap() {
            SIREN.with_lock(|siren| {
                siren.stop();
            });
            break;
        }
    }

    SIREN.with_lock(|siren| {
        siren.stop();
    });

    rprintln!("Launch");

    loop {
        asm::wfi();
    }
}
