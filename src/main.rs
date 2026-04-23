#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_rp::peripherals::{PIN_11, PIN_12, PIN_13, PWM_SLICE5, PWM_SLICE6, USB};
use embassy_rp::pwm::{Config, Pwm, PwmOutput, SetDutyCycle};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use log::info;
use panic_halt as _;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static SIGNAL: Signal<ThreadModeRawMutex, bool> = Signal::new();

struct RgbLed {
    r: PwmOutput<'static>,
    g: PwmOutput<'static>,
    b: PwmOutput<'static>,
}

impl RgbLed {
    fn new(
        pwm_gr: Peri<'static, PWM_SLICE6>,
        pwm_b: Peri<'static, PWM_SLICE5>,
        pin_r: Peri<'static, PIN_13>,
        pin_g: Peri<'static, PIN_12>,
        pin_b: Peri<'static, PIN_11>,
    ) -> Self {
        let mut config = Config::default();
        config.top = 65535;
        config.divider = 16u8.into();

        let (_, Some(b)) = Pwm::new_output_b(pwm_b, pin_b, config.clone()).split() else {
            unreachable!()
        };
        let (Some(g), Some(r)) = Pwm::new_output_ab(pwm_gr, pin_g, pin_r, config).split() else {
            unreachable!()
        };

        Self { r, g, b }
    }

    fn set_red(&mut self) {
        self.r.set_duty_cycle_fully_off().unwrap();
        self.g.set_duty_cycle_fully_on().unwrap();
        self.b.set_duty_cycle_fully_on().unwrap();
    }

    fn set_green(&mut self) {
        self.r.set_duty_cycle_fully_on().unwrap();
        self.g.set_duty_cycle_fully_off().unwrap();
        self.b.set_duty_cycle_fully_on().unwrap();
    }
}

#[embassy_executor::task]
async fn usb_logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Trace, driver);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    _spawner.spawn(usb_logger_task(driver).unwrap());

    let led = RgbLed::new(p.PWM_SLICE6, p.PWM_SLICE5, p.PIN_13, p.PIN_12, p.PIN_11);
    _spawner.spawn(servo(led).unwrap());
    _spawner.spawn(button(p.PIN_10.into()).unwrap());

    info!("Starting the app...");
}

#[embassy_executor::task]
async fn servo(mut led: RgbLed) {
    let mut active = true;
    led.set_green(); // initial state

    loop {
        if let Some(new_state) = SIGNAL.try_take() {
            active = new_state;
            if active {
                led.set_green();
            } else {
                led.set_red();
            }
        }

        if active {
            // Rotate 180deg
        }

        Timer::after_millis(20).await;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: Peri<'static, AnyPin>) {
    let mut button: Input<'static> = Input::new(pin, Pull::None);
    let mut running = true;

    loop {
        button.wait_for_low().await;
        running = !running;
        SIGNAL.signal(running);
        info!("Sonar {}", if running { "active" } else { "paused" });
        Timer::after_millis(200).await;
        button.wait_for_high().await;
    }
}
