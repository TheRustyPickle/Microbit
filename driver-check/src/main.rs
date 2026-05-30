#![no_main]
#![no_std]

use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use driver_magno::{MagnoAxis, MagnoSensor};
use driver_temp::TemperatureSensorPac;
use microbit::Board;
use microbit::hal::gpiote::Gpiote;
use microbit::hal::pac::{self, interrupt};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

static GPIO: LockMut<Gpiote> = LockMut::new();
static TEMP: LockMut<TemperatureSensorPac> = LockMut::new();
static MAGNO: LockMut<MagnoSensor> = LockMut::new();

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();

    let gpiot = Gpiote::new(board.GPIOTE);

    let channel_0 = gpiot.channel0();

    let button_a = board.buttons.button_a.degrade();
    channel_0.input_pin(&button_a).hi_to_lo().enable_interrupt();

    let temp_driver = TemperatureSensorPac::new(board.TEMP);
    let magno_driver = MagnoSensor::new();

    GPIO.init(gpiot);
    TEMP.init(temp_driver);
    MAGNO.init(magno_driver);

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::GPIOTE);
    }

    loop {
        wfi()
    }
}

#[interrupt]
fn GPIOTE() {
    GPIO.with_lock(|gpio| {
        let button_a = gpio.channel0().is_event_triggered();

        if button_a {
            let mut current_temp = 0;
            let mut current_magno = MagnoAxis::default();

            TEMP.with_lock(|temp| current_temp = temp.read_temp_blocking());
            MAGNO.with_lock(|magno| current_magno = magno.get_magno_value_blocking().unwrap());

            rprintln!("Current temperature: {}", current_temp);
            rprintln!(
                "Current Magno: X: {}, Y: {}, Z: {}",
                current_magno.x,
                current_magno.y,
                current_magno.z
            );
        }

        gpio.channel0().reset_events();
    });
}
