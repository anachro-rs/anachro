#![no_main]
#![no_std]

use bbqueue::{consts::*, framed::FrameGrantW, BBBuffer, ConstBBBuffer};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::OutputPin;
use key_003 as _; // global logger + panicking-behavior + memory layout
use nrf52840_hal::{
    self as hal,
    clocks::LfOscConfiguration,
    gpio::{
        p0::Parts as P0Parts,
        p1::{Parts as P1Parts, P1_04},
        Input, Level, Output, Pin, PullUp, PushPull,
    },
    pac::{Peripherals, TIMER2, TWIM0, UARTE0},
    ppi::{Parts as PpiParts, Ppi0},
    spim::{Frequency, Pins as SpimPins, Spim, TransferSplit, MODE_0},
    spis::{Mode, Pins as SpisPins, Spis, Transfer},
    timer::{Instance as TimerInstance, Periodic, Timer},
    twim::{Frequency as TwimFrequency, Pins as TwimPins},
    uarte::{Baudrate, Parity, Pins},
    Twim,
};

use anachro_client::{pubsub_table, Client, ClientIoError, Error};
use anachro_server::{Broker, Uuid};

use anachro_icd::Version;
use anachro_spi::{arbitrator::EncLogicHLArbitrator, component::EncLogicHLComponent};
use anachro_spi_nrf52::{arbitrator::NrfSpiArbLL, component::NrfSpiComLL};
use heapless::{consts, Vec as HVec};

use serde::{Deserialize, Serialize};

use groundhog_nrf52::GlobalRollingTimer;

use core::iter::{Cloned, Cycle};
use core::slice::Iter;
use core::sync::atomic::AtomicBool;
use embedded_hal::blocking::i2c::Write;
use embedded_hal::blocking::i2c::WriteRead;
use embedded_hal::digital::v2::InputPin;
use fleet_uarte::{
    anachro_io::AnachroUarte,
    app::UarteApp,
    buffer::UarteBuffer,
    buffer::UarteParts,
    cobs_buf::Buffer,
    irq::{UarteIrq, UarteTimer},
};
use groundhog::RollingTimer;
use smart_leds::{colors, gamma, Gamma, RGB8};
use stargazer_icd::{KeyboardTable, Keypress};
use heapless::Vec;

