#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use defmt_rtt as _; // global logger
use groundhog::RollingTimer;
use groundhog_nrf52::GlobalRollingTimer;
use nrf52840_hal::pac::TIMER2;
use panic_probe as _; // memory layout

#[defmt::timestamp]
fn timestamp() -> u64 {
    GlobalRollingTimer::new().get_ticks() as u64
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
