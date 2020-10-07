#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use defmt_rtt as _; // global logger
use nrf52840_hal::pac::TIMER2;
use panic_probe as _; // memory layout

#[defmt::timestamp]
fn timestamp() -> u64 {
    let tptr = unsafe { &*TIMER2::ptr() };

    tptr.tasks_capture[1].write(|w| unsafe { w.bits(1) });
    let time = tptr.cc[1].read().bits();
    time as u64
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
