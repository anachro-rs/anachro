use std::net::TcpStream;

use std::default::Default;
use std::io::{ErrorKind, Read, Write};

use serde::{Serialize, Deserialize};

use postcard::{to_stdvec_cobs, from_bytes_cobs};

use crate::{
    Result, Error,
    component::EncLogicLLComponent,
    arbitrator::EncLogicLLArbitrator,
};

#[derive(Debug, Serialize, Deserialize)]
enum TcpSpiMsg {
    ReadyState(bool),
    GoState(bool),
    Payload(Vec<u8>),
}

pub struct TcpSpiComLL {
    stream: TcpStream,
    go_state: Option<bool>,
    ready_state: bool,
    incoming_payload: Option<Vec<u8>>,
    pending_data: Vec<u8>,
    pending_exchange: Option<PendingExchange>,
}

struct PendingExchange {
    data_out: *const u8,
    data_out_len: usize,
    data_in: *mut u8,
    data_in_max: usize,
}

impl TcpSpiComLL {
    pub fn new(mut stream: TcpStream) -> Self {
        let init_msg = to_stdvec_cobs(&TcpSpiMsg::ReadyState(false)).unwrap();

        // Send init message declaring GO state
        stream.write_all(&init_msg).unwrap();

        TcpSpiComLL {
            stream,
            go_state: None,
            ready_state: false,
            incoming_payload: None,
            pending_data: Vec::default(),
            pending_exchange: None,
        }
    }
}

impl EncLogicLLComponent for TcpSpiComLL {

    fn process(&mut self) -> Result<()> {
        let mut buf = [0u8; 1024];

        // Receive incoming messages
        loop {
            match self.stream.read(&mut buf) {
                Ok(n) if n > 0 => {
                    self.pending_data.extend_from_slice(&buf[..n]);
                }
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("TCP Error: {:?}", e);
                    panic!()
                }
            }
        }

        // Process any messages
        while let Some(p) = self.pending_data.iter().position(|c| *c == 0x00) {
            let mut remainder = self.pending_data.split_off(p + 1);
            core::mem::swap(&mut remainder, &mut self.pending_data);
            let mut payload = remainder;

            // println!("TCP: got {:?}", payload);

            if let Ok(msg) = from_bytes_cobs::<TcpSpiMsg>(&mut payload) {
                match msg {
                    TcpSpiMsg::GoState(state) => {
                        // println!("Go is now: {}", state);
                        self.go_state = Some(state);
                    }
                    TcpSpiMsg::ReadyState(_) => {
                        panic!("We're a component! No one should be sending us ready state!");
                    }
                    TcpSpiMsg::Payload(payload) => {
                        // println!("Payload!");
                        assert!(self.incoming_payload.is_none(), "DATA LOSS");
                        self.incoming_payload = Some(payload);
                    }
                }
            }
        }

        Ok(())
    }

    fn is_ready_active(&mut self) -> Result<bool> {
        Ok(self.ready_state)
    }

    fn is_go_active(&mut self) -> Result<bool> {
        self.go_state.ok_or(Error::ToDo)
    }

    /// Set the READY line low (active)
    fn notify_ready(&mut self) -> Result<()> {
        self.ready_state = true;
        let msg = TcpSpiMsg::ReadyState(true);
        let payload = to_stdvec_cobs(&msg).map_err(|_| Error::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| Error::ToDo)?;
        Ok(())
    }

    /// Set the READY line high (inactive)
    fn clear_ready(&mut self) -> Result<()> {
        self.ready_state = false;
        let msg = TcpSpiMsg::ReadyState(false);
        let payload = to_stdvec_cobs(&msg).map_err(|_| Error::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| Error::ToDo)?;
        Ok(())
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
        if self.pending_exchange.is_some() {
            return Err(Error::ToDo);
        }
        self.pending_exchange = Some(PendingExchange {
            data_out,
            data_out_len,
            data_in,
            data_in_max,
        });
        self.notify_ready()?;
        Ok(())
    }

    /// Actually begin exchanging data
    ///
    /// Will return an error if READY and GO are not active
    fn trigger_exchange(&mut self) -> Result<()> {
        if !(self.is_go_active()? && self.ready_state) {
            return Err(Error::ToDo);
        }

        let exch = match self.pending_exchange.as_ref() {
            Some(ex) => ex,
            None => return Err(Error::ToDo),
        };

        // Hello! I am pretending to be DMA!
        let payload =
            unsafe { core::slice::from_raw_parts(exch.data_out, exch.data_out_len) }.to_vec();

        let msg = to_stdvec_cobs(&TcpSpiMsg::Payload(payload)).map_err(|_| Error::ToDo)?;
        self.stream.write_all(&msg).map_err(|_| Error::ToDo)?;

        Ok(())
    }

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> Result<bool> {
        Ok(self.pending_exchange.is_some())
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
        let is_go_active = self.is_go_active()?;

        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(Error::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None if !is_go_active => {
                // println!("No go!");
                return Err(Error::ArbitratorHungUp);
            }
            None => {
                self.pending_exchange = Some(exch);
                // Still busy
                return Err(Error::ToDo);
            }
            Some(inc) => inc,
        };

        // It's me, DMA!
        let out_slice = unsafe { core::slice::from_raw_parts_mut(exch.data_in, exch.data_in_max) };

        let copy_amt = exch.data_in_max.min(inc.len());

        out_slice[..copy_amt].copy_from_slice(&inc[..copy_amt]);

        if clear_ready {
            self.clear_ready()?;
        }

        if !is_go_active {
            Err(Error::ArbitratorHungUp)
        } else {
            Ok(copy_amt)
        }
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

        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(Error::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None => {
                return Err(Error::IncompleteTransaction(0));
            }
            Some(inc) => inc,
        };

        // It's me, DMA!
        let out_slice = unsafe { core::slice::from_raw_parts_mut(exch.data_in, exch.data_in_max) };

        let copy_amt = exch.data_in_max.min(inc.len());

        out_slice[..copy_amt].copy_from_slice(&inc[..copy_amt]);

        Ok(copy_amt)
    }
}

