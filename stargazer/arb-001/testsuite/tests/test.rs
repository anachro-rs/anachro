#![no_std]
#![no_main]

use cortex_m_rt::entry;
use arb_001 as _; // memory layout + panic handler

#[entry]
fn main() -> ! {
    assert!(false, "TODO: Write actual tests");

    arb_001::exit();
}
