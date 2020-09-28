use nrf52840_hal::{
    spis::{
        Spis,
        Instance,
        Mode,
        TransferSplit,
    },
    gpio::{
        Pin,
        Output,
        Input,
        Floating,
        PushPull,
    }
};

use embedded_hal::digital::v2::{
    StatefulOutputPin,
    OutputPin,
    InputPin,
};

use embedded_dma::{ReadBuffer, WriteBuffer};

use anachro_spi::{
    Result, Error,
    arbitrator::EncLogicLLArbitrator,
};

unsafe impl<S> Send for Periph<S>
where
    S: Instance + Send,
{ }

enum Periph<S>
where
    S: Instance + Send,
{
    Idle(Spis<S>),
    Pending(TransferSplit<S, ConstRawSlice, MutRawSlice>),
    Aborted(TransferSplit<S, ConstRawSlice, MutRawSlice>),
    Unstable,
}

pub struct NrfSpiArbLL<S>
where
    S: Instance + Send,
{
    periph: Periph<S>,
    ready_pin: Pin<Input<Floating>>,
    go_pin: Pin<Output<PushPull>>,
}

impl<S> NrfSpiArbLL<S>
where
    S: Instance + Send,
{
    pub fn new(spis: Spis<S>, ready_pin: Pin<Input<Floating>>, mut go_pin: Pin<Output<PushPull>>) -> Self {
        go_pin.set_high().ok();
        spis.set_default_char(0x00)
            .set_orc(0x00)
            .set_mode(Mode::Mode0)
            .auto_acquire(true)
            .reset_events();
        spis.try_acquire().ok();
        spis.disable();

        Self {
            periph: Periph::Idle(spis),
            go_pin,
            ready_pin,
        }
    }
}


impl<S> EncLogicLLArbitrator for NrfSpiArbLL<S>
where
    S: Instance + Send,
{

    fn process(&mut self) -> Result<()> {
        let mut current = Periph::Unstable;
        core::mem::swap(&mut current, &mut self.periph);
        self.periph = match current {
            Periph::Idle(p) => Periph::Idle(p),
            Periph::Pending(p) => Periph::Pending(p),
            Periph::Aborted(mut p) => {
                if p.is_done() {
                    let (_tx, _rx, p) = p.wait();
                    Periph::Idle(p)
                } else {
                    Periph::Aborted(p)
                }
            }
            Periph::Unstable => return Err(Error::ToDo),
        };

        Ok(())
    }

    fn is_ready_active(&mut self) -> Result<bool> {
        self.ready_pin.is_low().map_err(|_| Error::ToDo)
    }

    fn notify_go(&mut self) -> Result<()> {
        self.go_pin.set_low().map_err(|_| Error::ToDo)
    }

    fn clear_go(&mut self) -> Result<()> {
        self.go_pin.set_high().map_err(|_| Error::ToDo)
    }

    fn is_go_active(&mut self) -> Result<bool> {
        self.go_pin.is_set_low().map_err(|_| Error::ToDo)
    }

    fn prepare_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()> {
        match self.is_ready_active() {
            Ok(true) => {},
            _ => return Err(Error::ToDo),
        }


        let mut old_periph = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut old_periph);

        let spis = match old_periph {
            Periph::Idle(spis) => spis,
            Periph::Pending(_) | Periph::Aborted(_) | Periph::Unstable => {
                self.periph = old_periph;
                return Err(Error::ToDo)
            }
        };

        let _spis_ref = match spis.try_acquire() {
            Ok(sr) => sr,
            Err(_e) => {
                self.periph = Periph::Idle(spis);
                return Err(Error::ToDo);
            }
        };

        spis.enable();
        let txfr = match spis.transfer_split(
            ConstRawSlice { ptr: data_out, len: data_out_len },
            MutRawSlice { ptr: data_in, len: data_in_max },
        ) {
            Ok(t) => t,
            Err((_e, p, _tx, _rx)) => {
                self.periph = Periph::Idle(p);
                return Err(Error::ToDo);
            }
        };

        self.periph = Periph::Pending(txfr);
        self.notify_go().ok();


        Ok(())
    }

    fn is_exchange_active(&self) -> Result<bool> {
        match self.periph {
            Periph::Idle(_) => Ok(false),
            Periph::Pending(_) => Ok(true),
            Periph::Aborted(_) => Ok(true), // maybe?
            Periph::Unstable => Err(Error::ToDo),
        }
    }

    fn complete_exchange(&mut self, clear_go: bool) -> Result<usize> {
        let mut current = Periph::Unstable;
        core::mem::swap(&mut self.periph, &mut current);

        let amt = match current {
            Periph::Idle(p) => {
                self.periph = Periph::Idle(p);
                return Err(Error::ToDo);
            }
            Periph::Aborted(p) => {
                self.periph = Periph::Aborted(p);
                return Err(Error::ToDo);
            }
            Periph::Unstable => {
                return Err(Error::ToDo);
            }
            Periph::Pending(mut p) => {
                if p.is_done() {
                    let (_tx, _rx, p) = p.wait();
                    let amt = p.amount() as usize;
                    p.disable();
                    self.periph = Periph::Idle(p);
                    amt
                } else {
                    self.periph = Periph::Pending(p);
                    return Err(Error::ToDo);
                }
            }
        };

        if clear_go {
            self.clear_go()?;
        }

        Ok(amt)
    }

    fn abort_exchange(&mut self) -> Result<usize> {
        self.clear_go().ok();

        let mut current = Periph::Unstable;
        core::mem::swap(&mut current, &mut self.periph);

        let mut amt = 0;
        self.periph = match current {
            Periph::Idle(p) => Periph::Idle(p),
            Periph::Pending(mut p) => {
                if p.is_done() {
                    let (_tx, _rx, p) = p.wait();
                    amt = p.amount() as usize;
                    p.disable();
                    Periph::Idle(p)
                } else {
                    Periph::Aborted(p)
                }
            },
            Periph::Aborted(p) => Periph::Aborted(p),
            Periph::Unstable => Periph::Unstable,
        };

        Ok(amt)
    }
}

struct ConstRawSlice {
    ptr: *const u8,
    len: usize,
}

struct MutRawSlice {
    ptr: *mut u8,
    len: usize,
}

unsafe impl WriteBuffer for MutRawSlice {
    type Word = u8;

    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize) {
        (self.ptr, self.len)
    }
}

unsafe impl ReadBuffer for ConstRawSlice {
    type Word = u8;

    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.ptr, self.len)
    }
}
