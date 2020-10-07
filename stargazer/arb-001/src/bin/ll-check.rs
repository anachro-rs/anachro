#![no_main]
#![no_std]

use arb_001 as _; // global logger + panicking-behavior + memory layout
use bbqueue::{consts::*, framed::FrameGrantW, BBBuffer, ConstBBBuffer};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
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

// COMPONENT     ARBITRATOR
// P0.03   <=>   P1.05              SCK
// P0.04   <=>   P1.04              CIPO
// P0.28   <=>   P1.03              COPI
// P0.29   <=>   P1.02              GO
// P0.30   <=>   P1.01              CSn

// P0.31   <=>   P1.06          SCK
// ~P1.15~   <=>   P1.07          CIPO
// P1.14   <=>   P1.08          COPI
// P1.13   <=>   P1.10          GO      // CSn
// P1.12   <=>   P1.11          READY

static BB_ARB_OUT: BBBuffer<U1024> = BBBuffer(ConstBBBuffer::new());
static BB_ARB_INC: BBBuffer<U1024> = BBBuffer(ConstBBBuffer::new());

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

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let mut buf = [0u8; 1024];

    let board = Peripherals::take().unwrap();
    let mut p0_gpios = P0Parts::new(board.P0);
    let mut p1_gpios = P1Parts::new(board.P1);

    // D18/A0       CARD1-GO    P0.04
    let mut card1_go = p0_gpios.p0_04;
    // D19/A1       CARD2-GO    P0.05
    let mut card2_go = p0_gpios.p0_05;
    // D20/A2       CARD3-GO    P0.30
    let card3_go = p0_gpios.p0_30;
    // D21/A3       CARD4-GO    P0.28
    let card4_go = p0_gpios.p0_28;
    // D22/A4       CARD5-GO    P0.02
    let card5_go = p0_gpios.p0_02;
    // D23/A5       CARD6-GO    P0.03
    let card6_go = p0_gpios.p0_03;
    // SCLK/D15     CARD7-GO    P0.14
    let card7_go = p0_gpios.p0_14;

    // D13          CARDx-COPI  P1.09
    let cardx_copi = p1_gpios.p1_09;
    // D12          CARDx-SCK   P0.08
    let cardx_sck = p0_gpios.p0_08;
    // D11          CARDx-CSn   P0.06
    let cardx_csn = p0_gpios.p0_06;
    // D10          CARDx-CIPO  P0.27
    let cardx_cipo = p0_gpios.p0_27;

    // D9
    let d9 = p0_gpios.p0_26.into_floating_input();

    // D6           SERIAL2-TX  P0.07
    let serial2_tx = p0_gpios.p0_07;
    // D5           SERIAL2-RX  P1.08
    let serial2_rx = p1_gpios.p1_08;
    // SCL          SERIAL1-TX  P0.11
    let serial1_tx = p0_gpios.p0_11;
    // SDA          SERIAL1-RX  P0.12
    let serial1_rx = p0_gpios.p0_12;

    let arb_pins = SpisPins {
        sck: cardx_sck.into_floating_input().degrade(),
        cipo: Some(cardx_cipo.into_floating_input().degrade()),
        copi: Some(cardx_copi.into_floating_input().degrade()),
        cs: cardx_csn.into_floating_input().degrade(),
    };

    let mut arb_go = card2_go.into_push_pull_output(Level::High).degrade();

    let mut arb_spis = Spis::new(board.SPIS1, arb_pins);

    arb_spis.set_mode(Mode::Mode0);

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

    let mut ll_arb = NrfSpiArbLL::new(arb_spis, arb_go);

    let (mut prodo, _cons) = BB_ARB_OUT.try_split_framed().unwrap();
    let (mut prodi, _cons) = BB_ARB_INC.try_split_framed().unwrap();

    use anachro_spi::arbitrator::EncLogicLLArbitrator;

    for _ in 0..1000 {
        for _ in 0..2 {
            let mut ogr = prodo.grant(8).unwrap();
            let mut igr = prodi.grant(32).unwrap();

            ogr.copy_from_slice(&[69; 8]);
            igr.copy_from_slice(&[0xB4; 32]);

            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

            ll_arb
                .prepare_exchange(ogr.as_mut_ptr(), 8, igr.as_mut_ptr(), 32)
                .unwrap();

            let amt = loop {
                if let Ok(amt) = ll_arb.complete_exchange() {
                    break amt;
                }
                ll_arb.process().unwrap();
                timer.delay_us(10u32);
            };

            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

            defmt::info!("amt: {:?}", amt);
            defmt::info!("out: {:?}", &ogr[..]);
            defmt::info!("inc: {:?}", &igr[..]);
        }

        ll_arb.clear_go().unwrap();
    }

    arb_001::exit();
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
