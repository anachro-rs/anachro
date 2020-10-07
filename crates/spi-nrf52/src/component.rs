use nrf52840_hal::{
    gpio::{Floating, Input, Output, Pin, PushPull},
    spim::{Instance, Spim, TransferSplit},
};

use embedded_hal::digital::v2::{InputPin, OutputPin};

use embedded_dma::{ReadBuffer, WriteBuffer};

use crate::{ConstRawSlice, MutRawSlice};
use anachro_spi::{component::EncLogicLLComponent, Error, Result};

pub struct NrfSpiComLL<S>
where
    S: Instance + Send,
{
    periph: Periph<S>,
    csn_pin: Pin<Output<PushPull>>,
    go_pin: Pin<Input<Floating>>,
}

unsafe impl<S> Send for Periph<S> where S: Instance + Send {}

enum Periph<S>
where
    S: Instance + Send,
{
    Idle(Spim<S>),
    Pending(TransferSplit<S, ConstRawSlice, MutRawSlice>),
    Unstable,
}

impl<S> NrfSpiComLL<S>
where
    S: Instance + Send,
{
    pub fn new(
        spim: Spim<S>,
        mut csn_pin: Pin<Output<PushPull>>,
        go_pin: Pin<Input<Floating>>,
    ) -> Self {
        csn_pin.set_high().ok();
        Self {
            periph: Periph::Idle(spim),
            csn_pin,
            go_pin,
        }
    }
}

impl<S> EncLogicLLComponent for NrfSpiComLL<S>
where
    S: Instance + Send,
{
    /// Process low level messages
    fn process(&mut self) -> Result<()> {
        Ok(())
    }

    /// Set the READY line low (active)
    fn notify_csn(&mut self) -> Result<()> {
        defmt::info!("Component: Active CSN");
        self.csn_pin.set_low().map_err(|_| Error::GpioError)
    }

    /// Set the READY line high (inactive)
    fn clear_csn(&mut self) -> Result<()> {
        // defmt::info!("Component: Inactive CSN");
        self.csn_pin.set_high().map_err(|_| Error::GpioError)
    }

    /// Query whether the GO line is low (active)
    // TODO: just &self?
    fn is_go_active(&mut self) -> Result<bool> {
        self.go_pin.is_low().map_err(|_| Error::GpioError)
    }

    /// Prepare data to be exchanged. The data MUST not be referenced
    /// until `complete_exchange` or `abort_exchange` has been called.
    ///
    /// NOTE: Data will not be sent until `trigger_exchange` has been
    /// called. This will automatically set the READY line if it is
    /// not already active.
    ///
    /// An error will be returned if an exchange is already in progress
    // TODO: `embedded-dma`?
    fn begin_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()> {
        let mut old_periph = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut old_periph);

        let spim = match old_periph {
            Periph::Idle(spim) => spim,
            Periph::Pending(_) | Periph::Unstable => {
                self.periph = old_periph;
                panic!("Incorrect state");
                return Err(Error::IncorrectState);
            }
        };

        let crs = ConstRawSlice {
            ptr: data_out,
            len: data_out_len,
        };

        let mrs = MutRawSlice {
            ptr: data_in,
            len: data_in_max,
        };

        defmt::info!("Triggering exchange");
        defmt::trace!("tx_len: {:?}, rx_len: {:?}", crs.len, mrs.len);

        let txfr = match spim.dma_transfer_split(crs, mrs) {
            Ok(t) => t,
            Err((p, _e)) => {
                defmt::error!("Error in triggering exchange!");
                self.periph = Periph::Idle(p);
                panic!("Spi Error");
                return Err(Error::SpiError);
            }
        };

        defmt::trace!("triggered exchange");

        self.periph = Periph::Pending(txfr);

        Ok(())
    }

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool> {
        match self.periph {
            Periph::Idle(_) => Ok(false),
            Periph::Pending(_) => Ok(true),
            Periph::Unstable => Err(Error::UnstableFailure),
        }
    }

    /// Attempt to complete a `exchange` action.
    ///
    /// Returns `Ok(())` if the `exchange` completed successfully.
    ///
    /// Will return an error if the exchange is still in progress.
    ///
    /// Use `abort_exchange` to force the exchange to completion even
    /// if it is still in progress.
    fn complete_exchange(&mut self) -> Result<usize> {
        let mut current = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut current);

        let amt = match current {
            Periph::Idle(p) => {
                self.periph = Periph::Idle(p);
                panic!("Incorrect State");
                return Err(Error::IncorrectState);
            }
            Periph::Unstable => {
                panic!("Unstable");
                return Err(Error::UnstableFailure);
            }
            Periph::Pending(mut p) => {
                if p.is_done() {
                    let (tx, mut rx, p) = p.wait();
                    let amt_rx = unsafe { rx.write_buffer().1 };
                    let amt_tx = unsafe { tx.read_buffer().1 };
                    defmt::info!("Completed exchange: rx: {:?}, tx: {:?}", amt_rx, amt_tx);
                    self.periph = Periph::Idle(p);
                    amt_rx
                } else {
                    self.periph = Periph::Pending(p);
                    return Err(Error::TransactionBusy);
                }
            }
        };

        Ok(amt)
    }

    /// Stop the `exchange` action immediately
    ///
    /// Returns `Ok(())` if the exchange had already been completed.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize> {
        let mut current = Periph::Unstable;
        core::mem::swap(&mut current, &mut self.periph);

        let mut amt = 0;
        self.periph = match current {
            Periph::Idle(p) => Periph::Idle(p),
            Periph::Pending(mut p) => {
                if p.is_done() {
                    let (_tx, mut rx, p) = p.wait();
                    amt = unsafe { rx.write_buffer().1 };
                    Periph::Idle(p)
                } else {
                    // TODO: Fast abort?
                    let (_tx, _rx, p) = p.wait();
                    Periph::Idle(p)
                }
            }
            Periph::Unstable => Periph::Unstable,
        };

        Ok(amt)
    }
}
