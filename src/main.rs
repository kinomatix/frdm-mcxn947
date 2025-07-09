#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;
use mcx_pac::interrupt;
use defmt::info;

use embassy_time::Timer;
use systick_timer::SystickDriver;

embassy_time_driver::time_driver_impl!(static DRIVER: SystickDriver<4>
    = SystickDriver::new(8_000_000, 7999));

#[cortex_m_rt::exception]
fn SysTick() {
    DRIVER.systick_interrupt();
}

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    info!("Booting MCXN947 with Rust");

    let mut cm_periph = cortex_m::Peripherals::take().unwrap();
    DRIVER.start(&mut cm_periph.SYST);

    loop {
        Timer::after_secs(5).await;
        info!("Waited 5 secs");
    }
}
