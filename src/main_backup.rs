#![no_std]
#![no_main]
use core::time::Duration;

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_rp::peripherals::{PIN_11, PIN_12, PIN_13, PIO0, PWM_SLICE5, PWM_SLICE6, USB};
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pio_programs::pwm::{PioPwm, PioPwmProgram};
use embassy_rp::pwm::{Config, Pwm, PwmOutput, SetDutyCycle};
// use embassy_rp::usb;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use log::info;
use panic_halt as _;

// uncalibrated defaults
const DEFAULT_MIN_PULSE_WIDTH: u64 = 1000;
const DEFAULT_MAX_PULSE_WIDTH: u64 = 2000;
const DEFAULT_MAX_DEGREE_ROTATION: u64 = 160;
// Period of each cycle
const REFRESH_INTERVAL: u64 = 20000;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

static SIGNAL: Signal<ThreadModeRawMutex, bool> = Signal::new();

pub struct ServoBuilder<'d, T: Instance, const SM: usize> {
    pwm: PioPwm<'d, T, SM>,
    period: Duration,
    min_pulse_width: Duration,
    max_pulse_width: Duration,
    max_degree_rotation: u64,
}

impl<'d, T: Instance, const SM: usize> ServoBuilder<'d, T, SM> {
    pub fn new(pwm: PioPwm<'d, T, SM>) -> Self {
        Self {
            pwm,
            period: Duration::from_micros(REFRESH_INTERVAL),
            min_pulse_width: Duration::from_micros(DEFAULT_MIN_PULSE_WIDTH),
            max_pulse_width: Duration::from_micros(DEFAULT_MAX_PULSE_WIDTH),
            max_degree_rotation: DEFAULT_MAX_DEGREE_ROTATION,
        }
    }

    pub fn set_period(mut self, duration: Duration) -> Self {
        self.period = duration;
        self
    }

    pub fn set_min_pulse_width(mut self, duration: Duration) -> Self {
        self.min_pulse_width = duration;
        self
    }
    pub fn set_max_pulse_width(mut self, duration: Duration) -> Self {
        self.max_pulse_width = duration;
        self
    }
    pub fn set_max_degree_rotation(mut self, degree: u64) -> Self {
        self.max_degree_rotation = degree;
        self
    }

    pub fn build(mut self) -> Servo<'d, T, SM> {
        self.pwm.set_period(self.period);
        Servo {
            pwm: self.pwm,
            min_pulse_width: self.min_pulse_width,
            max_pulse_width: self.max_pulse_width,
            max_degree_rotation: self.max_degree_rotation,
        }
    }
}

pub struct Servo<'d, T: Instance, const SM: usize> {
    pwm: PioPwm<'d, T, SM>,
    min_pulse_width: Duration,
    max_pulse_width: Duration,
    max_degree_rotation: u64,
}

impl<'d, T: Instance, const SM: usize> Servo<'d, T, SM> {
    pub fn start(&mut self) {
        self.pwm.start();
    }

    pub fn stop(&mut self) {
        self.pwm.stop();
    }

    pub fn write_time(&mut self, duration: Duration) {
        self.pwm.write(duration);
    }

    pub fn rotate(&mut self, degree: u64) {
        let degree_per_nano_second = (self.max_pulse_width.as_nanos() as u64
            - self.min_pulse_width.as_nanos() as u64)
            / self.max_degree_rotation;
        let mut duration = Duration::from_nanos(
            degree * degree_per_nano_second + self.min_pulse_width.as_nanos() as u64,
        );
        if self.max_pulse_width < duration {
            duration = self.max_pulse_width;
        }

        self.pwm.write(duration);
    }
}

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

        // Start in red state: r (b-channel) LOW = on, g (a-channel) HIGH = off, b HIGH = off.
        // This avoids a white flash while the task hasn't called set_red() yet.
        let mut config_gr = config.clone();
        config_gr.compare_a = 65535; // g off
        config_gr.compare_b = 0; // r on

        let mut config_b = config;
        config_b.compare_b = 65535; // b off

        let (_, Some(b)) = Pwm::new_output_b(pwm_b, pin_b, config_b).split() else {
            unreachable!()
        };
        let (Some(g), Some(r)) = Pwm::new_output_ab(pwm_gr, pin_g, pin_r, config_gr).split() else {
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

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);
    let prg = PioPwmProgram::new(&mut common);
    let pwm_pio = PioPwm::new(&mut common, sm0, p.PIN_16, &prg);

    let led = RgbLed::new(p.PWM_SLICE6, p.PWM_SLICE5, p.PIN_13, p.PIN_12, p.PIN_11);

    _spawner.spawn(ultrasonic(p.PIN_19.into(), p.PIN_18.into()).unwrap());
    _spawner.spawn(servo(led, pwm_pio).unwrap());
    _spawner.spawn(button(p.PIN_10.into()).unwrap());
}

/// Ultrasonic sensor task. Only fires the trigger pulse and measures echo when the system is active.
#[embassy_executor::task]
async fn ultrasonic(trig: Peri<'static, AnyPin>, echo: Peri<'static, AnyPin>) {
    let mut trig = Output::new(trig, Level::Low);
    let echo_pin = Input::new(echo, Pull::Down);

    loop {
        // trigger pulse
        trig.set_high();
        Timer::after_micros(10).await;
        trig.set_low();

        // wait for echo start
        while echo.is_low() {}
        let start = Instant::now();

        // wait for echo end
        while echo.is_high() {}
        let duration = Instant::now() - start;
        let us = duration.as_micros() as f32;
        let distance = us * 0.0343 / 2.0;
        info!("Distance {} cm", distance);
    }
}

#[embassy_executor::task]
async fn servo(mut led: RgbLed, pwm: PioPwm<'static, PIO0, 0>) {
    let mut active = false;
    let mut degree: u64 = 0;
    let mut forward = true;
    led.set_red();

    let mut servo = ServoBuilder::new(pwm)
        .set_max_degree_rotation(180)
        .set_min_pulse_width(Duration::from_micros(500))
        .set_max_pulse_width(Duration::from_micros(2500))
        .build();

    servo.start();

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
            servo.rotate(degree);
            if forward {
                if degree < 180 {
                    degree += 1;
                } else {
                    forward = false;
                    degree -= 1;
                }
            } else if degree > 0 {
                degree -= 1;
            } else {
                forward = true;
                degree += 1;
            }
        }

        Timer::after_millis(20).await;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: Peri<'static, AnyPin>) {
    let mut button: Input<'static> = Input::new(pin, Pull::Up);
    let mut running = false;

    loop {
        button.wait_for_low().await;
        Timer::after_millis(50).await;
        if button.is_high() {
            continue; // noise or bounce, not a real press
        }
        running = !running;
        SIGNAL.signal(running);
        info!("Sonar {}", if running { "active" } else { "paused" });
        button.wait_for_high().await;
        Timer::after_millis(50).await; // debounce release
    }
}
