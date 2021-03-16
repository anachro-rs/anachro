//! `groundhog` - A rolling timer
//!
//! Sometimes you just want a simple rolling timer.
//!
//! Make sure you poll it often enough.

#![cfg_attr(not(test), no_std)]

use core::ops::Div;
use sealed::{Promote, RollingSince};

pub trait RollingTimer {
    type Tick: RollingSince + Promote + Div<Output = Self::Tick>;

    const TICKS_PER_SECOND: Self::Tick;

    /// Get the current tick
    fn get_ticks(&self) -> Self::Tick;

    /// Determine if the timer is initialized
    fn is_initialized(&self) -> bool;

    /// Get the number of ticks since the other measurement
    ///
    /// Make sure the old value isn't too stale.
    ///
    /// NOTE: if the timer is not initialized, the timer may
    /// return a tick value of `0` until it has been initialized.
    fn ticks_since(&self, rhs: Self::Tick) -> Self::Tick {
        self.get_ticks().since(rhs)
    }

    /// Get the number of whole seconds since the other measurement
    ///
    /// Make sure the old value isn't too stale
    fn seconds_since(&self, rhs: Self::Tick) -> Self::Tick {
        self.ticks_since(rhs) / Self::TICKS_PER_SECOND
    }

    /// Get the number of whole milliseconds since the other measurement
    ///
    /// Make sure the old value isn't too stale
    ///
    /// If the number of milliseconds is larger than Self::Tick::max(),
    /// then it will saturate at the max value
    fn millis_since(&self, rhs: Self::Tick) -> Self::Tick {
        let delta_tick = self.ticks_since(rhs);
        delta_tick.mul_then_div(
            <Self::Tick as RollingSince>::MILLIS_PER_SECOND,
            Self::TICKS_PER_SECOND,
        )
    }

    /// Get the number of whole microseconds since the other measurement
    ///
    /// Make sure the old value isn't too stale
    ///
    /// If the number of microseconds is larger than Self::Tick::max(),
    /// then it will saturate at the max value
    fn micros_since(&self, rhs: Self::Tick) -> Self::Tick {
        let delta_tick = self.ticks_since(rhs);
        delta_tick.mul_then_div(
            <Self::Tick as RollingSince>::MICROS_PER_SECOND,
            Self::TICKS_PER_SECOND,
        )
    }
}

mod sealed {
    use core::convert::TryInto;
    use core::ops::{Div, Mul};

    pub trait Promote: Sized + Copy {
        type NextSize: From<Self>
            + Ord
            + TryInto<Self>
            + Mul<Output = Self::NextSize>
            + Div<Output = Self::NextSize>;
        const MAX_VAL: Self::NextSize;

        fn promote(&self) -> Self::NextSize {
            (*self).into()
        }
        fn saturate_demote(other: Self::NextSize) -> Self {
            match Self::MAX_VAL.min(other).try_into() {
                Ok(t) => t,
                Err(_) => unsafe { core::hint::unreachable_unchecked() },
            }
        }

        fn mul_then_div(&self, mul: Self, div: Self) -> Self {
            Self::saturate_demote((self.promote() * mul.promote()) / div.promote())
        }
    }

    pub trait RollingSince {
        const MILLIS_PER_SECOND: Self;
        const MICROS_PER_SECOND: Self;
        fn since(&self, other: Self) -> Self;
    }

    impl Promote for u32 {
        type NextSize = u64;
        const MAX_VAL: u64 = 0xFFFF_FFFF;
    }

    #[cfg(feature = "u128")]
    impl Promote for u64 {
        type NextSize = u128;
        const MAX_VAL: u128 = 0xFFFF_FFFF_FFFF_FFFF;
    }

    impl RollingSince for u32 {
        const MILLIS_PER_SECOND: u32 = 1_000;
        const MICROS_PER_SECOND: u32 = 1_000_000;
        fn since(&self, other: u32) -> u32 {
            self.wrapping_sub(other)
        }
    }

    #[cfg(feature = "u128")]
    impl RollingSince for u64 {
        const MILLIS_PER_SECOND: u64 = 1_000;
        const MICROS_PER_SECOND: u64 = 1_000_000;
        fn since(&self, other: u64) -> u64 {
            self.wrapping_sub(other)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    struct TestTimer(&'static AtomicU32);
    impl RollingTimer for TestTimer {
        type Tick = u32;
        const TICKS_PER_SECOND: Self::Tick = 10;

        fn get_ticks(&self) -> u32 {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn simple_wrap() {
        assert_eq!(0x10u32.since(0xFFFFFFF0), 0x20);
        assert_eq!(0xFFFFFFF0u32.since(0x10), 0xFFFFFFE0);
    }

    #[test]
    fn timer_test() {
        static TIMER: AtomicU32 = AtomicU32::new(20);
        let timer = TestTimer(&TIMER);

        assert_eq!(timer.ticks_since(0), 20);
        assert_eq!(timer.seconds_since(0), 2);
        assert_eq!(timer.millis_since(0), 2000);
        assert_eq!(timer.micros_since(0), 2000000);

        TIMER.store(0xFFFF_FFFF, Ordering::SeqCst);

        assert_eq!(timer.ticks_since(0), 0xFFFF_FFFF);
        assert_eq!(timer.seconds_since(0), 0xFFFF_FFFF / 10);

        // Out of range
        assert_eq!(timer.millis_since(0), 0xFFFF_FFFF);
        assert_eq!(timer.micros_since(0), 0xFFFF_FFFF);

        TIMER.store(0x4000_0000, Ordering::SeqCst);

        assert_eq!(timer.ticks_since(0xC000_0000), 0x8000_0000);
        assert_eq!(timer.seconds_since(0xC000_0000), 0x8000_0000 / 10);

        // Out of range
        assert_eq!(timer.millis_since(0xC000_0000), 0xFFFF_FFFF);
        assert_eq!(timer.micros_since(0xC000_0000), 0xFFFF_FFFF);
    }
}
