#![no_main]
#![no_std]

#![allow(unused_imports)]


use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use nrf52840_hal::{
    self as hal,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIS1, SPIM0, TIMER2},
    ppi::Parts as PpiParts,
    spim::{Frequency, Pins as SpimPins, Spim, MODE_0, TransferSplit},
    spis::{Pins as SpisPins, Spis, Transfer, Mode},
    timer::{Timer, Periodic, Instance as TimerInstance},
    uarte::{Pins, Baudrate, Parity},
};
use key_003 as _; // global logger + panicking-behavior + memory layout
use bbqueue::{
    consts::*,
    BBBuffer,
    ConstBBBuffer,
    framed::FrameGrantW,
};

use anachro_server::{Broker, Uuid};
use anachro_client::{pubsub_table, Client, ClientIoError, Error};

use anachro_spi::{
    arbitrator::EncLogicHLArbitrator,
    component::EncLogicHLComponent,
};
use anachro_spi_nrf52::{
    arbitrator::NrfSpiArbLL,
    component::NrfSpiComLL,
};
use anachro_icd::Version;
use heapless::{consts, Vec as HVec};
use postcard::to_slice_cobs;

use serde::{Deserialize, Serialize};

use groundhog::RollingTimer;

use fleet_uarte::{
    buffer::UarteBuffer,
    buffer::UarteParts,
    anachro_io::AnachroUarte,
    cobs_buf::Buffer,
};

use core::sync::atomic::AtomicBool;

static FLEET_BUFFER: UarteBuffer<U2048, U2048> = UarteBuffer {
    txd_buf: BBBuffer( ConstBBBuffer::new() ),
    rxd_buf: BBBuffer( ConstBBBuffer::new() ),
    timeout_flag: AtomicBool::new(false),
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Demo {
    foo: u32,
    bar: i16,
    baz: (u8, u8),
}

pubsub_table! {
    AnachroTable,
    Subs => {
        Something: "foo/bar/baz" => Demo,
        Else: "bib/bim/bap" => (),
    },
    Pubs => {
        Etwas: "short/send" => (),
        Anders: "send/short" => (),
    },
}


#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let board = hal::pac::Peripherals::take().unwrap();

    let gpios = P0Parts::new(board.P0);
    let ppis = PpiParts::new(board.PPI);

    let mut pin_rx = gpios.p0_15.into_floating_input().degrade();
    let mut pin_tx = gpios.p0_16.into_push_pull_output(Level::Low).degrade();

    let UarteParts { app, timer, irq } = FLEET_BUFFER.try_split(
        Pins {
            rxd: pin_rx,
            txd: pin_tx,
            cts: None,
            rts: None,
        },
        Parity::EXCLUDED,
        Baudrate::BAUD1M,
        board.TIMER1,
        ppis.ppi0,
        board.UARTE0,
        256,
        10_000,
    ).unwrap();

    let mut timer = Timer::new(board.TIMER0);

    let buf: Buffer<U512> = Buffer::new();

    let mut an_uarte = AnachroUarte::new(
        app,
        buf,
        Uuid::from_bytes([42u8; 16]),
    );


    let mut client = Client::new(
        "loopy",
        Version {
            major: 0,
            minor: 4,
            trivial: 1,
            misc: 123,
        },
        987,
        AnachroTable::sub_paths(),
        AnachroTable::pub_paths(),
        Some(250),
    );

    client.process_one::<_, AnachroTable>(&mut an_uarte).ok();


    key_003::exit()
}
