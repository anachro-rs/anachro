#![no_main]
#![no_std]


use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use nrf52840_hal::{
    self as hal,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIS1, SPIM0, TIMER2},
    spim::{Frequency, Pins as SpimPins, Spim, MODE_0, TransferSplit},
    spis::{Pins as SpisPins, Spis, Transfer, Mode},
    timer::{Timer, Periodic, Instance as TimerInstance},
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


    let (mut prodo, _cons) = BB_CON_OUT.try_split_framed().unwrap();
    let (mut prodi, _cons) = BB_CON_INC.try_split_framed().unwrap();

    use anachro_spi::component::EncLogicLLComponent;

    for _ in 0..1000 {

        while !nrf_cli.is_go_active().unwrap() {
            timer.delay_us(10u32);
        }

        defmt::info!("Starting");

        nrf_cli.notify_csn().unwrap();

        timer.delay_us(10u32);

        let mut ogr = prodo.grant(32).unwrap();
        let mut igr = prodi.grant(8).unwrap();

        ogr.copy_from_slice(&[42; 32]);
        igr.copy_from_slice(&[0xAC; 8]);

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        nrf_cli.begin_exchange(
            ogr.as_mut_ptr(),
            32,
            igr.as_mut_ptr(),
            8,
        ).unwrap();

        let amt = loop {
            if let Ok(amt) = nrf_cli.complete_exchange() {
                break amt;
            }
            timer.delay_us(10u32);
        };

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        nrf_cli.clear_csn().unwrap();

        defmt::info!("amt: {:?}", amt);
        defmt::info!("out: {:?}", &ogr[..]);
        defmt::info!("inc: {:?}", &igr[..]);

        defmt::info!("Waiting for GO to clear");

        timer.delay_ms(100u32);
    }

    key_003::exit();
}

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
