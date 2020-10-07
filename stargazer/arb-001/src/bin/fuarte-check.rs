#![no_main]
#![no_std]

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use nrf52840_hal::{
    self as hal,
    clocks::LfOscConfiguration,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIS1, SPIM0, TIMER2, UARTE0},
    ppi::{Parts as PpiParts, Ppi0},
    spim::{Frequency, Pins as SpimPins, Spim, MODE_0, TransferSplit},
    spis::{Pins as SpisPins, Spis, Transfer, Mode},
    timer::{Timer, Periodic, Instance as TimerInstance},
    uarte::{Pins, Baudrate, Parity},
};
use arb_001 as _; // global logger + panicking-behavior + memory layout
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

use fleet_uarte::{
    buffer::UarteBuffer,
    buffer::UarteParts,
    anachro_io::AnachroUarte,
    cobs_buf::Buffer,
    irq::{UarteIrq, UarteTimer},
    app::UarteApp,
};

use groundhog_nrf52::GlobalRollingTimer;
use core::sync::atomic::AtomicBool;

use groundhog::RollingTimer;

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

#[rtic::app(device = crate::hal::pac, peripherals = true, monotonic = groundhog_nrf52::GlobalRollingTimer)]
const APP: () = {
    struct Resources {
        broker: Broker,
        anachro_uarte: AnachroUarte<U2048, U2048, U512>,
        uarte_timer: UarteTimer<TIMER2>,
        uarte_irq: UarteIrq<U2048, U2048, Ppi0, UARTE0>,
    }

    #[init(spawn = [anachro_periodic])]
    fn init(ctx: init::Context) -> init::LateResources {
        defmt::info!("Hello, world!");
        let board = ctx.device;

        // Setup clocks
        let clocks = hal::clocks::Clocks::new(board.CLOCK);
        let clocks = clocks.enable_ext_hfosc();
        let clocks = clocks.set_lfclk_src_external(LfOscConfiguration::NoExternalNoBypass);
        clocks.start_lfclk();

        // Setup global timer
        GlobalRollingTimer::init(board.TIMER0);


        let p0_gpios = P0Parts::new(board.P0);
        let p1_gpios = P1Parts::new(board.P1);
        let ppis = PpiParts::new(board.PPI);

        // // D18/A0       CARD1-GO    P0.04
        // let mut card1_go = p0_gpios.p0_04;
        // // D19/A1       CARD2-GO    P0.05
        // let mut card2_go = p0_gpios.p0_05;
        // // D20/A2       CARD3-GO    P0.30
        // let card3_go = p0_gpios.p0_30;
        // // D21/A3       CARD4-GO    P0.28
        // let card4_go = p0_gpios.p0_28;
        // // D22/A4       CARD5-GO    P0.02
        // let card5_go = p0_gpios.p0_02;
        // // D23/A5       CARD6-GO    P0.03
        // let card6_go = p0_gpios.p0_03;
        // // SCLK/D15     CARD7-GO    P0.14
        // let card7_go = p0_gpios.p0_14;

        // // D13          CARDx-COPI  P1.09
        // let cardx_copi = p1_gpios.p1_09;
        // // D12          CARDx-SCK   P0.08
        // let cardx_sck = p0_gpios.p0_08;
        // // D11          CARDx-CSn   P0.06
        // let cardx_csn = p0_gpios.p0_06;
        // // D10          CARDx-CIPO  P0.27
        // let cardx_cipo = p0_gpios.p0_27;

        // D9
        let d9 = p0_gpios.p0_26.into_floating_input();

        // D6           SERIAL2-TX  P0.07
        let serial2_tx = p0_gpios.p0_07;
        // D5           SERIAL2-RX  P1.08
        let serial2_rx = p1_gpios.p1_08;
        // // SCL          SERIAL1-TX  P0.11
        // let serial1_tx = p0_gpios.p0_11;
        // // SDA          SERIAL1-RX  P0.12
        // let serial1_rx = p0_gpios.p0_12;

        let UarteParts { app, timer, irq } = FLEET_BUFFER.try_split(
            Pins {
                rxd: serial2_rx.into_floating_input().degrade(),
                txd: serial2_tx.into_push_pull_output(Level::Low).degrade(),
                cts: None,
                rts: None,
            },
            Parity::EXCLUDED,
            Baudrate::BAUD1M,
            board.TIMER2,
            ppis.ppi0,
            board.UARTE0,
            255,
            10_000,
        ).map_err(drop).unwrap();

        let an_uarte = AnachroUarte::new(
            app,
            Buffer::new(),
            Uuid::from_bytes([42u8; 16]),
        );

        let mut broker = Broker::default();
        broker.register_client(&Uuid::from_bytes([42u8; 16])).unwrap();

        // Spawn periodic tasks
        ctx.spawn.anachro_periodic().ok();

        init::LateResources {
            broker,
            anachro_uarte: an_uarte,
            uarte_timer: timer,
            uarte_irq: irq,
        }


        // defmt::info!("Starting loop");

        // let mut countdown = 0;
        // let mut last_d9 = false;

        // loop {

        // }
    }

    #[task(resources = [broker, anachro_uarte], schedule = [anachro_periodic])]
    fn anachro_periodic(ctx: anachro_periodic::Context) {
        // static mut HAS_CONNECTED: bool = false;

        let broker = ctx.resources.broker;
        let uarte = ctx.resources.anachro_uarte;

        let mut out_msgs: HVec<_, consts::U16> = HVec::new();

        match broker.process_msg(uarte, &mut out_msgs) {
            Ok(_) => {},
            Err(e) => {
                defmt::error!("broker proc msg: {:?}", e);
                // arb_001::exit();
            }
        }

        if !out_msgs.is_empty() {
            defmt::info!("broker sending {:?} msgs", out_msgs.len());
        }

        let mut serout: HVec<HVec<u8, consts::U128>, consts::U16> = HVec::new();

        for msg in out_msgs {
            // TODO: Routing
            defmt::info!("Out message!");
            use postcard::to_vec_cobs;
            if let Ok(resp) = to_vec_cobs(&msg.msg) {
                defmt::info!("resp out: {:?}", &resp[..]);
                serout.push(resp).unwrap();
            } else {
                defmt::error!("Ser failed!");
                arb_001::exit();
            }
        }

        for msg in serout {
            match uarte.enqueue(&msg) {
                Ok(_) => defmt::info!("arb_port enqueued."),
                Err(()) => {
                    defmt::error!("enqueue failed!");
                    arb_001::exit();
                }
            }
        }

        ctx.schedule
            .anachro_periodic(ctx.scheduled + 1_000) // 1ms
            .ok();
    }

    #[task(binds = TIMER2, resources = [uarte_timer])]
    fn timer2(ctx: timer2::Context) {
        // fleet uarte timer
        ctx.resources.uarte_timer.interrupt();
    }

    #[task(binds = UARTE0_UART0, resources = [uarte_irq])]
    fn uarte0(ctx: uarte0::Context) {
        // fleet uarte interrupt
        ctx.resources.uarte_irq.interrupt();
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            // Don't WFI/WFE for now
            cortex_m::asm::nop();
        }
    }

    // Sacrificial hardware interrupts
    extern "C" {
        fn SWI1_EGU1();
    // fn SWI2_EGU2();
    // fn SWI3_EGU3();
    }
};

// #[cortex_m_rt::entry]
// fn main() -> ! {
//     defmt::info!("Hello, world!");


//     // defmt::error!("Connected!");

//     // arb_001::exit()

// }
