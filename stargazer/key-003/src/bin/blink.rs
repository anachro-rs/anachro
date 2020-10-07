#![no_main]
#![no_std]

use {
    embedded_hal::blocking::delay::DelayMs,
    key_003 as _, // global logger + panicking-behavior + memory layout
    nrf52840_hal::{
        self as hal,
        gpio::{p0::Parts as P0Parts, Level},
        Rng, Timer,
    },
    nrf_smartled::pwm::Pwm,
    smart_leds::{colors, gamma, RGB8},
    smart_leds_trait::SmartLedsWrite,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let board = hal::pac::Peripherals::take().unwrap();

    let mut timer = Timer::new(board.TIMER0);
    let gpios = P0Parts::new(board.P0);

    let sdout = gpios.p0_16.into_push_pull_output(Level::Low);

    let mut leds = Pwm::new(board.PWM0, sdout.degrade());

    let _rng = Rng::new(board.RNG);
    let mut pixels = [RGB8::default(); 60];
    let mut base_pixels = [RGB8::default(); 60];

    leds.write(pixels.iter().cloned()).ok();

    let color_path = &[
        colors::RED,
        colors::ORANGE,
        colors::YELLOW,
        colors::GREEN,
        colors::BLUE,
        colors::INDIGO,
        colors::VIOLET,
    ];

    let mut ct: u8 = 0;

    let mut color_iter = color_path.iter().cycle();

    let mut num: u8 = 0;
    pixels.iter_mut().for_each(|pixel| {
        pixel.r = pixel.r + num;
        num = num.wrapping_add(10);
    });

    loop {
        if (ct == 0) || (ct == 128) {
            defmt::info!("New colors!");
            for (pix, col) in base_pixels.iter_mut().zip(&mut color_iter) {
                *pix = *col;
            }
        }

        for (pixel, base) in pixels.iter_mut().zip(base_pixels.iter()) {
            pixel.r = libm::fabsf(
                (base.r as f32) * (libm::sinf((ct as f32 / 255.0) * core::f32::consts::PI * 2.0)),
            ) as u8;
            pixel.g = libm::fabsf(
                (base.g as f32) * (libm::sinf((ct as f32 / 255.0) * core::f32::consts::PI * 2.0)),
            ) as u8;
            pixel.b = libm::fabsf(
                (base.b as f32) * (libm::sinf((ct as f32 / 255.0) * core::f32::consts::PI * 2.0)),
            ) as u8;
        }

        ct += 1;

        leds.write(gamma(pixels.iter().cloned())).ok();
        timer.delay_ms(10u32);
    }
}
