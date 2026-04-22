#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use log::info;
use defmt_rtt as _;
use panic_halt as _;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[derive(Clone, Copy, Debug)]
enum Button {
    Left,
    Right,
}

static SIGNAL: Signal<ThreadModeRawMutex, Button> = Signal::new();

#[embassy_executor::task]
async fn usb_logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Trace, driver);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    _spawner.spawn(usb_logger_task(driver).unwrap());

    let leds: [Output<'static>; 3] = [
        Output::new(p.PIN_15, Level::Low),
        Output::new(p.PIN_17, Level::Low),
        Output::new(p.PIN_16, Level::Low),
    ];

    _spawner.spawn(led_controller(leds).unwrap());
    _spawner.spawn(button(p.PIN_12.into(), Button::Left).unwrap());
    _spawner.spawn(button(p.PIN_14.into(), Button::Right).unwrap());

    info!("Starting the app...");
}

#[embassy_executor::task]
async fn led_controller(mut leds: [Output<'static>; 3]) {
    let mut index = 0;

    loop {
        let event = SIGNAL.wait().await;

        match event {
            Button::Left  => index = (index + 2) % 3,
            Button::Right => index = (index + 1) % 3,
        }

        for (i, led) in leds.iter_mut().enumerate() {
            if i == index { led.set_high(); } else { led.set_low(); }
        }
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: Peri<'static, AnyPin>, dir: Button) {
    let mut button: Input<'static> = Input::new(pin, Pull::None);

    loop {
        button.wait_for_low().await;
        info!("Button {:?} pressed!", dir);
        SIGNAL.signal(dir);
        Timer::after_millis(200).await;
        button.wait_for_high().await;
    }
}
