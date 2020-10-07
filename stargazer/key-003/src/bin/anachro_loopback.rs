#![no_main]
#![no_std]

use bbqueue::{consts::*, framed::FrameGrantW, BBBuffer, ConstBBBuffer};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
use key_003 as _; // global logger + panicking-behavior + memory layout
use nrf52840_hal::{
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
        cs: p1.p1_01.into_floating_input().degrade(),
    };

    let mut con_go = p0.p0_29.into_floating_input().degrade();
    let mut arb_go = p1.p1_02.into_push_pull_output(Level::High).degrade();

    let mut con_csn = p0.p0_30.into_push_pull_output(Level::High).degrade();

    let mut arb_spis = Spis::new(board.SPIS1, arb_pins);
    let mut con_spim = Spim::new(board.SPIM0, con_pins, Frequency::M8, MODE_0, 0x00);

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

    let mut arb_port = EncLogicHLArbitrator::new(
        Uuid::from_bytes([0x01; 16]),
        NrfSpiArbLL::new(arb_spis, arb_go),
        hog_1,
        &BB_ARB_OUT,
        &BB_ARB_INC,
    )
    .unwrap();

    let mut broker = Broker::default();
    broker
        .register_client(&Uuid::from_bytes([0x01; 16]))
        .unwrap();

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
        Some(25),
    );

    defmt::info!("Starting loop");

    let mut countdown = 0;

    while !client.is_connected() {
        timer.delay_us(10u32);

        if let Err(e) = cio.poll() {
            // println!("{:?}", e);
            defmt::error!("oops");
            client.reset_connection();
            continue;
        }

        if let Err(e) = arb_port.poll() {
            defmt::error!("poll err");
        }

        countdown += 1;

        if countdown >= 10 {
            countdown = 0;
        } else {
            continue;
        }

        arb_port.query_component().ok();

        defmt::info!("loop.");
        // timer.delay_ms(1u32);

        // AJM: We shouldn't have to manually poll the IO like this

        match client.process_one::<_, AnachroTable>(&mut cio) {
            Ok(Some(msg)) => {
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
                defmt::error!("ClientApp: error!");
                // defmt::info!("error: {:?}", e);
            }
        }

        let mut out_msgs: HVec<_, consts::U16> = HVec::new();
        defmt::info!("broker sending {:?} msgs", out_msgs.len());
        match broker.process_msg(&mut arb_port, &mut out_msgs) {
            Ok(_) => {}
            Err(e) => {
                defmt::error!("broker proc msg: {:?}", e);
                key_003::exit();
            }
        }

        let mut serout: HVec<HVec<u8, consts::U128>, consts::U16> = HVec::new();

        for msg in out_msgs {
            // TODO: Routing
            defmt::info!("Out message!");
            use postcard::to_vec_cobs;
            if let Ok(resp) = to_vec_cobs(&msg.msg) {
                defmt::info!("resp out: {:?}", &resp[..]);
                // match cio.enqueue(resp) {
                //     Ok(_) => defmt::info!("cio enqueued."),
                //     Err(e) => {
                //         defmt::error!("enqueue failed: {:?}", e);
                //         key_003::exit();
                //     }
                // }
                serout.push(resp).unwrap();
            }
        }

        for msg in serout {
            match arb_port.enqueue(&msg) {
                Ok(_) => defmt::info!("arb_port enqueued."),
                Err(e) => {
                    defmt::error!("enqueue failed: {:?}", e);
                    key_003::exit();
                }
            }
        }
    }

    defmt::error!("Connected!");

    key_003::exit()
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
