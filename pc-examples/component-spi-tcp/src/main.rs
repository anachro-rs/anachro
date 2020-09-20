#![allow(unused_imports)]

use anachro_spi::{
    self as spi,
    Error as SpiError,
    Result as SpiResult,
    component::{EncLogicLLComponent, EncLogicHLComponent},
};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet, VecDeque};
use std::default::Default;
use std::io::{ErrorKind, Read, Write};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use postcard::{from_bytes_cobs, to_stdvec_cobs};
use serde::{Deserialize, Serialize};


use bbqueue::{
    consts::*,
    framed::{FrameConsumer, FrameProducer, FrameGrantR, FrameGrantW},
    ArrayLength, BBBuffer, ConstBBBuffer,
};

static BB_OUT: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );
static BB_INP: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );

fn main() {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.set_nonblocking(true).unwrap();

    println!("Component connected!");
    let mut com = EncLogicHLComponent::new(
        TcpSpiComLL::new(stream),
        &BB_OUT,
        &BB_INP
    ).unwrap();

    let mut last_tx = Instant::now();

    while let Ok(_) = com.poll() {
        while let Some(msg) = com.dequeue() {
            println!("==> Got HL msg: {:?}", &msg[..]);
            msg.release();
        }

        if last_tx.elapsed() > Duration::from_secs(5) {
            println!("==> Enqueuing!");
            com.enqueue(&[0x0F; 9]).unwrap();
            last_tx = Instant::now();
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum TcpSpiMsg {
    ReadyState(bool),
    GoState(bool),
    Payload(Vec<u8>),
}

struct TcpSpiComLL {
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
    fn new(mut stream: TcpStream) -> Self {
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

    fn process(&mut self) -> SpiResult<()> {
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

    fn is_ready_active(&mut self) -> SpiResult<bool> {
        Ok(self.ready_state)
    }

    fn is_go_active(&mut self) -> SpiResult<bool> {
        self.go_state.ok_or(SpiError::ToDo)
    }

    /// Set the READY line low (active)
    fn notify_ready(&mut self) -> SpiResult<()> {
        self.ready_state = true;
        let msg = TcpSpiMsg::ReadyState(true);
        let payload = to_stdvec_cobs(&msg).map_err(|_| SpiError::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| SpiError::ToDo)?;
        Ok(())
    }

    /// Set the READY line high (inactive)
    fn clear_ready(&mut self) -> SpiResult<()> {
        self.ready_state = false;
        let msg = TcpSpiMsg::ReadyState(false);
        let payload = to_stdvec_cobs(&msg).map_err(|_| SpiError::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| SpiError::ToDo)?;
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
    ) -> SpiResult<()> {
        if self.pending_exchange.is_some() {
            return Err(SpiError::ToDo);
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
    fn trigger_exchange(&mut self) -> SpiResult<()> {
        if !(self.is_go_active()? && self.ready_state) {
            return Err(SpiError::ToDo);
        }

        let exch = match self.pending_exchange.as_ref() {
            Some(ex) => ex,
            None => return Err(SpiError::ToDo),
        };

        // Hello! I am pretending to be DMA!
        let payload =
            unsafe { core::slice::from_raw_parts(exch.data_out, exch.data_out_len) }.to_vec();

        let msg = to_stdvec_cobs(&TcpSpiMsg::Payload(payload)).map_err(|_| SpiError::ToDo)?;
        self.stream.write_all(&msg).map_err(|_| SpiError::ToDo)?;

        Ok(())
    }

    /// Is a `exchange` action still in progress?
    fn is_exchange_active(&self) -> SpiResult<bool> {
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
    fn complete_exchange(&mut self, clear_ready: bool) -> SpiResult<usize> {
        let is_go_active = self.is_go_active()?;

        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(SpiError::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None if !is_go_active => {
                // println!("No go!");
                return Err(SpiError::ArbitratorHungUp);
            }
            None => {
                self.pending_exchange = Some(exch);
                // Still busy
                return Err(SpiError::ToDo);
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
            Err(SpiError::ArbitratorHungUp)
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
    fn abort_exchange(&mut self) -> SpiResult<usize> {
        self.clear_ready().ok();

        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(SpiError::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None => {
                return Err(SpiError::IncompleteTransaction(0));
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
