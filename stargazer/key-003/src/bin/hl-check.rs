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

    let mut nrf_cli = NrfSpiComLL::new(con_spim, con_csn, con_go);
    let mut cio = EncLogicHLComponent::new(nrf_cli, hog_2, &BB_CON_OUT, &BB_CON_INC).unwrap();

    for _ in 0..20 {
        cio.enqueue(&[42u8; 32]).unwrap();
    }

    let mut ct = 0;

    while ct < 100 {
        cio.poll().unwrap();
        timer.delay_us(10u32);

        if let Some(msg) = cio.dequeue() {
            assert_eq!(&msg[..], &[69; 8]);
            msg.release();
            ct += 1;

            if ct % 10 == 0 {
                for _ in 0..10 {
                    cio.enqueue(&[42u8; 32]).unwrap();
                }
            }

            defmt::info!("Got one!");
        }
    }

    key_003::exit();
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
