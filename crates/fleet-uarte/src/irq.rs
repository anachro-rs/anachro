use crate::hal::{
    gpio::Port,
    pac::{Interrupt, NVIC, UARTE0, UARTE1},
    ppi::{ConfigurablePpi, Ppi},
    target_constants::EASY_DMA_SIZE,
    timer::Instance as TimerInstance,
    uarte::{Baudrate, Instance as UarteInstance, Parity, Pins},
};
use bbqueue::{ArrayLength, Consumer, GrantR, GrantW, Producer};
use core::sync::atomic::{compiler_fence, AtomicBool, Ordering::SeqCst};
use embedded_hal::digital::v2::OutputPin;

pub struct UarteTimer<Timer>
where
    Timer: TimerInstance,
{
    pub(crate) timer: Timer,
    pub(crate) timeout_flag: &'static AtomicBool,
    pub(crate) interrupt: Interrupt,
}

impl<Timer> UarteTimer<Timer>
where
    Timer: TimerInstance,
{
    pub fn init(&mut self, microsecs: u32) {
        self.timer.disable_interrupt();
        self.timer.timer_cancel();
        self.timer.set_periodic();
        self.timer.set_shorts_periodic();
        self.timer.enable_interrupt();

        self.timer.timer_start(microsecs);
    }

    pub fn interrupt(&self) {
        // pend uarte interrupt
        // TODO: Don't hardcode UARTE0
        self.timer.timer_reset_event();
        self.timeout_flag.store(true, SeqCst);
        NVIC::pend(self.interrupt);
    }
}

pub struct UarteIrq<OutgoingLen, IncomingLen, Channel, Uarte>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    Channel: Ppi + ConfigurablePpi,
    Uarte: FleetUarteInstance,
{
    pub(crate) outgoing_cons: Consumer<'static, OutgoingLen>,
    pub(crate) incoming_prod: Producer<'static, IncomingLen>,
    pub(crate) timeout_flag: &'static AtomicBool,
    pub(crate) rx_grant: Option<GrantW<'static, IncomingLen>>,
    pub(crate) tx_grant: Option<GrantR<'static, OutgoingLen>>,
    pub(crate) uarte: Uarte,
    pub(crate) block_size: usize,
    pub(crate) ppi_ch: Channel,
}

impl<OutgoingLen, IncomingLen, Channel, Uarte> UarteIrq<OutgoingLen, IncomingLen, Channel, Uarte>
where
    OutgoingLen: ArrayLength<u8>,
    IncomingLen: ArrayLength<u8>,
    Channel: Ppi + ConfigurablePpi,
    Uarte: FleetUarteInstance,
{
    pub fn init(&mut self, pins: Pins, parity: Parity, baudrate: Baudrate) {
        uarte_setup(&self.uarte, pins, parity, baudrate);

        self.ppi_ch.enable();

        if let Ok(mut gr) = self.incoming_prod.grant_exact(self.block_size) {
            uarte_start_read(&self.uarte, &mut gr).unwrap();
            self.rx_grant = Some(gr);
        }
    }

    pub fn interrupt(&mut self) {
        let endrx = self.uarte.events_endrx.read().bits() != 0;
        let endtx = self.uarte.events_endtx.read().bits() != 0;
        let rxdrdy = self.uarte.events_rxdrdy.read().bits() != 0;
        let error = self.uarte.events_error.read().bits() != 0;
        let txstopped = self.uarte.events_txstopped.read().bits() != 0;

        let timeout = self.timeout_flag.swap(false, SeqCst);
        let errsrc = self.uarte.errorsrc.read().bits();

        // RX section
        if endrx || timeout || self.rx_grant.is_none() {
            // We only flush the connection if:
            //
            // * We didn't get a "natural" end of reception (full buffer), AND
            // * The timer expired, AND
            // * We have received one or more bytes to the receive buffer
            if !endrx && timeout && rxdrdy {
                uarte_cancel_read(&self.uarte);
            }

            compiler_fence(SeqCst);

            // Get the bytes received. If the rxdrdy flag wasn't set, then we haven't
            // actually received any bytes, and we can't trust the `amount` field
            // (it may have a stale value from the last reception)
            let amt = if rxdrdy {
                self.uarte.rxd.amount.read().bits() as usize
            } else {
                0
            };

            // If we received data, cycle the grant and get a new one
            if amt != 0 || self.rx_grant.is_none() {
                let gr = self.rx_grant.take();

                // If the buffer was full last time, we may not actually have a grant right now
                if let Some(gr) = gr {
                    gr.commit(amt);
                }

                // Attempt to get the next grant. If we don't get one now, no worries,
                // we'll try again on the next timeout
                if let Ok(mut gr) = self.incoming_prod.grant_exact(self.block_size) {
                    uarte_start_read(&self.uarte, &mut gr).unwrap();
                    self.rx_grant = Some(gr);
                }
            }
        }

        // TX Section
        if endtx || self.tx_grant.is_none() {
            if endtx {
                if let Some(gr) = self.tx_grant.take() {
                    let len = gr.len();
                    gr.release(len.min(EASY_DMA_SIZE));
                }
            }

            if let Ok(gr) = self.outgoing_cons.read() {
                let len = gr.len();
                uarte_start_write(&self.uarte, &gr[..len.min(EASY_DMA_SIZE)]).unwrap();
                self.tx_grant = Some(gr);
            }
        }

        // Clear events we processed
        if endrx {
            self.uarte.events_endrx.write(|w| w);
        }
        if endtx {
            self.uarte.events_endtx.write(|w| w);
        }
        if error {
            self.uarte.events_error.write(|w| w);
        }
        if rxdrdy {
            self.uarte.events_rxdrdy.write(|w| w);
        }
        if txstopped {
            self.uarte.events_txstopped.write(|w| w);
        }

        // Clear any errors
        if errsrc != 0 {
            self.uarte.errorsrc.write(|w| unsafe { w.bits(errsrc) });
        }
    }
}

