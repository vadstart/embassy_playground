#![no_std]
#![no_main]

use cortex_m::Peripherals;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use panic_halt as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 1. create executor(s)
    // 2. spawn task(s)
    // 3. run executor
    info!("Starting the app...");

    let p: Peripherals = embassy_rp::init(Default::default());
}

// use embedded_hal::digital::OutputPin;
// use panic_halt as _;
// use rp235x_hal::clocks::init_clocks_and_plls;
// use rp235x_hal::gpio::Pins;
// use rp235x_hal::{self as hal, entry};
// use rp235x_hal::{Clock, pac};

// #[unsafe(link_section = ".start_block")]
// #[used]
// pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

// #[entry]
// fn main() -> ! {
// Take ownership of the peripherals
// let mut pac = pac::Peripherals::take().unwrap();
// let core = cortex_m::Peripherals::take().unwrap();
// // Set up the watchdog driver - needed by the clock setup code
// let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
// let sio = hal::Sio::new(pac.SIO);
//
// // External high-speed crystal on the pico board is 12Mhz
// let external_xtal_freq_hz = 12_000_000u32;
// let clocks = init_clocks_and_plls(
//     external_xtal_freq_hz,
//     pac.XOSC,
//     pac.CLOCKS,
//     pac.PLL_SYS,
//     pac.PLL_USB,
//     &mut pac.RESETS,
//     &mut watchdog,
// )
// .ok()
// .unwrap();
//
// let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
//
// // GPIO 32 -> onboard LED
// let pins = Pins::new(
//     pac.IO_BANK0,
//     pac.PADS_BANK0,
//     sio.gpio_bank0,
//     &mut pac.RESETS,
// );
//
// let mut led_pin = pins.gpio15.into_push_pull_output();
//
// loop {
//     led_pin.set_high().unwrap();
//     delay.delay_ms(5000);
//     led_pin.set_low().unwrap();
//     delay.delay_ms(500);
// }
// }
