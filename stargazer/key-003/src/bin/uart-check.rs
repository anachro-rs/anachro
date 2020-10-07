#![no_main]
#![no_std]
#![allow(unused_imports)]

use bbqueue::{consts::*, framed::FrameGrantW, BBBuffer, ConstBBBuffer};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
use key_003 as _; // global logger + panicking-behavior + memory layout
use nrf52840_hal::{
    self as hal,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIM0, SPIS1, TIMER2},
    spim::{Frequency, Pins as SpimPins, Spim, TransferSplit, MODE_0},
    spis::{Mode, Pins as SpisPins, Spis, Transfer},
    timer::{Instance as TimerInstance, Periodic, Timer},
};

use anachro_client::{pubsub_table, Client, ClientIoError, Error};
use anachro_server::{Broker, Uuid};

use anachro_icd::Version;
use anachro_spi::{arbitrator::EncLogicHLArbitrator, component::EncLogicHLComponent};
use anachro_spi_nrf52::{arbitrator::NrfSpiArbLL, component::NrfSpiComLL};
use heapless::{consts, Vec as HVec};
use postcard::to_slice_cobs;

use serde::{Deserialize, Serialize};

use groundhog::RollingTimer;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let board = hal::pac::Peripherals::take().unwrap();

    let gpios = P0Parts::new(board.P0);

    let mut pin_rx = gpios.p0_15.into_push_pull_output(Level::Low);
    let mut pin_tx = gpios.p0_16.into_push_pull_output(Level::Low);

    let mut timer = Timer::new(board.TIMER0);

    loop {
        defmt::info!("Both low");
        pin_rx.set_low().ok();
        pin_tx.set_low().ok();
        timer.delay_ms(3000u32);

        defmt::info!("RX High, TX Low");
        pin_rx.set_high().ok();
        pin_tx.set_low().ok();
        timer.delay_ms(3000u32);

        defmt::info!("RX Low, TX High");
        pin_rx.set_low().ok();
        pin_tx.set_high().ok();
        timer.delay_ms(3000u32);

        defmt::info!("Both High");
        pin_rx.set_high().ok();
        pin_tx.set_high().ok();
        timer.delay_ms(3000u32);
    }

    key_003::exit()
}
