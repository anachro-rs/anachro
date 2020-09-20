#![allow(unused_imports)]

use anachro_spi::{self as spi, EncLogicLLArbitrator, Error as SpiError, Result as SpiResult};
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

struct BBFullDuplex<CT>
where
    CT: ArrayLength<u8>,
{
    prod: FrameProducer<'static, CT>,
    cons: FrameConsumer<'static, CT>,
}

impl<CT> BBFullDuplex<CT>
where
    CT: ArrayLength<u8>,
{
    fn new(
        a: &'static BBBuffer<CT>,
    ) -> Result<BBFullDuplex<CT>, ()> {
        let (prod, cons) = a.try_split_framed().map_err(drop)?;

        Ok(BBFullDuplex {
            prod,
            cons,
        })
    }
}

fn main() {
    // FOR NOW, just accept a single connection.
    // Deal with parallel later
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        let mut last_tx = Instant::now();

        stream.set_nonblocking(true).unwrap();
        println!("{:?} connected", addr);
        let mut arb = TcpSpiArbHL::new(stream, &BB_OUT, &BB_INP).unwrap();

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

struct TcpSpiArbHL<CT>
where
    CT: ArrayLength<u8>,
{
    ll: TcpSpiArbLL,
    outgoing_msgs: BBFullDuplex<CT>,
    incoming_msgs: BBFullDuplex<CT>,
    out_grant: Option<FrameGrantR<'static, CT>>,
    in_grant: Option<FrameGrantW<'static, CT>>,
    out_buf: [u8; 4],
    sent_hdr: bool,
}

impl<CT> TcpSpiArbHL<CT>
where
    CT: ArrayLength<u8>
{
    pub fn new(
        stream: TcpStream,
        outgoing: &'static BBBuffer<CT>,
        incoming: &'static BBBuffer<CT>
    ) -> Result<Self, ()> {
        Ok(TcpSpiArbHL {
            ll: TcpSpiArbLL::new(stream),
            outgoing_msgs: BBFullDuplex::new(outgoing)?,
            incoming_msgs: BBFullDuplex::new(incoming)?,
            out_buf: [0u8; 4],
            out_grant: None,
            in_grant: None,
            sent_hdr: false,
        })
    }

    pub fn dequeue(&mut self) -> Option<FrameGrantR<CT>> {
        self.incoming_msgs.cons.read()
    }

    // TODO: `enqueue_with` function or something for zero-copy grants
    pub fn enqueue(&mut self, msg: &[u8]) -> Result<(), ()> {
        let len = msg.len();
        let mut wgr = self.outgoing_msgs.prod.grant(len).map_err(drop)?;
        wgr.copy_from_slice(msg);
        wgr.commit(len);
        Ok(())
    }

    pub fn poll(&mut self) -> Result<(), ()> {
        self.ll.process()?;

        if !self.ll.is_exchange_active().unwrap() {
            match self.ll.is_ready_active() {
                Ok(true) => {
                    if !self.sent_hdr {
                        // println!("Got READY, start header exchange!");
                        assert!(self.out_grant.is_none(), "Why do we have an out grant already?!");
                        assert!(self.in_grant.is_none(), "Why do we have an in grant already?!");

                        // This will be a pointer to the incoming grant
                        let in_ptr;

                        // Note: Hardcoded to 4, as we are expecting a u32
                        self.in_grant  = Some({
                            let mut igr = self.incoming_msgs.prod.grant(4).map_err(drop)?;
                            in_ptr = igr.as_mut_ptr();
                            igr
                        });
                        self.out_grant = self.outgoing_msgs.cons.read();

                        // Fill the output buffer with the size of the next body
                        self.out_buf = self
                            .out_grant
                            .as_ref()
                            .map(|msg| msg.len() as u32)
                            .unwrap_or(0)
                            .to_le_bytes();

                        self.ll
                            .prepare_exchange(
                                self.out_buf.as_ptr(),
                                self.out_buf.len(),
                                in_ptr,
                                4,
                            )
                            .unwrap();
                    } else {
                        // println!("Got READY, start data exchange!");

                        // TODO:
                        // * Parse the input data into a u32 to get the Component's len
                        // * Release the incoming grant (we don't want to send u32s to the app)
                        // * Request a grant that is component_len

                        // Note: This drops igr by taking it out of the option
                        let amt = if let Some(igr) = self.in_grant.take() {
                            assert_eq!(igr.len(), 4);
                            let mut buf = [0u8; 4];
                            buf.copy_from_slice(&igr);
                            let amt = u32::from_le_bytes(buf);
                            amt as usize
                        } else {
                            // Why don't we have a grant here?
                            // TODO: Probably want to drop grants, if any, and abort exch
                            panic!("logic error: No igr from header rx");
                        };

                        // HACK: Always request one byte so we'll get a grant for a valid pointer/
                        // to not handle potentially None grants. I might just want to have something
                        // else for this case
                        let in_ptr;
                        self.in_grant = match self.incoming_msgs.prod.grant(amt.max(1)) {
                            Ok(mut igr) => {
                                in_ptr = igr.as_mut_ptr();
                                Some(igr)
                            },
                            Err(_) => {
                                // TODO: probably want to abort and clear grants
                                todo!("Handle insufficient size for incoming message")
                            }
                        };

                        // Do we actually have an outgoing message?
                        let (ptr, len) = if let Some(msg) = self.out_grant.as_ref() {
                            (msg.as_ptr(), msg.len())
                        } else {
                            (self.out_buf.as_ptr(), 0)
                        };

                        self.ll
                            .prepare_exchange(
                                ptr,
                                len,
                                in_ptr,
                                amt,
                            )
                            .unwrap();
                    }
                }
                Ok(false) if self.ll.go_state => {
                    // println!("clearing go!");
                    self.ll.clear_go().unwrap();
                    self.sent_hdr = false;
                }
                _ => {}
            }
        } else {
            if !self.ll.is_ready_active().unwrap() {
                // println!("aborting!");
                self.ll.abort_exchange().ok();
                self.sent_hdr = false;

                // Drop grants without comitting.
                // Do AFTER aborting exchange, to defuse DMA.
                self.in_grant = None;
                self.out_grant = None;
            }

            match self.ll.complete_exchange(false) {
                Err(_) => {}
                Ok(amt) => {

                    // println!("got {:?}!", in_buf_inner);

                    if self.sent_hdr {

                        assert!(self.in_grant.is_some(), "Why don't we have an in grant at the end of exchange?");

                        if let Some(igr) = self.in_grant.take() {
                            igr.commit(amt);
                        }
                        if let Some(ogr) = self.out_grant.take() {
                            ogr.release();
                        }
                    }
                    // else NOTE: if we just finished sending the header, DON'T clean up yet, as we
                    // will do that at the start of the next transaction.

                    // If we hadn't sent the header, we just did.
                    // If we have sent the header, we need to for the
                    // next message
                    self.sent_hdr = !self.sent_hdr;
                }
            }
        }
        Ok(())
    }
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

    fn process(&mut self) -> Result<(), ()> {
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
}

impl EncLogicLLArbitrator for TcpSpiArbLL {
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
