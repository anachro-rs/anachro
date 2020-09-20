#![allow(unused_imports)]

use anachro_spi::{
    self as spi,
    Error as SpiError,
    Result as SpiResult,
    arbitrator::{
        EncLogicLLArbitrator,
        EncLogicHLArbitrator,
    },
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

// NOTE: For arbitrator, I will need 2xBBQueues per connection.
// this might be sort of unwieldy for something like 7-8 connections?
static BB_OUT: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );
static BB_INP: BBBuffer<U2048> = BBBuffer( ConstBBBuffer::new() );

fn main() {
    // FOR NOW, just accept a single connection.
    // Deal with parallel later
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        let mut last_tx = Instant::now();

        stream.set_nonblocking(true).unwrap();
        println!("{:?} connected", addr);
        let mut arb = EncLogicHLArbitrator::new(
            TcpSpiArbLL::new(stream),
            &BB_OUT,
            &BB_INP
        ).unwrap();

        while let Ok(_) = arb.poll() {
            while let Some(msg) = arb.dequeue() {
                println!("==> Got HL msg: {:?}", &msg[..]);
                msg.release();
            }

            if last_tx.elapsed() > Duration::from_secs(10) {
                println!("==> Enqueuing!");
                arb.enqueue(&[0xF0; 6]).unwrap();
                last_tx = Instant::now();
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum TcpSpiMsg {
    ReadyState(bool),
    GoState(bool),
    Payload(Vec<u8>),
}

struct TcpSpiArbLL {
    stream: TcpStream,
    go_state: bool,
    ready_state: Option<bool>,
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

impl TcpSpiArbLL {
    fn new(mut stream: TcpStream) -> Self {
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

    fn is_ready_active(&mut self) -> SpiResult<bool> {
        self.ready_state.ok_or(SpiError::ToDo)
    }

    fn notify_go(&mut self) -> SpiResult<()> {
        self.go_state = true;
        let msg = TcpSpiMsg::GoState(true);
        let payload = to_stdvec_cobs(&msg).map_err(|_| SpiError::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| SpiError::ToDo)?;
        Ok(())
    }

    fn clear_go(&mut self) -> SpiResult<()> {
        // println!("cleargo");
        self.go_state = false;
        let msg = TcpSpiMsg::GoState(false);
        let payload = to_stdvec_cobs(&msg).map_err(|_| SpiError::ToDo)?;
        self.stream
            .write_all(&payload)
            .map_err(|_| SpiError::ToDo)?;
        Ok(())
    }

    fn is_go_active(&mut self) -> SpiResult<bool> {
        Ok(self.go_state)
    }

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
        if !self.is_ready_active()? {
            return Err(SpiError::ToDo);
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

    fn is_exchange_active(&self) -> SpiResult<bool> {
        Ok(self.pending_exchange.is_some())
    }

    fn complete_exchange(&mut self, clear_go: bool) -> SpiResult<usize> {
        let exch = match self.pending_exchange.take() {
            Some(ex) => ex,
            None => return Err(SpiError::ToDo),
        };

        let inc = match self.incoming_payload.take() {
            None => {
                self.pending_exchange = Some(exch);
                return Err(SpiError::IncompleteTransaction(0));
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

        let msg = to_stdvec_cobs(&TcpSpiMsg::Payload(payload)).map_err(|_| SpiError::ToDo)?;
        self.stream.write_all(&msg).map_err(|_| SpiError::ToDo)?;

        // It's me, DMA!
        let out_slice = unsafe { core::slice::from_raw_parts_mut(exch.data_in, exch.data_in_max) };

        let copy_amt = exch.data_in_max.min(inc.len());

        out_slice[..copy_amt].copy_from_slice(&inc[..copy_amt]);

        if clear_go {
            self.clear_go()?;
        }

        Ok(copy_amt)
    }

    fn abort_exchange(&mut self) -> SpiResult<usize> {
        self.clear_go().ok();

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