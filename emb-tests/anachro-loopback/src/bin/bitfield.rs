#![no_main]
#![no_std]

use anachro_loopback as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    // value of the FREQUENCY register (nRF52840 device; RADIO peripheral)
    let frequency: u32 = 276;
    defmt::debug!("FREQUENCY: {0:0..7}, MAP: {0:8..9}", frequency);

    anachro_loopback::exit()
}
