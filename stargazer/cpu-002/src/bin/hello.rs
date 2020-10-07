#![no_main]
#![no_std]

use cpu_002 as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    cpu_002::exit()
}
