#![no_std]
#![no_main]

use cortex_m_rt::entry;
use spi_loopback as _; // memory layout + panic handler

#[entry]
fn main() -> ! {
    assert!(false, "TODO: Write actual tests");

    spi_loopback::exit();
}