/// Start a UARTE read transaction by setting the control
/// values and triggering a read task
fn uarte_start_read<T: FleetUarteInstance>(uarte: &T, rx_buffer: &mut [u8]) -> Result<(), ()> {
    // This is overly restrictive. See (similar SPIM issue):
    // https://github.com/nrf-rs/nrf52/issues/17
    if rx_buffer.len() > u8::max_value() as usize {
        return Err(());
    }

    // NOTE: RAM slice check is not necessary, as a mutable slice can only be
    // built from data located in RAM

    // Conservative compiler fence to prevent optimizations that do not
    // take in to account actions by DMA. The fence has been placed here,
    // before any DMA action has started
    compiler_fence(SeqCst);

    // Set up the DMA read
    uarte.rxd.ptr.write(|w|
        // We're giving the register a pointer to the stack. Since we're
        // waiting for the UARTE transaction to end before this stack pointer
        // becomes invalid, there's nothing wrong here.
        //
        // The PTR field is a full 32 bits wide and accepts the full range
        // of values.
        unsafe { w.ptr().bits(rx_buffer.as_ptr() as u32) });
    uarte.rxd.maxcnt.write(|w|
        // We're giving it the length of the buffer, so no danger of
        // accessing invalid memory. We have verified that the length of the
        // buffer fits in an `u8`, so the cast to `u8` is also fine.
        //
        // The MAXCNT field is at least 8 bits wide and accepts the full
        // range of values.
        unsafe { w.maxcnt().bits(rx_buffer.len() as _) });

    // Start UARTE Receive transaction
    uarte.tasks_startrx.write(|w|
        // `1` is a valid value to write to task registers.
        unsafe { w.bits(1) });

    Ok(())
}

/// Stop an unfinished UART read transaction and flush FIFO to DMA buffer
fn uarte_cancel_read<T: FleetUarteInstance>(uarte: &T) {
    uarte.events_rxto.write(|w| w);

    // Stop reception
    uarte.tasks_stoprx.write(|w| unsafe { w.bits(1) });

    // Wait for the reception to have stopped
    while uarte.events_rxto.read().bits() == 0 {}

    // Reset the event flag
    uarte.events_rxto.write(|w| w);

    // Ask UART to flush FIFO to DMA buffer
    uarte.tasks_flushrx.write(|w| unsafe { w.bits(1) });

    // Wait for the flush to complete.
    while uarte.events_endrx.read().bits() == 0 {}

    // The event flag itself is later reset by `finalize_read`.
}

