#![no_main]
#![no_std]

use bbqueue::{consts::*, framed::FrameGrantW, BBBuffer, ConstBBBuffer};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
use cpu_002 as _; // global logger + panicking-behavior + memory layout
use nrf52840_hal::{
    self as hal,
    clocks::LfOscConfiguration,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac::{Peripherals, SPIM0, SPIS1, TIMER2, UARTE0},
    ppi::{Parts as PpiParts, Ppi0},
    spim::{Frequency, Pins as SpimPins, Spim, TransferSplit, MODE_0},
    spis::{Mode, Pins as SpisPins, Spis, Transfer},
    timer::{Instance as TimerInstance, Periodic, Timer},
    uarte::{Baudrate, Parity, Pins},
};

use anachro_client::{pubsub_table, Client, ClientIoError, Error};
use anachro_server::{Broker, Uuid};

use anachro_icd::Version;
use anachro_spi::{arbitrator::EncLogicHLArbitrator, component::EncLogicHLComponent};
use anachro_spi_nrf52::{arbitrator::NrfSpiArbLL, component::NrfSpiComLL};
use heapless::{consts, Vec as HVec};

use serde::{Deserialize, Serialize};

use groundhog_nrf52::GlobalRollingTimer;
use core::sync::atomic::AtomicBool;
use stargazer_icd::{CpuTable, Keypress};

static BB_CON_OUT: BBBuffer<U2048> = BBBuffer(ConstBBBuffer::new());
static BB_CON_INC: BBBuffer<U2048> = BBBuffer(ConstBBBuffer::new());


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Demo {
    foo: u32,
    bar: i16,
    baz: (u8, u8),
}

#[rtic::app(device = crate::hal::pac, peripherals = true, monotonic = groundhog_nrf52::GlobalRollingTimer)]
const APP: () = {
    struct Resources {
        client: Client,
        anachro_spim: EncLogicHLComponent<NrfSpiComLL<SPIM0>, U2048, GlobalRollingTimer>,
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

        let gpios = P0Parts::new(board.P0);
        let ppis = PpiParts::new(board.PPI);


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

        let con_spim = Spim::new(board.SPIM0, con_pins, Frequency::M8, MODE_0, 0x00);


        let nrf_cli = NrfSpiComLL::new(con_spim, con_csn, con_go);

        let mut cio = EncLogicHLComponent::new(nrf_cli, GlobalRollingTimer::new(), &BB_CON_OUT, &BB_CON_INC).unwrap();


        let client = Client::new(
            "cpu-002",
            Version {
                major: 0,
                minor: 4,
                trivial: 1,
                misc: 123,
            },
            987,
            CpuTable::sub_paths(),
            CpuTable::pub_paths(),
            Some(250),
        );

        // Spawn periodic tasks
        ctx.spawn.anachro_periodic().ok();

        init::LateResources {
            client,
            anachro_spim: cio,
        }
    }

    #[task(resources = [client, anachro_spim], schedule = [anachro_periodic])]
    fn anachro_periodic(ctx: anachro_periodic::Context) {
        static mut HAS_CONNECTED: bool = false;

        if let Err(e) = ctx.resources.anachro_spim.poll() {
            defmt::error!("{:?}", e);
            defmt::error!("oops");
            // client.reset_connection();
        }

        let res = ctx
            .resources
            .client
            .process_one::<_, CpuTable>(ctx.resources.anachro_spim);

        match res {
            Ok(Some(_msg)) => {
                defmt::info!("ClientApp: Got one");
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
            }
        }

        if !*HAS_CONNECTED && ctx.resources.client.is_connected() {
            defmt::info!("Connected!");
            *HAS_CONNECTED = true;
        }

        ctx.schedule
            .anachro_periodic(ctx.scheduled + 10_000) // 1ms
            .ok();
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

//     client.process_one::<_, CpuTable>(&mut an_uarte).ok();

//     key_003::exit()
// }
