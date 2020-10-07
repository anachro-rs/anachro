#![no_main]
#![no_std]

use key_003 as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    key_003::exit()
}
