#![no_std]
#![no_main]

use cortex_m::asm::bootstrap;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::{pac, prelude::*, rcc::Config};

struct AppSlot {
    base: u32,
    end: u32,
}
const SLOT_A: AppSlot = AppSlot {
    base: 0x0801_0000,
    end: 0x0808_0000,
};

fn read_vectors(app: &AppSlot) -> (u32, u32) {
    let sp = unsafe { (app.base as *const u32).read_volatile() };
    let rv = unsafe { (app.base as *const u32).add(1).read_volatile() };
    (sp, rv)
}

fn validate_app(app: &AppSlot) -> bool {
    let (sp, rv) = read_vectors(app);
    (rv > app.base && rv < app.end) && (rv & 0b1 > 0) && (sp > 0x2000_0000 && sp <= 0x2002_0000)
}

// Makes the STM jump to the app.
fn jump_to_app(app: &AppSlot) -> ! {
    // Stack pointer and reset vector.
    let (sp, rv) = read_vectors(app);

    // Disable interrupts and counter. Then, write the application's vector table to the VTOR.
    unsafe {
        let mut cp = cortex_m::Peripherals::steal();
        cp.SYST.disable_counter();
        cp.SYST.disable_interrupt();
        cp.SCB.vtor.write(app.base);
        bootstrap(sp as *const u32, rv as *const u32);
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi().sysclk(16.MHz()));

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    let mut delay = cp.SYST.delay(&rcc.clocks);

    // Blink LED twice to indicate that the program has started.
    led.toggle();
    delay.delay_ms(1000);
    led.toggle();
    delay.delay_ms(1000);

    cortex_m::interrupt::disable();

    // Get address to application

    if validate_app(&SLOT_A) {
        jump_to_app(&SLOT_A);
    }

    loop {}
}
