#![no_std]
#![no_main]

use cortex_m_rt::entry;
use anachro_loopback as _; // memory layout + panic handler

#[entry]
fn main() -> ! {
    assert!(false, "TODO: Write actual tests");

    anachro_loopback::exit();
}