pub struct TcpSpiArbLL {
    stream: TcpStream,
    go_state: bool,
    ready_state: Option<bool>,
    incoming_payload: Option<Vec<u8>>,
    pending_data: Vec<u8>,
    pending_exchange: Option<PendingExchange>,
}

impl TcpSpiArbLL {
    pub fn new(mut stream: TcpStream) -> Self {
        let init_msg = to_stdvec_cobs(&TcpSpiMsg::GoState(false)).unwrap();

        // Send init message declaring GO state
        stream.write_all(&init_msg).unwrap();

        TcpSpiArbLL {
            stream,
            go_state: false,
            ready_state: None,
            incoming_payload: None,
            pending_data: Vec::default(),
            pending_exchange: None,
        }
    }
}

impl EncLogicLLArbitrator for TcpSpiArbLL {

    fn process(&mut self) -> Result<()> {
        let mut buf = [0u8; 1024];

        // Receive incoming messages
        loop {
            match self.stream.read(&mut buf) {
                Ok(n) if n > 0 => {
                    self.pending_data.extend_from_slice(&buf[..n]);
                }
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("TCP Error: {:?}", e);
                    panic!()
                }
            }
        }

        // Process any messages
        while let Some(p) = self.pending_data.iter().position(|c| *c == 0x00) {
            let mut remainder = self.pending_data.split_off(p + 1);
            core::mem::swap(&mut remainder, &mut self.pending_data);
            let mut payload = remainder;

            // println!("TCP: got {:?}", payload);

            if let Ok(msg) = from_bytes_cobs::<TcpSpiMsg>(&mut payload) {
                match msg {
                    TcpSpiMsg::ReadyState(state) => {
                        // println!("Ready is now: {}", state);
                        self.ready_state = Some(state);
                    }
                    TcpSpiMsg::GoState(_) => {
                        panic!("We're an arbitrator! No one should tell us Go state!");
                    }
                    TcpSpiMsg::Payload(payload) => {
                        // println!("Payload!");
                        assert!(self.incoming_payload.is_none(), "DATALOSS");
                        self.incoming_payload = Some(payload);
                    }
                }
            }
        }

        Ok(())
    }

    fn is_ready_active(&mut self) -> Result<bool> {
        self.ready_state.ok_or(Error::ToDo)
    }

    fn notify_go(&mut self) -> Result<()> {
        self.go_state = true;
        let msg = TcpSpiMsg::GoState(true);
        let payload = to_stdvec_cobs(&msg).map_err(|_| Error::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| Error::ToDo)?;
        Ok(())
    }

    fn clear_go(&mut self) -> Result<()> {
        // println!("cleargo");
        self.go_state = false;
        let msg = TcpSpiMsg::GoState(false);
        let payload = to_stdvec_cobs(&msg).map_err(|_| Error::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| Error::ToDo)?;
        Ok(())
    }

    fn is_go_active(&mut self) -> Result<bool> {
        Ok(self.go_state)
    }

    fn prepare_exchange(
        &mut self,
        data_out: *const u8,
        data_out_len: usize,
        data_in: *mut u8,
        data_in_max: usize,
    ) -> Result<()> {
        if self.pending_exchange.is_some() {
            return Err(Error::ToDo);
        }
        if !self.is_ready_active()? {
            return Err(Error::ToDo);
        }

        self.pending_exchange = Some(PendingExchange {
            data_out,
            data_out_len,
            data_in,
            data_in_max,
        });

        self.notify_go()?;

        Ok(())
    }

    fn is_exchange_active(&self) -> Result<bool> {
        Ok(self.pending_exchange.is_some())
    }

    fn complete_exchange(&mut self, clear_go: bool) -> Result<usize> {
        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(Error::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None => {
                self.pending_exchange = Some(exch);
                return Err(Error::IncompleteTransaction(0));
            }
            Some(inc) => inc,
        };

        // So, this is actually responding. This is to prevent
        // sending messages if the client hangs up instead of
        // sending data. Would typically be done automatically
        // by a SPI peripheral as the client clocks out data

        // Hello! I am pretending to be DMA!
        let payload =
            unsafe { core::slice::from_raw_parts(exch.data_out, exch.data_out_len) }.to_vec();

        let msg = to_stdvec_cobs(&TcpSpiMsg::Payload(payload)).map_err(|_| Error::ToDo)?;
        self.stream.write_all(&msg).map_err(|_| Error::ToDo)?;

        // It's me, DMA!
        let out_slice = unsafe { core::slice::from_raw_parts_mut(exch.data_in, exch.data_in_max) };

        let copy_amt = exch.data_in_max.min(inc.len());

        out_slice[..copy_amt].copy_from_slice(&inc[..copy_amt]);

        if clear_go {
            self.clear_go()?;
        }

        Ok(copy_amt)
    }

    fn abort_exchange(&mut self) -> Result<usize> {
        self.clear_go().ok();

        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(Error::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None => {
                return Err(Error::IncompleteTransaction(0));
            }
            Some(inc) => inc,
        };

        // It's me, DMA!
        let out_slice = unsafe { core::slice::from_raw_parts_mut(exch.data_in, exch.data_in_max) };

        let copy_amt = exch.data_in_max.min(inc.len());

        out_slice[..copy_amt].copy_from_slice(&inc[..copy_amt]);

        Ok(copy_amt)
    }
}
