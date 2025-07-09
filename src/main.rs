#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use mcx_pac::interrupt;
use panic_probe as _;

use embassy_time::Timer;
use systick_timer::SystickDriver;

pub use mcx_pac as pac;

mod gpio;

embassy_time_driver::time_driver_impl!(static DRIVER: SystickDriver<4>
    = SystickDriver::new(8_000_000, 7999));

#[cortex_m_rt::exception]
fn SysTick() {
    DRIVER.systick_interrupt();
}

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    info!("Booting MCXN947 with Rust");
    gpio::init();

    let mut wake_up_btn = unsafe { gpio::P0_23.steal() };
    wake_up_btn.set_as_input();

    let mut r_led = unsafe { gpio::P0_10.steal() };
    r_led.set_as_output();

    let mut cm_periph = cortex_m::Peripherals::take().unwrap();
    DRIVER.start(&mut cm_periph.SYST);

    loop {
        Timer::after_secs(5).await;
        info!("Waited 5 secs");
        if (wake_up_btn.is_set()) {
            info!("WakeUp set!");
        }
        r_led.toggle();
    }
}
