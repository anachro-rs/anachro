#![no_main]
#![no_std]

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

    // CIPO         P0.15
    let cipo = gpios.p0_15;
    // COPI         P0.13
    let copi = gpios.p0_13;
    // SCK          P0.14
    let sck = gpios.p0_14;
    // A5  CSn      P0.03
    let a5 = gpios.p0_03;
    // A4  GO       P0.02
    let go = gpios.p0_02;

    let con_pins = SpimPins {
        sck: sck.into_push_pull_output(Level::Low).degrade(),
        miso: Some(cipo.into_floating_input().degrade()),
        mosi: Some(copi.into_push_pull_output(Level::Low).degrade()),
    };

    let con_go = go.into_floating_input().degrade();
    let con_csn = a5.into_push_pull_output(Level::High).degrade();

    let con_spim = Spim::new(board.SPIM0, con_pins, Frequency::M2, MODE_0, 0x00);

    let mut timer = Timer::new(board.TIMER0);

    use embedded_hal::timer::CountDown;

    let mut ts_timer = Timer::new(board.TIMER2);
    ts_timer.start(0xFFFF_FFFFu32);

    let mut rolling_timer_1 = Timer::new(board.TIMER4).into_periodic();
    rolling_timer_1.start(0xFFFF_FFFFu32);

    let mut rolling_timer_2 = Timer::new(board.TIMER3).into_periodic();
    rolling_timer_2.start(0xFFFF_FFFFu32);

    let hog_1 = TimeHog {
        timer: rolling_timer_1,
    };

    let hog_2 = TimeHog {
        timer: rolling_timer_2,
    };

    let nrf_cli = NrfSpiComLL::new(con_spim, con_csn, con_go);

    let mut cio = EncLogicHLComponent::new(nrf_cli, hog_2, &BB_CON_OUT, &BB_CON_INC).unwrap();

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

    defmt::info!("Starting loop");

    let mut countdown = 0;

    while !client.is_connected() {
        timer.delay_us(10u32);

        if let Err(e) = cio.poll() {
            defmt::error!("{:?}", e);
            defmt::error!("oops");
            // client.reset_connection();
            continue;
        }

        countdown += 1;

        if countdown >= 10 {
            countdown = 0;
        } else {
            continue;
        }

        // defmt::info!("loop.");
        // timer.delay_ms(1u32);

        // AJM: We shouldn't have to manually poll the IO like this

        match client.process_one::<_, AnachroTable>(&mut cio) {
            Ok(Some(_msg)) => {
                defmt::info!("ClientApp: Got one");
                // defmt::info!("Got: {:?}", msg);
            }
            Ok(None) => {}
            Err(Error::ClientIoError(ClientIoError::NoData)) => {}
            Err(e) => {
                match e {
                    Error::Busy => defmt::info!("ClientApp: busy"),
                    Error::NotActive => defmt::info!("ClientApp: not active"),
                    Error::UnexpectedMessage => defmt::info!("ClientApp: Un Ex Me"),
                    Error::ClientIoError(cie) => {
                        match cie {
                            ClientIoError::ParsingError => defmt::info!("ClientApp: parseerr"),
                            ClientIoError::NoData => defmt::info!("ClientApp: nodata"),
                            ClientIoError::OutputFull => defmt::info!("ClientApp: out full"),
                        }
                        defmt::info!("ClientApp: Cl Io Er");
                    }
                }
                // defmt::error!("ClientApp: error!");
                // defmt::info!("error: {:?}", e);
            }
        }
    }

    defmt::error!("Connected!");

    key_003::exit()
}

static BB_CON_OUT: BBBuffer<U1024> = BBBuffer(ConstBBBuffer::new());
static BB_CON_INC: BBBuffer<U1024> = BBBuffer(ConstBBBuffer::new());

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

struct TimeHog<T: TimerInstance> {
    timer: Timer<T, Periodic>,
}

impl<T: TimerInstance> RollingTimer for TimeHog<T> {
    type Tick = u32;
    const TICKS_PER_SECOND: Self::Tick = 1_000_000;

    fn get_ticks(&self) -> Self::Tick {
        self.timer.read()
    }
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
