#![no_main]
#![no_std]

use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use heapless::index_set::FnvIndexSet;
use heapless::spsc::Queue;
use microbit::Board;
use microbit::display::nonblocking::{Display, GreyscaleImage};
use microbit::hal::gpiote::Gpiote;
use microbit::hal::pac::{self, interrupt};
use microbit::hal::rtc::RtcInterrupt;
use microbit::hal::{Clocks, Rng, Rtc, timer};
use microbit::pac::{RTC0, TIMER1};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

static SNAKE: LockMut<Snake> = LockMut::new();
static DISPLAY: LockMut<Display<TIMER1>> = LockMut::new();
static ANIM_TIMER: LockMut<Rtc<RTC0>> = LockMut::new();
static GPIO: LockMut<Gpiote> = LockMut::new();
static PENDING_MOVE: LockMut<Option<TurnDirection>> = LockMut::new();
static FOOD_LOCATION: LockMut<Position> = LockMut::new();

type Timer = timer::Timer<pac::TIMER0>;

pub struct Prng {
    value: u32,
}

impl Prng {
    pub fn seeded(rng: &mut Rng) -> Self {
        Self::new(rng.random_u32())
    }

    pub fn new(seed: u32) -> Self {
        Self { value: seed }
    }

    fn xorshift32(mut input: u32) -> u32 {
        input ^= input << 13;
        input ^= input >> 17;
        input ^= input << 5;
        input
    }

