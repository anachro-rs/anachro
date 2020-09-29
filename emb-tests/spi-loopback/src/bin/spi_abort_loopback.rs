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
use spi_loopback as _; // global logger + panicking-behavior + memory layout
use bbqueue::{
    consts::*,
    BBBuffer,
    ConstBBBuffer,
    framed::FrameGrantW,
};

// Controller    Peripheral
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

static BB_OUT: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );
static BB_INC: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );
static BB_TRE: BBBuffer<U1024> = BBBuffer( ConstBBBuffer::new() );

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let board = Peripherals::take().unwrap();
    let mut p0 = P0Parts::new(board.P0);
    let mut p1 = P1Parts::new(board.P1);

    let spim0_pins = SpimPins {
        sck: p0.p0_03.into_push_pull_output(Level::Low).degrade(),
        miso: Some(p0.p0_04.into_floating_input().degrade()),
        mosi: Some(p0.p0_28.into_push_pull_output(Level::Low).degrade()),
    };
    let mut spim0_sck = p0.p0_29.into_push_pull_output(Level::High).degrade();

    let spis1_pins = SpisPins {
        sck: p1.p1_05.into_floating_input().degrade(),
        cipo: Some(p1.p1_04.into_floating_input().degrade()),
        copi: Some(p1.p1_03.into_floating_input().degrade()),
        cs: p1.p1_02.into_floating_input().degrade(),
    };

    let mut spim = SpimState::Periph(Spim::new(board.SPIM0, spim0_pins, Frequency::K125, MODE_0, 0x00));
    let mut spis = SpisState::Periph(Spis::new(board.SPIS1, spis1_pins));

    let (mut out_prod, mut out_cons) = BB_OUT.try_split_framed().unwrap();
    let (mut inc_prod, mut inc_cons) = BB_INC.try_split_framed().unwrap();
    let (mut tre_prod, mut tre_cons) = BB_TRE.try_split_framed().unwrap();

    let mut timer = Timer::new(board.TIMER0);

    let mut spim_ctr: u8 = 0;
    let mut spis_ctr: u8 = 128;

    loop {
        let mut spim_inner = SpimState::Unstable;
        let mut spis_inner = SpisState::Unstable;

        core::mem::swap(&mut spis_inner, &mut spis);
        core::mem::swap(&mut spim_inner, &mut spim);

        defmt::info!("Loop!");

        let mut ogr = out_prod.grant(32).unwrap();
        let mut igr = inc_prod.grant(32).unwrap();
        let mut tgr = tre_prod.grant(32).unwrap();

        for o in ogr.iter_mut() {
            *o = spim_ctr;
            spim_ctr = spim_ctr.wrapping_add(1);
        }

        for i in igr.iter_mut() {
            *i = spis_ctr;
            spis_ctr = spis_ctr.wrapping_add(1);
        }

        if let SpisState::Periph(spis_m) = spis_inner {
            spis_inner = SpisState::Transfer(spis_m.transfer(igr).map_err(drop).unwrap());
        } else {
            spi_loopback::exit();
        }

        if let SpimState::Periph(spim_m) = spim_inner {
            spim0_sck.set_low().ok();
            spim_inner = SpimState::Transfer(spim_m.dma_transfer_split(tgr, ogr).map_err(drop).unwrap());
        }

        defmt::info!("Waiting 1ms to abort (~1/2 way)...");
        timer.delay_ms(1u8);
        defmt::info!("Bailing!");

        let buf = if let SpisState::Transfer(mut xfrs) = spis_inner {
            let (buf, p) = xfrs.bail();
            defmt::info!("made it {:?} bytes", p.amount());
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            defmt::info!("spis immediate: {:?}", &buf[..]);
            p.enable();
            spis = SpisState::Periph(p);
            buf
        } else {
            spi_loopback::exit();
        };

        loop {
            if let SpimState::Transfer(mut xfrm) = spim_inner {
                if xfrm.is_done() {
                    let (tx, rx, p) = xfrm.wait();
                    spim = SpimState::Periph(p);
                    spim0_sck.set_high().ok();
                    defmt::info!("got {:?}", &rx[..]);
                    break;
                } else {
                    spim_inner = SpimState::Transfer(xfrm);
                }
            }
        }

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        defmt::info!("spis delayed: {:?}", &buf[..]);

        defmt::info!("All finished!");

        timer.delay_ms(1000u32);
    }

    // spi_loopback::exit()
}

enum SpisState {
    Periph(Spis<SPIS1>),
    Transfer(Transfer<SPIS1, FrameGrantW<'static, U1024>>),
    Unstable,
}

enum SpimState {
    Periph(Spim<SPIM0>),
    Transfer(TransferSplit<SPIM0, FrameGrantW<'static, U1024>, FrameGrantW<'static, U1024>>),
    Unstable,
}
