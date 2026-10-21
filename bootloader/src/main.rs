#![no_std]
#![no_main]

use cortex_m::asm::bootstrap;
use cortex_m_rt::entry;
use frame::FeedResult;
use panic_halt as _;
use stm32f4xx_hal::serial::config::Config as SerialConfig;
use stm32f4xx_hal::serial::{Rx, Tx};
use stm32f4xx_hal::{pac, prelude::*, rcc::Config};

use crate::flash_map::{AppSlot, RAM_END, RAM_START};
use crate::frame::FrameParser;

mod crc;
mod flash_driver;
mod flash_map;
mod frame;
mod update;

const BAUD_RATE: u32 = 115200;

const ACK: u8 = 0x06;
const NACK: u8 = 0x15;

/// Reads and returns the stack pointer and reset vector.
fn read_vectors(app: &AppSlot) -> (u32, u32) {
    let sp = unsafe { (app.base as *const u32).read_volatile() };
    let rv = unsafe { (app.base as *const u32).add(1).read_volatile() };
    (sp, rv)
}

/// Valid if stack pointer and reset vector are within expected range.
/// If app slot contains garbage, this test will prevent the stm from jumping to it.
fn validate_app(sp: u32, rv: u32, base: u32, end: u32) -> bool {
    (rv > base && rv < end) && (rv & 0b1 > 0) && (sp > RAM_START && sp <= RAM_END)
}

/// Makes the STM jump to the app.
fn jump_to_app(sp: u32, rv: u32, base: u32) -> ! {
    // Disable interrupts and counter. Then, write the application's vector table to the VTOR.
    unsafe {
        let mut cp = cortex_m::Peripherals::steal();
        cp.SYST.disable_counter();
        cp.SYST.disable_interrupt();
        cp.SCB.vtor.write(base);
        bootstrap(sp as *const u32, rv as *const u32);
    }
}

/// Validates app and jumps to app if valid. Only returns if app was invalid.
fn validate_and_launch_app(app: &AppSlot) {
    let (sp, rv) = read_vectors(app);
    if validate_app(sp, rv, app.base, app.end) {
        jump_to_app(sp, rv, app.base)
    }
}

/// Uart loop that waits to receive frames from host.
fn uart_rc_frames(
    tx: &mut Tx<pac::USART2, u8>,
    rx: &mut Rx<pac::USART2, u8>,
    flash: &mut pac::FLASH,
    updater: &mut update::Updater,
) -> ! {
    let mut frame_parser = FrameParser::new();
    loop {
        if let Ok(byte) = rx.read() {
            match frame_parser.feed(byte) {
                FeedResult::Frame(_) => {
                    if updater.handle(flash, frame_parser.payload()) {
                        tx.bwrite_all(&[ACK]).ok();
                    } else {
                        tx.bwrite_all(&[NACK]).ok();
                    }
                }
                FeedResult::BadLength | FeedResult::BadCrc => {
                    // Something went wrong.
                    tx.bwrite_all(&[NACK]).ok();
                }
                FeedResult::Pending => {}
            }
        }
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi().sysclk(16.MHz()));

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    let gpioc = dp.GPIOC.split(&mut rcc);
    let button = gpioc.pc13.into_pull_up_input();

    let mut delay = cp.SYST.delay(&rcc.clocks);

    // Blink LED once to indicate that the program has started.
    led.toggle();
    delay.delay_ms(1000);
    led.toggle();

    cortex_m::interrupt::disable();

    // If button is not pressed, try to launch application. Otherwise, enter update mode.
    if button.is_high() {
        // Not pressed: try to boot
        for i in 0..flash_map::SLOT_COUNT {
            if let Some(slot) = flash_map::slot(i) {
                validate_and_launch_app(&slot); // returns only if the slot is invalid
            }
        }
    }

    // No app is currently flashed on to board, open uart.
    let tx = gpioa.pa2.into_alternate();
    let rx = gpioa.pa3.into_alternate();
    let config = SerialConfig::default().baudrate(BAUD_RATE.bps());
    let serial = dp.USART2.serial::<u8>((tx, rx), config, &mut rcc).unwrap();
    let (mut tx_half, mut rx_half) = serial.split();
    led.toggle();

    let mut flash = dp.FLASH;
    let mut updater = update::Updater::new();

    uart_rc_frames(&mut tx_half, &mut rx_half, &mut flash, &mut updater);
}
