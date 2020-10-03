use nrf52840_hal::{
    gpio::{Floating, Input, Output, Pin, PushPull},
    spim::{Instance, Spim, TransferSplit},
};

use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin};

use embedded_dma::WriteBuffer;

use crate::{ConstRawSlice, MutRawSlice};
use anachro_spi::{component::EncLogicLLComponent, Error, Result};

pub struct NrfSpiComLL<S>
where
    S: Instance + Send,
{
    periph: Periph<S>,
    ready_pin: Pin<Output<PushPull>>,
    go_pin: Pin<Input<Floating>>,
}

unsafe impl<S> Send for Periph<S> where S: Instance + Send {}

enum Periph<S>
where
    S: Instance + Send,
{
    Idle(Spim<S>),
    Awaiting((Spim<S>, ConstRawSlice, MutRawSlice)),
    Pending(TransferSplit<S, ConstRawSlice, MutRawSlice>),
    Unstable,
}

impl<S> NrfSpiComLL<S>
where
    S: Instance + Send,
{
    pub fn new(
        spim: Spim<S>,
        mut ready_pin: Pin<Output<PushPull>>,
        go_pin: Pin<Input<Floating>>,
    ) -> Self {
        ready_pin.set_high().ok();
        Self {
            periph: Periph::Idle(spim),
            ready_pin,
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

    /// Is the Component requesting a transaction?
    fn is_ready_active(&mut self) -> Result<bool> {
        self.ready_pin.is_set_low().map_err(|_| Error::GpioError)
    }

    /// Set the READY line low (active)
    fn notify_ready(&mut self) -> Result<()> {
        self.ready_pin.set_low().map_err(|_| Error::GpioError)
    }

    /// Set the READY line high (inactive)
    fn clear_ready(&mut self) -> Result<()> {
        self.ready_pin.set_high().map_err(|_| Error::GpioError)
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
    fn prepare_exchange(
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
            Periph::Pending(_) | Periph::Awaiting(_) | Periph::Unstable => {
                self.periph = old_periph;
                return Err(Error::IncorrectState);
            }
        };

        defmt::trace!("preparing exchange, {:?} bytes out", data_out_len);

        self.periph = Periph::Awaiting((
            spim,
            ConstRawSlice {
                ptr: data_out,
                len: data_out_len,
            },
            MutRawSlice {
                ptr: data_in,
                len: data_in_max,
            },
        ));
        self.notify_ready()?;

        Ok(())
    }

    /// Actually begin exchanging data
    ///
    /// Will return an error if READY and GO are not active
    fn trigger_exchange(&mut self) -> Result<()> {
        let mut old_periph = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut old_periph);

        let (spim, crs, mrs) = if let Periph::Awaiting((p, crs, mrs)) = old_periph {
            (p, crs, mrs)
        } else {
            core::mem::swap(&mut self.periph, &mut old_periph);
            return Err(Error::IncorrectState);
        };

        defmt::info!("Triggering exchange");
        defmt::trace!("tx_len: {:?}, rx_len: {:?}", crs.len, mrs.len);

        let txfr = match spim.dma_transfer_split(crs, mrs) {
            Ok(t) => t,
            Err((p, _e)) => {
                self.periph = Periph::Idle(p);
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
            Periph::Awaiting(_) => Ok(true),
            Periph::Pending(_) => Ok(true),
            Periph::Unstable => Err(Error::UnstableFailure),
        }
    }

    /// Attempt to complete a `exchange` action.
    ///
    /// Returns `Ok(())` if the `exchange` completed successfully.
    ///
    /// If the exchange is successful and `clear_ready` is `true`,
    /// then the READY line will be cleared.
    ///
    /// Will return an error if the exchange is still in progress.
    /// If the exchange is still in progress, `clear_ready` is ignored.
    ///
    /// Use `abort_exchange` to force the exchange to completion even
    /// if it is still in progress.
    fn complete_exchange(&mut self, clear_ready: bool) -> Result<usize> {
        let mut current = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut current);

        let amt = match current {
            Periph::Idle(p) => {
                self.periph = Periph::Idle(p);
                return Err(Error::IncorrectState);
            }
            Periph::Awaiting((p, crs, mrs)) => {
                self.periph = Periph::Awaiting((p, crs, mrs));
                return Err(Error::TransactionBusy);
            }
            Periph::Unstable => {
                return Err(Error::UnstableFailure);
            }
            Periph::Pending(mut p) => {
                if p.is_done() {
                    let (_tx, mut rx, p) = p.wait();
                    let amt = unsafe { rx.write_buffer().1 };
                    self.periph = Periph::Idle(p);
                    amt
                } else {
                    self.periph = Periph::Pending(p);
                    return Err(Error::TransactionBusy);
                }
            }
        };

        if clear_ready {
            self.clear_ready()?;
        }

        Ok(amt)
    }

    /// Stop the `exchange` action immediately
    ///
    /// Returns `Ok(())` if the exchange had already been completed.
    ///
    /// In all cases, the READY line will be cleared.
    ///
    /// If the exchange had not yet completed, an Error containing the
    /// number of successfully sent bytes will be returned.
    fn abort_exchange(&mut self) -> Result<usize> {
        self.clear_ready().ok();

        let mut current = Periph::Unstable;
        core::mem::swap(&mut current, &mut self.periph);

        let mut amt = 0;
        self.periph = match current {
            Periph::Idle(p) => Periph::Idle(p),
            Periph::Awaiting((p, _r, _t)) => Periph::Idle(p),
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
