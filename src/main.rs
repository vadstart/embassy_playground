#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use defmt::{expect, info};
use { defmt_rtt as _, panic_probe as _ };

#[derive(Clone, Copy)]
enum Button {
    Left,
    Right,
}

static SIGNAL: Signal<ThreadModeRawMutex, Button> = Signal::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting the app...");

    let p = embassy_rp::init(Default::default());
    let leds: [Output<'static>; 3] = [
        Output::new(p.PIN_15, Level::Low),
        Output::new(p.PIN_17, Level::Low),
        Output::new(p.PIN_16, Level::Low)
    ];

    _spawner.spawn(led_controller(leds).unwrap());
    _spawner.spawn(button(p.PIN_12.into(), Button::Left).unwrap());
    _spawner.spawn(button(p.PIN_14.into(), Button::Right).unwrap());
}

#[embassy_executor::task]
async fn led_controller(mut leds: [Output<'static>; 3]) {
    let mut index = 0;

    loop {
        let event = SIGNAL.wait().await;

        match event {
            Button::Left => {
                index = (index + 2) % 3;
            }
            Button::Right => {
                index = (index + 1) % 3;
            }
        }

        for (i,led) in leds.iter_mut().enumerate() {
            if i == index {
                led.set_high();
            } else {
                led.set_low();
            }
        }
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: Peri<'static, AnyPin>, dir: Button) {
    let mut button: Input<'static> = Input::new(pin, Pull::None);

    loop {
        button.wait_for_low().await;
        info!("Button {} pressed!", dir);
        SIGNAL.signal(dir);
        // led.set_high();
        Timer::after_millis(200).await; // Debounce
        button.wait_for_high().await;
        // led.set_low();
    }
}