static FLEET_BUFFER: UarteBuffer<U2048, U2048> = UarteBuffer {
    txd_buf: BBBuffer(ConstBBBuffer::new()),
    rxd_buf: BBBuffer(ConstBBBuffer::new()),
    timeout_flag: AtomicBool::new(false),
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Demo {
    foo: u32,
    bar: i16,
    baz: (u8, u8),
}


const COLORS: &[RGB8] = &[
    colors::RED,
    colors::ORANGE,
    colors::YELLOW,
    colors::GREEN,
    colors::BLUE,
    colors::INDIGO,
    colors::VIOLET,
];

#[rtic::app(device = crate::hal::pac, peripherals = true, monotonic = groundhog_nrf52::GlobalRollingTimer)]
const APP: () = {
    struct Resources {
        client: Client,
        anachro_uarte: AnachroUarte<U2048, U2048, U512>,
        uarte_timer: UarteTimer<TIMER2>,
        uarte_irq: UarteIrq<U2048, U2048, Ppi0, UARTE0>,
        rows: [Pin<Output<PushPull>>; 8],
        cols: [Pin<Input<PullUp>>; 8],
        leds: IS31FL3733,
    }

    #[init(spawn = [anachro_periodic, key_periodic])]
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

        let gpios_p0 = P0Parts::new(board.P0);
        let gpios_p1 = P1Parts::new(board.P1);
        let ppis = PpiParts::new(board.PPI);

        let pin_rx = gpios_p0.p0_15.into_floating_input().degrade();
        let pin_tx = gpios_p0.p0_16.into_push_pull_output(Level::Low).degrade();

        // # ROWS = (P0_05, P0_06, P0_07, P0_08, P1_09, P1_08, P0_12, P0_11)
        let mut rows = [
            gpios_p0.p0_05.into_push_pull_output(Level::Low).degrade(), // Start with first row low,
            gpios_p0.p0_06.into_push_pull_output(Level::High).degrade(), // and all other rows high
            gpios_p0.p0_07.into_push_pull_output(Level::High).degrade(),
            gpios_p0.p0_08.into_push_pull_output(Level::High).degrade(),
            gpios_p1.p1_09.into_push_pull_output(Level::High).degrade(),
            gpios_p1.p1_08.into_push_pull_output(Level::High).degrade(),
            gpios_p0.p0_12.into_push_pull_output(Level::High).degrade(),
            gpios_p0.p0_11.into_push_pull_output(Level::High).degrade(),
        ];

        // # COLS = (P0_19, P0_20, P0_21, P0_22, P0_23, P0_24, P0_25, P0_26)
        let mut cols = [
            gpios_p0.p0_19.into_pullup_input().degrade(),
            gpios_p0.p0_20.into_pullup_input().degrade(),
            gpios_p0.p0_21.into_pullup_input().degrade(),
            gpios_p0.p0_22.into_pullup_input().degrade(),
            gpios_p0.p0_23.into_pullup_input().degrade(),
            gpios_p0.p0_24.into_pullup_input().degrade(),
            gpios_p0.p0_25.into_pullup_input().degrade(),
            gpios_p0.p0_26.into_pullup_input().degrade(),
        ];

        let twim = Twim::new(
            board.TWIM0,
            TwimPins {
                scl: gpios_p1.p1_06.into_floating_input().degrade(),
                sda: gpios_p1.p1_05.into_floating_input().degrade(),
            },
            TwimFrequency::K400, // ?
        );
        let mut leds = IS31FL3733::new(twim, gpios_p1.p1_04.into_push_pull_output(Level::Low));
        let gtmr = GlobalRollingTimer::new();
        let start = gtmr.get_ticks();

        while gtmr.millis_since(start) < 50 {}
        leds.reset().unwrap();
        while gtmr.millis_since(start) < 100 {}
        leds.setup().unwrap();
        while gtmr.millis_since(start) < 150 {}

        let UarteParts { app, timer, irq } = FLEET_BUFFER
            .try_split(
                Pins {
                    rxd: pin_rx,
                    txd: pin_tx,
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
            )
            .map_err(drop)
            .unwrap();

        let an_uarte = AnachroUarte::new(app, Buffer::new(), Uuid::from_bytes([42u8; 16]));

        let client = Client::new(
            "key-003",
            Version {
                major: 0,
                minor: 4,
                trivial: 1,
                misc: 123,
            },
            987,
            KeyboardTable::sub_paths(),
            KeyboardTable::pub_paths(),
            Some(250),
        );

        // Spawn periodic tasks
        ctx.spawn.anachro_periodic().ok();
        ctx.spawn.key_periodic().ok();

        init::LateResources {
            client,
            anachro_uarte: an_uarte,
            uarte_timer: timer,
            uarte_irq: irq,
            rows,
            cols,
            leds,
        }
    }

    #[task(resources = [client, anachro_uarte], schedule = [anachro_periodic])]
    fn anachro_periodic(ctx: anachro_periodic::Context) {
        static mut HAS_CONNECTED: bool = false;

        let res = ctx
            .resources
            .client
            .process_one::<_, KeyboardTable>(ctx.resources.anachro_uarte);

        if !*HAS_CONNECTED && ctx.resources.client.is_connected() {
            defmt::info!("Connected!");
            *HAS_CONNECTED = true;
        }

        match res {
            Ok(Some(_msg)) => defmt::info!("Got a message!"),
            Ok(None) => {}
            Err(e) => defmt::error!("ERR: {:?}", e),
        }

        ctx.schedule
            .anachro_periodic(ctx.scheduled + 10_000) // 10ms
            .ok();
    }

    #[task(resources = [rows, cols, leds, client, anachro_uarte], schedule = [key_periodic])]
    fn key_periodic(ctx: key_periodic::Context) {
        static mut LAST_STATE: [[bool; 8]; 8] = [[false; 8]; 8];
        static mut ROW_IDX: usize = 0;
        static mut COLOR_ITER: Option<Gamma<Cycle<Cloned<Iter<'static, RGB8>>>>> = None;

        if COLOR_ITER.is_none() {
            *COLOR_ITER = Some(gamma(COLORS.iter().cloned().cycle()));
        }

        let color_iter = COLOR_ITER.as_mut().unwrap();

        for (c_idx, col) in ctx.resources.cols.iter_mut().enumerate() {
            if let Ok(true) = col.is_low() {
                if !LAST_STATE[*ROW_IDX][c_idx] {
                    defmt_key(*ROW_IDX, c_idx);

                    let mut buf = [0u8; 1024];

                    //  TODO - can I do this automatically?
                    if let Some(key) = char_key(*ROW_IDX, c_idx) {
                        let pubby = match KeyboardTable::Key(Keypress{ character: key }).serialize(&mut buf) {
                            Ok(pb) => {
                                match ctx.resources.client.publish(ctx.resources.anachro_uarte, pb.path, pb.buf) {
                                    Ok(_) => defmt::info!("Sent Pub!"),
                                    Err(_) => defmt::error!("Pub Send Error!"),
                                }
                            },
                            Err(_) => {

                            },
                        };
                    }

                    ctx.resources
                        .leds
                        .update_pixel(((*ROW_IDX * 8) + c_idx) as u8, color_iter.next().unwrap())
                        .unwrap();
                }
                LAST_STATE[*ROW_IDX][c_idx] = true;
            } else {
                if LAST_STATE[*ROW_IDX][c_idx] {
                    ctx.resources
                        .leds
                        .update_pixel(((*ROW_IDX * 8) + c_idx) as u8, colors::BLACK)
                        .unwrap();
                }
                LAST_STATE[*ROW_IDX][c_idx] = false;
            }
        }

        ctx.resources.rows[*ROW_IDX].set_high().ok();

        *ROW_IDX += 1;
        if *ROW_IDX >= ctx.resources.rows.len() {
            *ROW_IDX = 0;
        }

        ctx.resources.rows[*ROW_IDX].set_low().ok();

        ctx.schedule
            .key_periodic(ctx.scheduled + 1_000) // 1ms
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

pub struct IS31FL3733 {
    i2c: Twim<TWIM0>,
    power: P1_04<Output<PushPull>>,
}

impl IS31FL3733 {
    fn new(i2c: Twim<TWIM0>, mut power: P1_04<Output<PushPull>>) -> Self {
        power.set_high().ok();
        Self { i2c, power }
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), ()> {
        <Twim<TWIM0> as Write>::write(&mut self.i2c, 0x50, buf).map_err(drop)
    }

    fn page(&mut self, page: u8) -> Result<(), ()> {
        self.write(&[0xFE, 0xC5])?;
        self.write(&[0xFD, page])
    }

    fn reset(&mut self) -> Result<(), ()> {
        self.page(3)?;
        self.read(0x11)
    }

    fn read(&mut self, reg_id: u8) -> Result<(), ()> {
        let inbuf = [reg_id];
        let mut outbuf = [0u8; 1];
        self.i2c.write_read(0x50, &inbuf, &mut outbuf).map_err(drop)
    }

    fn set_brightness(&mut self, brightness: u8) -> Result<(), ()> {
        self.page(3)?;
        self.write(&[1, brightness])
    }

    fn setup(&mut self) -> Result<(), ()> {
        self.page(3)?;
        self.write(&[2, (2 << 5) | (0 << 1)])?;
        self.write(&[3, (2 << 5) | (3 << 1)])?;
        self.write(&[4, (0 << 4)])?;

        self.write(&[6, (2 << 5) | (0 << 1)])?;
        self.write(&[7, (2 << 5) | (2 << 1)])?;
        self.write(&[8, (0 << 4)])?;

        self.write(&[0xA, (1 << 5) | (0 << 1)])?;
        self.write(&[0xB, (1 << 5) | (1 << 1)])?;
        self.write(&[0xC, (0 << 4)])?;

        self.write(&[0, 1])?;
        self.write(&[0, 3])?;
        self.write(&[0xE, 0])?;

        self.page(0)?;
        let mut buf = [0xFF; 0x18 + 1];
        buf[0] = 0x00;
        self.write(&buf)?;

        self.set_brightness(255)
    }

    fn update_pixel(&mut self, i: u8, pix: RGB8) -> Result<(), ()> {
        let row = i >> 4; // # i // 16
        let col = i & 15; // # i % 16
        self.page(1)?;
        self.write(&[row * 48 + col, pix.g])?;
        self.write(&[row * 48 + 16 + col, pix.r])?;
        self.write(&[row * 48 + 32 + col, pix.b])?;
        Ok(())
    }
}

fn defmt_key(row: usize, col: usize) {
    match (row, col) {
        (0, 0) => defmt::info!("ESC"),
        (0, 1) => defmt::info!("1"),
        (0, 2) => defmt::info!("2"),
        (0, 3) => defmt::info!("3"),
        (0, 4) => defmt::info!("4"),
        (0, 5) => defmt::info!("5"),
        (0, 6) => defmt::info!("6"),
        (0, 7) => defmt::info!("7"),
        (1, 0) => defmt::info!("8"),
        (1, 1) => defmt::info!("9"),
        (1, 2) => defmt::info!("0"),
        (1, 3) => defmt::info!("-"),
        (1, 4) => defmt::info!("="),
        (1, 5) => defmt::info!("BACKSPACE"),
        (1, 6) => defmt::info!("|"),
        (1, 7) => defmt::info!("]"),
        (2, 0) => defmt::info!("["),
        (2, 1) => defmt::info!("P"),
        (2, 2) => defmt::info!("O"),
        (2, 3) => defmt::info!("I"),
        (2, 4) => defmt::info!("U"),
        (2, 5) => defmt::info!("Y"),
        (2, 6) => defmt::info!("T"),
        (2, 7) => defmt::info!("R"),
        (3, 0) => defmt::info!("E"),
        (3, 1) => defmt::info!("W"),
        (3, 2) => defmt::info!("Q"),
        (3, 3) => defmt::info!("TAB"),
        (3, 4) => defmt::info!("CAPS"),
        (3, 5) => defmt::info!("A"),
        (3, 6) => defmt::info!("S"),
        (3, 7) => defmt::info!("D"),
        (4, 0) => defmt::info!("F"),
        (4, 1) => defmt::info!("G"),
        (4, 2) => defmt::info!("H"),
        (4, 3) => defmt::info!("J"),
        (4, 4) => defmt::info!("K"),
        (4, 5) => defmt::info!("L"),
        (4, 6) => defmt::info!(";"),
        (4, 7) => defmt::info!("\""),
        (5, 0) => defmt::info!("ENTER"),
        (5, 1) => defmt::info!("RSHIFT"),
        (5, 2) => defmt::info!("/"),
        (5, 3) => defmt::info!("."),
        (5, 4) => defmt::info!(","),
        (5, 5) => defmt::info!("M"),
        (5, 6) => defmt::info!("N"),
        (5, 7) => defmt::info!("B"),
        (6, 0) => defmt::info!("V"),
        (6, 1) => defmt::info!("C"),
        (6, 2) => defmt::info!("X"),
        (6, 3) => defmt::info!("Z"),
        (6, 4) => defmt::info!("LSHIFT"),
        (6, 5) => defmt::info!("LCTRL"),
        (6, 6) => defmt::info!("LGUI"),
        (6, 7) => defmt::info!("LALT"),
        (7, 0) => defmt::info!("SPACE"),
        (7, 1) => defmt::info!("RALT"),
        (7, 2) => defmt::info!("MENU"),
        (7, 3) => defmt::info!("L1"),
        (7, 4) => defmt::info!("RCTRL"),
        _ => defmt::error!("?????"),
    }
}

fn char_key(row: usize, col: usize) -> Option<char> {
    match (row, col) {
        (0, 1) => Some('1'),
        (0, 2) => Some('2'),
        (0, 3) => Some('3'),
        (0, 4) => Some('4'),
        (0, 5) => Some('5'),
        (0, 6) => Some('6'),
        (0, 7) => Some('7'),
        (1, 0) => Some('8'),
        (1, 1) => Some('9'),
        (1, 2) => Some('0'),
        (1, 3) => Some('-'),
        (1, 4) => Some('='),
        (1, 6) => Some('|'),
        (1, 7) => Some(']'),
        (2, 0) => Some('['),
        (2, 1) => Some('P'),
        (2, 2) => Some('O'),
        (2, 3) => Some('I'),
        (2, 4) => Some('U'),
        (2, 5) => Some('Y'),
        (2, 6) => Some('T'),
        (2, 7) => Some('R'),
        (3, 0) => Some('E'),
        (3, 1) => Some('W'),
        (3, 2) => Some('Q'),
        (3, 5) => Some('A'),
        (3, 6) => Some('S'),
        (3, 7) => Some('D'),
        (4, 0) => Some('F'),
        (4, 1) => Some('G'),
        (4, 2) => Some('H'),
        (4, 3) => Some('J'),
        (4, 4) => Some('K'),
        (4, 5) => Some('L'),
        (4, 6) => Some(';'),
        (4, 7) => Some('\"'),
        (5, 2) => Some('/'),
        (5, 3) => Some('.'),
        (5, 4) => Some(','),
        (5, 5) => Some('M'),
        (5, 6) => Some('N'),
        (5, 7) => Some('B'),
        (6, 0) => Some('V'),
        (6, 1) => Some('C'),
        (6, 2) => Some('X'),
        (6, 3) => Some('Z'),
        (7, 0) => Some(' '),
        _ => None,
    }
}
