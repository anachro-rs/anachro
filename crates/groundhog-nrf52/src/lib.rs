#![no_std]

use groundhog::RollingTimer;
use nrf52840_hal::{pac::timer0::RegisterBlock as RegBlock0, timer::Instance};
use rtic::{Fraction, Monotonic};

use core::sync::atomic::{AtomicPtr, Ordering};

static TIMER_PTR: AtomicPtr<RegBlock0> = AtomicPtr::new(core::ptr::null_mut());

pub struct GlobalRollingTimer;

impl GlobalRollingTimer {
    pub const fn new() -> Self {
        Self
    }

    pub fn init<T: Instance>(timer: T) {
        timer.set_periodic();
        timer.timer_start(0xFFFF_FFFFu32);
        let t0 = timer.as_timer0();

        let old_ptr = TIMER_PTR.swap(t0 as *const _ as *mut _, Ordering::SeqCst);

        debug_assert!(old_ptr == core::ptr::null_mut());
    }
}

impl Monotonic for GlobalRollingTimer {
    type Instant = i32;

    fn ratio() -> Fraction {
        Fraction {
            numerator: 64,
            denominator: 1,
        }
    }

    fn now() -> Self::Instant {
        Self::new().get_ticks() as i32
    }

    fn zero() -> Self::Instant {
        0
    }

    unsafe fn reset() {
        if let Some(t0) = TIMER_PTR.load(Ordering::SeqCst).as_ref() {
            t0.tasks_clear.write(|w| w.bits(1));
        }
    }
}

impl RollingTimer for GlobalRollingTimer {
    type Tick = u32;
    const TICKS_PER_SECOND: u32 = 1_000_000;

    fn get_ticks(&self) -> u32 {
        if let Some(t0) = unsafe { TIMER_PTR.load(Ordering::SeqCst).as_ref() } {
            t0.tasks_capture[1].write(|w| unsafe { w.bits(1) });
            t0.cc[1].read().bits()
        } else {
            0
        }
    }
}