fn uarte_setup<T: FleetUarteInstance>(
    uarte: &T,
    mut pins: Pins,
    parity: Parity,
    baudrate: Baudrate,
) {
    // Select pins
    uarte.psel.rxd.write(|w| {
        let w = unsafe { w.pin().bits(pins.rxd.pin()) };
        #[cfg(feature = "52840")]
        let w = w.port().bit(port_bit(&pins.rxd.port()));
        w.connect().connected()
    });
    pins.txd.set_high().unwrap();
    uarte.psel.txd.write(|w| {
        let w = unsafe { w.pin().bits(pins.txd.pin()) };
        #[cfg(feature = "52840")]
        let w = w.port().bit(port_bit(&pins.txd.port()));
        w.connect().connected()
    });

    // Optional pins
    uarte.psel.cts.write(|w| {
        if let Some(ref pin) = pins.cts {
            let w = unsafe { w.pin().bits(pin.pin()) };
            #[cfg(feature = "52840")]
            let w = w.port().bit(port_bit(&pin.port()));
            w.connect().connected()
        } else {
            w.connect().disconnected()
        }
    });

    uarte.psel.rts.write(|w| {
        if let Some(ref pin) = pins.rts {
            let w = unsafe { w.pin().bits(pin.pin()) };
            #[cfg(feature = "52840")]
            let w = w.port().bit(port_bit(&pin.port()));
            w.connect().connected()
        } else {
            w.connect().disconnected()
        }
    });

    // Enable UARTE instance
    uarte.enable.write(|w| w.enable().enabled());

    // Configure
    let hardware_flow_control = pins.rts.is_some() && pins.cts.is_some();
    uarte
        .config
        .write(|w| w.hwfc().bit(hardware_flow_control).parity().variant(parity));

    // Configure frequency
    uarte.baudrate.write(|w| w.baudrate().variant(baudrate));

    // Clear all interrupts
    uarte.intenclr.write(|w| unsafe { w.bits(0xFFFFFFFF) });

    // Enable relevant interrupts
    uarte.intenset.write(|w| {
        w.endrx().set_bit();
        w.endtx().set_bit();
        w.error().set_bit();
        w
    });
}

fn uarte_start_write<T: FleetUarteInstance>(uarte: &T, tx_buffer: &[u8]) -> Result<(), ()> {
    if tx_buffer.len() > EASY_DMA_SIZE {
        return Err(());
    }

    // Conservative compiler fence to prevent optimizations that do not
    // take in to account actions by DMA. The fence has been placed here,
    // before any DMA action has started
    compiler_fence(SeqCst);

    // Reset the events.
    uarte.events_endtx.reset();
    uarte.events_txstopped.reset();

    // Set up the DMA write
    uarte.txd.ptr.write(|w|
        // We're giving the register a pointer to the stack. Since we're
        // waiting for the UARTE transaction to end before this stack pointer
        // becomes invalid, there's nothing wrong here.
        //
        // The PTR field is a full 32 bits wide and accepts the full range
        // of values.
        unsafe { w.ptr().bits(tx_buffer.as_ptr() as u32) });
    uarte.txd.maxcnt.write(|w|
        // We're giving it the length of the buffer, so no danger of
        // accessing invalid memory. We have verified that the length of the
        // buffer fits in an `u8`, so the cast to `u8` is also fine.
        //
        // The MAXCNT field is 8 bits wide and accepts the full range of
        // values.
        unsafe { w.maxcnt().bits(tx_buffer.len() as _) });

    // Start UARTE Transmit transaction
    uarte.tasks_starttx.write(|w|
        // `1` is a valid value to write to task registers.
        unsafe { w.bits(1) });

    Ok(())
}

#[allow(dead_code)]
fn port_bit(p: &Port) -> bool {
    match p {
        Port::Port0 => false,
        #[cfg(feature = "52840")]
        Port::Port1 => true,
    }
}

pub trait FleetUarteInstance: UarteInstance {
    const INTERRUPT: Interrupt;
}

impl FleetUarteInstance for UARTE0 {
    const INTERRUPT: Interrupt = Interrupt::UARTE0_UART0;
}

impl FleetUarteInstance for UARTE1 {
    const INTERRUPT: Interrupt = Interrupt::UARTE1;
}
