#![no_main]
#![no_std]

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::DelayMs;
use nrf52840_hal::{
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIS1, SPIM0,},
    spim::{Frequency, Pins as SpimPins, Spim, MODE_0, TransferSplit},
    spis::{Pins as SpisPins, Spis, Transfer},
    timer::Timer,
};
use anachro_loopback as _; // global logger + panicking-behavior + memory layout
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

// COMPONENT     ARBITRATOR
// P0.03   <=>   P1.05          SCK
// P0.04   <=>   P1.04          CIPO
// P0.28   <=>   P1.03          COPI
// P0.29   <=>   P1.02          GO      // CSn
// P0.30   <=>   P1.01          READY

// P0.31   <=>   P1.06          SCK
// P1.15   <=>   P1.07          CIPO
// P1.14   <=>   P1.08          COPI
// P1.13   <=>   P1.10          GO      // CSn
// P1.12   <=>   P1.11          READY

static BB_ARB_OUT: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );
static BB_ARB_INC: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );

static BB_CON_OUT: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );
static BB_CON_INC: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );

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

    let mut buf = [0u8; 1024];

    let board = Peripherals::take().unwrap();
    let mut p0 = P0Parts::new(board.P0);
    let mut p1 = P1Parts::new(board.P1);

    let con_pins = SpimPins {
        sck: p0.p0_03.into_push_pull_output(Level::Low).degrade(),
        miso: Some(p0.p0_04.into_floating_input().degrade()),
        mosi: Some(p0.p0_28.into_push_pull_output(Level::Low).degrade()),
    };

    let arb_pins = SpisPins {
        sck: p1.p1_05.into_floating_input().degrade(),
        cipo: Some(p1.p1_04.into_floating_input().degrade()),
        copi: Some(p1.p1_03.into_floating_input().degrade()),

        // This is problematic. Should I just common together
        // the GO pin and the real CSn? Then we can control when
        // things start/stop.
        //
        // NOTE: Common'd to p1.p1_15
        cs: p1.p1_01.into_floating_input().degrade(),
    };

    // Wrong polarity
    let mut con_go = p0.p0_29.into_floating_input().degrade();
    let mut arb_go = p1.p1_02.into_push_pull_output(Level::High).degrade();

    let mut con_ready = p0.p0_30.into_push_pull_output(Level::High).degrade();
    // TODO: See `cs` note
    let mut arb_ready = p1.p1_15.into_floating_input().degrade();

    let mut arb_spis = Spis::new(board.SPIS1, arb_pins);
    let mut con_spim = Spim::new(board.SPIM0, con_pins, Frequency::K125, MODE_0, 0x00);

    let mut timer = Timer::new(board.TIMER0);

    let mut arb_port = EncLogicHLArbitrator::new(
        Uuid::from_bytes([0x01; 16]),
        NrfSpiArbLL::new(arb_spis, arb_ready, arb_go),
        &BB_ARB_OUT,
        &BB_ARB_INC,
    ).unwrap();

    let mut broker = Broker::default();
    broker.register_client(&Uuid::from_bytes([0x01; 16])).unwrap();

    let nrf_cli = NrfSpiComLL::new(con_spim, con_ready, con_go);

    let mut cio = EncLogicHLComponent::new(
        nrf_cli,
        &BB_CON_OUT,
        &BB_CON_INC,
    ).unwrap();

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
        Some(25),
    );

    defmt::info!("Starting loop");

    while !client.is_connected() {
        defmt::info!("loop.");
        timer.delay_ms(250u32);

        // AJM: We shouldn't have to manually poll the IO like this
        if let Err(e) = cio.poll() {
            // println!("{:?}", e);
            defmt::error!("oops");
            client.reset_connection();
            continue;
        }

        match client.process_one::<_, AnachroTable>(&mut cio) {
            Ok(Some(msg)) => {
                defmt::info!("Got one");
                // defmt::info!("Got: {:?}", msg);
            }
            Ok(None) => {}
            Err(Error::ClientIoError(ClientIoError::NoData)) => {}
            Err(e) => {
                match e {
                    Error::Busy => defmt::info!("busy"),
                    Error::NotActive => defmt::info!("not active"),
                    Error::UnexpectedMessage => defmt::info!("Un Ex Me"),
                    Error::ClientIoError(cie) => {
                        match cie {
                            ClientIoError::ParsingError => defmt::info!("parseerr"),
                            ClientIoError::NoData => defmt::info!("nodata"),
                            ClientIoError::OutputFull => defmt::info!("out full"),
                        }
                        defmt::info!("Cl Io Er");
                    }
                }
                defmt::error!("error!");
                // defmt::info!("error: {:?}", e);
            }
        }

        if let Err(e) = arb_port.poll() {
            defmt::error!("poll err");
        }

        let mut out_msgs: HVec<_, consts::U16> = HVec::new();
        defmt::info!("broker sending {:?} msgs", out_msgs.len());
        broker.process_msg(&mut arb_port, &mut out_msgs).unwrap();
        for msg in out_msgs {
            // TODO: Routing
            defmt::info!("Out message!");
            if let Ok(resp) = to_slice_cobs(&msg.msg, &mut buf) {
                cio.enqueue(resp).unwrap();
            }
        }
    }

    defmt::info!("Connected!");

    anachro_loopback::exit()

}

// enum SpisState {
//     Periph(Spis<SPIS1>),
//     Transfer(Transfer<SPIS1, FrameGrantW<'static, U1024>>),
//     Unstable,
// }

// enum SpimState {
//     Periph(Spim<SPIM0>),
//     Transfer(TransferSplit<SPIM0, FrameGrantW<'static, U1024>, FrameGrantW<'static, U1024>>),
//     Unstable,
// }
