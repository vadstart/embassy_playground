#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_time::Timer;
use defmt::info;
use { defmt_rtt as _, panic_probe as _ };

// 1. create executor(s)
// 2. spawn task(s)
// 3. run executor
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting the app...");

    let p = embassy_rp::init(Default::default());
    let mut led_red: Output<'_> = Output::new(p.PIN_15, Level::Low);

    spawner.spawn(button(p.PIN_12.degrade(), "A")).unwrap();
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: AnyPin, id: &'static str) {
    let mut button: Input<'_> = Input::new(pin, Pull::None);

    loop {
        button.wait_for_low().await;
        led_red.set_high();
        Timer::after_millis(200).await; // Debounce
        button.wait_for_high().await;
        led_red.set_low();
    }
}