    pub fn random_u32(&mut self) -> u32 {
        self.value = Self::xorshift32(self.value);
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct Position {
    row: i8,
    col: i8,
}

#[derive(Clone, Copy, Debug)]
enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

impl MoveDirection {
    fn turn(&self, turn_direction: &TurnDirection) -> MoveDirection {
        match turn_direction {
            TurnDirection::Left => match self {
                MoveDirection::Up => MoveDirection::Left,
                MoveDirection::Left => MoveDirection::Down,
                MoveDirection::Down => MoveDirection::Right,
                MoveDirection::Right => MoveDirection::Up,
            },
            TurnDirection::Right => match self {
                MoveDirection::Up => MoveDirection::Right,
                MoveDirection::Right => MoveDirection::Down,
                MoveDirection::Down => MoveDirection::Left,
                MoveDirection::Left => MoveDirection::Up,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TurnDirection {
    Left,
    Right,
}

struct Snake {
    head: Position,
    tail: Queue<Position, 32>,
    all_positions: FnvIndexSet<Position, 32>,
    timer: Timer,
    direction: MoveDirection,
    prng: Prng,
}

impl Position {
    fn is_out_of_bounds(&self) -> bool {
        self.row < 0 || self.row > 4 || self.col < 0 || self.col > 4
    }

    fn wrap_bounds(&mut self) {
        if self.is_out_of_bounds() {
            let wrapped_position = Position {
                row: (self.row + 5) % 5,
                col: (self.col + 5) % 5,
            };

            self.row = wrapped_position.row;
            self.col = wrapped_position.col;
        }
    }

    pub fn random(rng: &mut Prng, exclude: &FnvIndexSet<Position, 32>) -> Self {
        let mut coords = Position {
            row: ((rng.random_u32() as usize) % 5) as i8,
            col: ((rng.random_u32() as usize) % 5) as i8,
        };

        while exclude.contains(&coords) {
            coords = Position {
                row: ((rng.random_u32() as usize) % 5) as i8,
                col: ((rng.random_u32() as usize) % 5) as i8,
            }
        }
        coords
    }
}

impl Snake {
    fn new(mut timer: Timer, rng: &mut Rng) -> Self {
        let head = Position { row: 2, col: 2 };
        let mut tail = Queue::new();
        let first_tail = Position { row: 1, col: 2 };
        tail.enqueue(first_tail).unwrap();

        timer.start(500_000);

        let direction = MoveDirection::Down;

        let prng = Prng::seeded(rng);

        let mut all_positions = FnvIndexSet::new();
        all_positions.insert(head).unwrap();
        all_positions.insert(first_tail).unwrap();

        Self {
            head,
            tail,
            timer,
            direction,
            prng,
            all_positions,
        }
    }

    fn get_snake_matrix(&self, food_brightness: u8) -> GreyscaleImage {
        let mut matrix: [[u8; 5]; 5] = [[0; 5]; 5];

        matrix[self.head.row as usize][self.head.col as usize] = 9;

        for pos in self.tail.iter() {
            matrix[pos.row as usize][pos.col as usize] = 6;
        }

        let mut food_position = Position { row: 0, col: 0 };

        FOOD_LOCATION.with_lock(|location| {
            food_position = *location;
        });

        matrix[food_position.row as usize][food_position.col as usize] = food_brightness;

        GreyscaleImage::new(&matrix)
    }

    fn snake_move_tick(&mut self) {
        self.timer.start(1_000_000);

        let mut new_direction = self.direction;

        PENDING_MOVE.with_lock(|pending_move| {
            if let Some(turn_direction) = pending_move {
                new_direction = new_direction.turn(turn_direction);
            }
            *pending_move = None;
        });

        self.direction = new_direction;

        let mut new_head_location = match self.direction {
            MoveDirection::Up => Position {
                row: self.head.row - 1,
                col: self.head.col,
            },
            MoveDirection::Down => Position {
                row: self.head.row + 1,
                col: self.head.col,
            },
            MoveDirection::Left => Position {
                row: self.head.row,
                col: self.head.col - 1,
            },
            MoveDirection::Right => Position {
                row: self.head.row,
                col: self.head.col + 1,
            },
        };

        new_head_location.wrap_bounds();

        if self.all_positions.contains(&new_head_location) {
            self.reset_game();
            return;
        }

        let old_head_location = self.head;

        self.head = new_head_location;

        self.tail.enqueue(old_head_location).unwrap();
        self.all_positions.insert(new_head_location).unwrap();

        if self.is_on_food() {
            let new_food_location = self.get_food();

            FOOD_LOCATION.with_lock(|location| {
                *location = new_food_location;
            });
        } else {
            let deleted_tail = self.tail.dequeue().unwrap();
            self.all_positions.remove(&deleted_tail);
        }
    }

    fn get_food(&mut self) -> Position {
        Position::random(&mut self.prng, &self.all_positions)
    }

    fn is_on_food(&self) -> bool {
        let mut food_location = Position { row: 0, col: 0 };

        FOOD_LOCATION.with_lock(|location| food_location = *location);

        self.head == food_location
    }

    fn reset_game(&mut self) {
        let head = Position { row: 2, col: 2 };
        let mut tail = Queue::new();
        let first_tail = Position { row: 1, col: 2 };
        tail.enqueue(first_tail).unwrap();

        let direction = MoveDirection::Down;

        let mut all_positions = FnvIndexSet::new();
        all_positions.insert(head).unwrap();
        all_positions.insert(first_tail).unwrap();

        self.head = head;
        self.tail = tail;
        self.direction = direction;
        self.all_positions = all_positions;

        let new_food_location = self.get_food();

        FOOD_LOCATION.with_lock(|location| {
            *location = new_food_location;
        });
    }
}

#[entry]
fn main() -> ! {
    if let Some(mut board) = Board::take() {
        rtt_init_print!();

        Clocks::new(board.CLOCK).start_lfclk();
        let mut rng = Rng::new(board.RNG);

        let mut timer0 = timer::Timer::new(board.TIMER0);

        timer0.enable_interrupt();
        timer0.reset_event();

        let mut snake = Snake::new(timer0, &mut rng);

        let initial_food = snake.get_food();

        SNAKE.init(snake);
        PENDING_MOVE.init(None);
        FOOD_LOCATION.init(initial_food);

        let gpiot = Gpiote::new(board.GPIOTE);

        let channel_0 = gpiot.channel0();
        let channel_1 = gpiot.channel1();

        let button_a = board.buttons.button_a.degrade();
        let button_b = board.buttons.button_b.degrade();

        channel_0.input_pin(&button_a).hi_to_lo().enable_interrupt();
        channel_1.input_pin(&button_b).hi_to_lo().enable_interrupt();

        GPIO.init(gpiot);

        let mut rtc0 = Rtc::new(board.RTC0, 2047).unwrap();
        rtc0.enable_event(RtcInterrupt::Tick);
        rtc0.enable_interrupt(RtcInterrupt::Tick, None);
        rtc0.enable_counter();

        let display = Display::new(board.TIMER1, board.display_pins);

        DISPLAY.init(display);
        ANIM_TIMER.init(rtc0);

        unsafe {
            board.NVIC.set_priority(pac::Interrupt::RTC0, 64);
            board.NVIC.set_priority(pac::Interrupt::TIMER1, 128);
            pac::NVIC::unmask(pac::Interrupt::RTC0);
            pac::NVIC::unmask(pac::Interrupt::TIMER1);
            pac::NVIC::unmask(pac::Interrupt::TIMER0);
            pac::NVIC::unmask(pac::Interrupt::GPIOTE);
        }
    }

    loop {
        wfi();
    }
}

#[interrupt]
fn TIMER1() {
    DISPLAY.with_lock(|display| {
        display.handle_display_event();
    });
}

#[interrupt]
fn TIMER0() {
    SNAKE.with_lock(|snake| {
        snake.timer.reset_event();
        snake.snake_move_tick();
    });
}

#[interrupt]
fn GPIOTE() {
    GPIO.with_lock(|gpio| {
        let button_a = gpio.channel0().is_event_triggered();
        let button_b = gpio.channel1().is_event_triggered();

        let mut new_move = None;

        match (button_a, button_b) {
            (true, false) => {
                new_move = Some(TurnDirection::Left);
            }
            (false, true) => {
                new_move = Some(TurnDirection::Right);
            }
            _ => {}
        }

        PENDING_MOVE.with_lock(|pending| {
            *pending = new_move;
        });

        gpio.channel0().reset_events();
        gpio.channel1().reset_events();
    });
}

#[interrupt]
unsafe fn RTC0() {
    static mut STEP: u8 = 0;
    ANIM_TIMER.with_lock(|rtc| {
        rtc.reset_event(RtcInterrupt::Tick);
    });

    let inner_brightness = match *STEP {
        0..=8 => 9 - *STEP,
        9..=12 => 0,
        _ => unreachable!(),
    };

    DISPLAY.with_lock(|display| {
        SNAKE.with_lock(|snake| {
            let snake_matrix = snake.get_snake_matrix(inner_brightness);

            display.show(&snake_matrix);
        });
    });

    *STEP += 1;
    if *STEP == 13 {
        *STEP = 0
    };
}
