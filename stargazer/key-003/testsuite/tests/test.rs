#![no_std]
#![no_main]

use cortex_m_rt::entry;
use key_003 as _; // memory layout + panic handler

#[entry]
fn main() -> ! {
    assert!(false, "TODO: Write actual tests");

    key_003::exit();
}
