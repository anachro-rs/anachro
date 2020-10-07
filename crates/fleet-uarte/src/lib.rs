#![no_std]

#[cfg(feature = "52810")]
use nrf52810_hal as hal;
#[cfg(feature = "52832")]
use nrf52832_hal as hal;
#[cfg(feature = "52840")]
use nrf52840_hal as hal;

pub mod app;
pub mod buffer;
pub mod irq;
pub mod cobs_buf;
pub mod anachro_io;

#[derive(Debug)]
pub enum Error {
    Todo,
}
