#![no_std]
#![no_main]

use arb_001 as _;
use cortex_m_rt::entry; // memory layout + panic handler

#[entry]
fn main() -> ! {
    assert!(false, "TODO: Write actual tests");

    arb_001::exit();
}
